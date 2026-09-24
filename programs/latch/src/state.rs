use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::EscrowError;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum DealState {
    Draft,
    Signed,
    Funded,
    Active,
    Deadlocked,
    Completed,
    Cancelled,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum DeadlockRule {
    /// After `timeout_secs` from the deadlock being raised, remaining funds release to the payee.
    TimeoutRelease,
    /// After `timeout_secs`, remaining funds return to the payer.
    TimeoutRefund,
    /// After `timeout_secs`, remaining funds split by `split_bps` (payee share).
    AutoSplit,
    /// A pubkey both parties chose at formation decides the payout.
    TieBreaker,
    /// Locked until mutual agreement; refunded to payer after a long `timeout_secs`.
    LongSunset,
    /// Locked until mutual sign-off or recovery. Requires every party's consent at signing.
    TrueDeadlock,
}

/// When the timeout clock for time-based deadlock rules runs.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum TimerMode {
    /// Clock starts when a party raises a deadlock (a single, unambiguous
    /// starting gun both parties triggered knowingly).
    FromDeadlock,
    /// Clock starts at activation and the deal auto-resolves on expiry while
    /// still Active (marketplace-style auto-release/refund). Raising a dispute
    /// pauses the clock; the raiser can withdraw to resume it. While paused,
    /// only mutual settlement or recovery move funds.
    FromActivation,
}

/// How a deadlock ended — carried in the DealResolved event.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum ResolutionPath {
    Mutual,
    TieBreaker,
    Timeout,
    Recovery,
}

/// Mint risk flags (bitfield). Persisted at creation, disclosed in the creation event,
/// and required to be covered by `accepted_risk_flags` before the deal can proceed.
pub mod risk_flags {
    pub const FREEZE_AUTHORITY: u16 = 1 << 0; // issuer can freeze the vault (USDC/USDT have this)
    pub const PERMANENT_DELEGATE: u16 = 1 << 1; // issuer can seize from the vault (PYUSD has this)
    pub const DORMANT_TRANSFER_HOOK: u16 = 1 << 2; // hook extension present but no program set
    pub const TRANSFER_FEE: u16 = 1 << 3; // amounts credited by balance delta
    pub const CONFIDENTIAL_CAPABLE: u16 = 1 << 4; // mint supports confidential transfers
    pub const INTEREST_BEARING: u16 = 1 << 5; // display-only divergence from raw amounts
    pub const MINT_CLOSE_AUTHORITY: u16 = 1 << 6; // informational
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct Milestone {
    pub amount: u64,
    /// Bitmap: bit i set = party i approved this milestone.
    pub approvals: u8,
    pub released: bool,
}

#[account]
#[derive(InitSpace)]
pub struct Deal {
    pub version: u8,
    pub bump: u8,
    pub deal_id: u64,
    pub creator: Pubkey,
    pub state: DealState,

    // Parties. Fixed arrays; only the first `num_parties` entries are meaningful.
    pub num_parties: u8,
    pub parties: [Pubkey; MAX_PARTIES],
    pub payer_idx: u8,
    pub payee_idx: u8,
    /// M-of-N approvals required to release a milestone.
    pub approval_threshold: u8,

    // Per-party bitmaps (bit i = party i).
    pub signed: u8,
    pub ready: u8,
    pub deadlock_consent: u8,
    pub cancel_approvals: u8,
    pub parties_signed_at: [i64; MAX_PARTIES],

    // Terms.
    pub terms_hash: [u8; 32],
    pub price_risk_term: u8,

    // Asset.
    pub mint: Pubkey,
    pub token_program: Pubkey,
    pub vault: Pubkey,
    pub total_amount: u64,
    /// Credited by vault balance delta, so transfer-fee mints can't corrupt accounting.
    pub deposited: u64,
    pub released_total: u64,
    pub risk_flags: u16,
    pub accepted_risk_flags: u16,

    // Milestones.
    pub num_milestones: u8,
    pub milestones: [Milestone; MAX_MILESTONES],

    // Deadlock rule — frozen once all parties sign.
    pub deadlock_rule: DeadlockRule,
    pub timer_mode: TimerMode,
    pub timeout_secs: i64,
    pub split_bps: u16,
    pub tie_breaker: Pubkey,
    pub deadlock_raised_at: i64,
    pub deadlock_raised_by: Pubkey,
    /// FromActivation accounting: seconds of Active-state clock accumulated
    /// before the current pause, and when the clock last (re)started.
    pub elapsed_at_pause: i64,
    pub last_resumed_at: i64,

    // Mutual/tie-breaker resolution proposal.
    pub proposal_active: bool,
    pub proposed_to_payee: u64,
    pub resolution_approvals: u8,
    pub tie_breaker_decided: bool,

    // Recovery (party-chosen, M-of-N; no company key exists anywhere in this program).
    pub num_recovery: u8,
    pub recovery_threshold: u8,
    pub recovery_signers: [Pubkey; MAX_RECOVERY_SIGNERS],
    pub recovery_proposal_active: bool,
    pub recovery_proposed_to_payee: u64,
    pub recovery_approvals: u8,
    /// Notice delay between "threshold of signatures reached" and executable.
    pub recovery_delay_secs: i64,
    /// When the current proposal first met the threshold (0 = not yet).
    pub recovery_ready_at: i64,

    // Timestamps (unix, from the Clock sysvar; never slot math).
    pub created_at: i64,
    pub signed_at: i64,
    pub funded_at: i64,
    pub activated_at: i64,

    /// Monotonic event sequence, one per emitted event.
    pub event_seq: u64,

    /// Reserved for future use (ZK phase: per-party ElGamal keys, etc.).
    pub _reserved: [u8; 64],
}

impl Deal {
    pub fn party_index(&self, key: &Pubkey) -> Option<u8> {
        self.parties[..self.num_parties as usize]
            .iter()
            .position(|p| p == key)
            .map(|i| i as u8)
    }

    pub fn require_party(&self, key: &Pubkey) -> Result<u8> {
        self.party_index(key).ok_or_else(|| error!(EscrowError::NotAParty))
    }

    /// Bitmap with one bit set per party.
    pub fn all_parties_mask(&self) -> u8 {
        (((1u16) << self.num_parties) - 1) as u8
    }

    pub fn recovery_index(&self, key: &Pubkey) -> Option<u8> {
        self.recovery_signers[..self.num_recovery as usize]
            .iter()
            .position(|p| p == key)
            .map(|i| i as u8)
    }

    pub fn next_seq(&mut self) -> u64 {
        let seq = self.event_seq;
        self.event_seq = self.event_seq.saturating_add(1);
        seq
    }

    pub fn require_state(&self, expected: DealState) -> Result<()> {
        require!(self.state == expected, EscrowError::InvalidState);
        Ok(())
    }
}

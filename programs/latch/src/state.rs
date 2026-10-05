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

/// What an unresolved dispute in a FromActivation deal turns into. Chosen at
/// formation. The default — the zero value, so every deal that never set one —
/// is `NeverExpire`: a dispute holds the funds until the parties settle, their
/// recovery signers act, or the payee refunds. The other policies apply once
/// the deal has spent its dispute window paused (cumulative across disputes).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum DisputePolicy {
    /// Disputes never expire.
    NeverExpire,
    /// Anyone may withdraw the expired dispute; the clock resumes and the
    /// deal's signed rule applies when it runs out.
    ResumeRule,
    /// The vault splits: the deal's `split_bps` under AutoSplit, else 50/50.
    Split,
    /// The vault returns to the payer.
    RefundPayer,
}

/// How a deadlock ended — carried in the DealResolved event.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum ResolutionPath {
    Mutual,
    TieBreaker,
    Timeout,
    Recovery,
    /// A FromActivation dispute outlived its window under the Split or
    /// RefundPayer policy.
    DisputeExpired,
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

    /// The tie-breaker's ruled payout (meaningful only while
    /// `tie_breaker_decided`). Stored apart from `proposed_to_payee` so a
    /// party counter-proposal can never overwrite or erase the ruling.
    /// Carved from the front of `_reserved` (zero in pre-existing accounts),
    /// so the account layout and size are unchanged.
    pub tie_breaker_amount: u64,

    // Dispute expiry (FromActivation only). Carved from the front of
    // `_reserved` like `tie_breaker_amount`: pre-existing accounts read zeros,
    // i.e. NeverExpire with nothing disputed yet.
    pub dispute_policy: DisputePolicy,
    /// Meaningful only for a policy other than NeverExpire (1–365 days).
    pub dispute_window_secs: u32,
    /// Seconds spent paused by disputes that have since been withdrawn.
    pub disputed_secs: u32,

    /// Reserved for future use (ZK phase: per-party ElGamal keys, etc.).
    pub _reserved: [u8; 47],
}

impl Deal {
    pub fn party_index(&self, key: &Pubkey) -> Option<u8> {
        self.parties[..self.num_parties as usize]
            .iter()
            .position(|p| p == key)
            .map(|i| i as u8)
    }

    pub fn require_party(&self, key: &Pubkey) -> Result<u8> {
        self.party_index(key)
            .ok_or_else(|| error!(EscrowError::NotAParty))
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

    /// Total seconds this deal has been paused by disputes, including the one
    /// open now. Repeated disputes share one window rather than each getting
    /// a fresh one.
    pub fn disputed_total(&self, now: i64) -> Result<i64> {
        let open = if self.state == DealState::Deadlocked {
            now.saturating_sub(self.deadlock_raised_at)
        } else {
            0
        };
        (self.disputed_secs as i64)
            .checked_add(open)
            .ok_or_else(|| error!(EscrowError::Overflow))
    }

    pub fn dispute_expired(&self, now: i64) -> Result<bool> {
        Ok(self.timer_mode == TimerMode::FromActivation
            && self.dispute_policy != DisputePolicy::NeverExpire
            && self.disputed_total(now)? >= self.dispute_window_secs as i64)
    }

    pub fn require_state(&self, expected: DealState) -> Result<()> {
        require!(self.state == expected, EscrowError::InvalidState);
        Ok(())
    }
}

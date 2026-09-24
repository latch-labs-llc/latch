use anchor_lang::prelude::*;

use crate::state::ResolutionPath;

// Every state change emits one event (via emit_cpi!, so indexers can't miss it in
// truncated logs). Each carries the deal key and a per-deal monotonic `seq` so the
// full contract history and a signature certificate can be rebuilt off-chain.

#[event]
pub struct DealCreated {
    pub deal: Pubkey,
    pub seq: u64,
    pub deal_id: u64,
    pub creator: Pubkey,
    pub mint: Pubkey,
    pub total_amount: u64,
    pub num_parties: u8,
    pub num_milestones: u8,
    pub terms_hash: [u8; 32],
    pub risk_flags: u16,
    pub timestamp: i64,
}

#[event]
pub struct TermsSigned {
    pub deal: Pubkey,
    pub seq: u64,
    pub party: Pubkey,
    pub party_index: u8,
    pub terms_hash: [u8; 32],
    pub all_signed: bool,
    pub timestamp: i64,
}

#[event]
pub struct DepositReceived {
    pub deal: Pubkey,
    pub seq: u64,
    pub from: Pubkey,
    pub amount_credited: u64,
    pub deposited_total: u64,
    pub fully_funded: bool,
    pub timestamp: i64,
}

#[event]
pub struct PartyReady {
    pub deal: Pubkey,
    pub seq: u64,
    pub party: Pubkey,
    pub all_ready: bool,
    pub timestamp: i64,
}

#[event]
pub struct MilestoneApproved {
    pub deal: Pubkey,
    pub seq: u64,
    pub milestone_index: u8,
    pub party: Pubkey,
    pub approvals: u8,
    pub timestamp: i64,
}

#[event]
pub struct MilestoneReleased {
    pub deal: Pubkey,
    pub seq: u64,
    pub milestone_index: u8,
    pub amount: u64,
    pub released_total: u64,
    pub completed: bool,
    pub timestamp: i64,
}

#[event]
pub struct DeadlockRaised {
    pub deal: Pubkey,
    pub seq: u64,
    pub raised_by: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct DeadlockWithdrawn {
    pub deal: Pubkey,
    pub seq: u64,
    pub withdrawn_by: Pubkey,
    /// Active-clock seconds already elapsed (FromActivation mode).
    pub elapsed_secs: i64,
    pub timestamp: i64,
}

#[event]
pub struct ResolutionSigned {
    pub deal: Pubkey,
    pub seq: u64,
    pub signer: Pubkey,
    pub proposed_to_payee: u64,
    pub is_tie_breaker: bool,
    pub timestamp: i64,
}

#[event]
pub struct DealResolved {
    pub deal: Pubkey,
    pub seq: u64,
    pub path: ResolutionPath,
    pub paid_to_payee: u64,
    pub refunded_to_payer: u64,
    pub timestamp: i64,
}

#[event]
pub struct RecoverySigned {
    pub deal: Pubkey,
    pub seq: u64,
    pub signer: Pubkey,
    pub proposed_to_payee: u64,
    pub signatures: u8,
    pub threshold: u8,
    /// When this proposal becomes executable (0 = threshold not yet met).
    /// On-chain notice: parties can see a pending recovery and object or settle.
    pub executable_at: i64,
    pub timestamp: i64,
}

#[event]
pub struct CancelSigned {
    pub deal: Pubkey,
    pub seq: u64,
    pub party: Pubkey,
    pub all_signed: bool,
    pub timestamp: i64,
}

#[event]
pub struct DealCancelled {
    pub deal: Pubkey,
    pub seq: u64,
    pub refunded_to_payer: u64,
    pub timestamp: i64,
}

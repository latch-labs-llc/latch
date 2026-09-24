pub mod constants;
pub mod error;
pub mod events;
pub mod instructions;
pub mod mint_guard;
pub mod payout;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("BT8tr7YXYLQPsLMq3qCipY1fBKyjT3gmGQB5amV2v5yv");

// Split-control escrow: two or more parties lock tokenized consideration in a
// program-owned vault. Funds move only on the parties' sign-offs, the resolution
// rule they chose at formation, or their party-chosen recovery signers. There is
// no admin instruction: no team key can move escrowed funds.
#[program]
pub mod latch {
    use super::*;

    pub fn create_deal(ctx: Context<CreateDeal>, params: CreateDealParams) -> Result<()> {
        instructions::create_deal::handle_create_deal(ctx, params)
    }

    pub fn sign_terms(ctx: Context<SignTerms>, consent_true_deadlock: bool) -> Result<()> {
        instructions::sign_terms::handle_sign_terms(ctx, consent_true_deadlock)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::handle_deposit(ctx, amount)
    }

    pub fn confirm_ready(ctx: Context<ConfirmReady>) -> Result<()> {
        instructions::confirm_ready::handle_confirm_ready(ctx)
    }

    pub fn approve_milestone(ctx: Context<ApproveMilestone>, index: u8) -> Result<()> {
        instructions::approve_milestone::handle_approve_milestone(ctx, index)
    }

    pub fn release_milestone(ctx: Context<ReleaseMilestone>, index: u8) -> Result<()> {
        instructions::release_milestone::handle_release_milestone(ctx, index)
    }

    pub fn raise_deadlock(ctx: Context<RaiseDeadlock>) -> Result<()> {
        instructions::raise_deadlock::handle_raise_deadlock(ctx)
    }

    pub fn withdraw_deadlock(ctx: Context<WithdrawDeadlock>) -> Result<()> {
        instructions::raise_deadlock::handle_withdraw_deadlock(ctx)
    }

    pub fn resolution_sign(ctx: Context<ResolutionSign>, amount_to_payee: u64) -> Result<()> {
        instructions::resolve::handle_resolution_sign(ctx, amount_to_payee)
    }

    pub fn resolve(ctx: Context<Resolve>) -> Result<()> {
        instructions::resolve::handle_resolve(ctx)
    }

    pub fn recovery_sign(ctx: Context<RecoverySign>, amount_to_payee: u64) -> Result<()> {
        instructions::recovery::handle_recovery_sign(ctx, amount_to_payee)
    }

    pub fn recovery_execute(ctx: Context<RecoveryExecute>) -> Result<()> {
        instructions::recovery::handle_recovery_execute(ctx)
    }

    pub fn cancel_draft(ctx: Context<CancelDraft>) -> Result<()> {
        instructions::cancel::handle_cancel_draft(ctx)
    }

    pub fn cancel_sign(ctx: Context<CancelSign>) -> Result<()> {
        instructions::cancel::handle_cancel_sign(ctx)
    }
}

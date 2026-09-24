use anchor_lang::prelude::*;

use crate::error::EscrowError;
use crate::events::{DeadlockRaised, DeadlockWithdrawn};
use crate::state::*;

#[event_cpi]
#[derive(Accounts)]
pub struct RaiseDeadlock<'info> {
    pub party: Signer<'info>,
    #[account(mut)]
    pub deal: Box<Account<'info, Deal>>,
}

pub fn handle_raise_deadlock(ctx: Context<RaiseDeadlock>) -> Result<()> {
    let deal = &mut ctx.accounts.deal;
    deal.require_state(DealState::Active)?;
    deal.require_party(&ctx.accounts.party.key())?;

    let now = Clock::get()?.unix_timestamp;
    deal.state = DealState::Deadlocked;
    deal.deadlock_raised_at = now;
    deal.deadlock_raised_by = ctx.accounts.party.key();
    if deal.timer_mode == TimerMode::FromActivation {
        // Pause the activation clock, banking the time elapsed so far.
        deal.elapsed_at_pause = deal
            .elapsed_at_pause
            .checked_add(now.saturating_sub(deal.last_resumed_at))
            .ok_or(EscrowError::Overflow)?;
    }

    let seq = deal.next_seq();
    emit_cpi!(DeadlockRaised {
        deal: deal.key(),
        seq,
        raised_by: ctx.accounts.party.key(),
        timestamp: now,
    });
    Ok(())
}

/// The party who raised the deadlock can withdraw it, returning the deal to
/// Active. In FromActivation mode the timeout clock resumes with its banked
/// elapsed time intact. Any pending resolution proposal is discarded.
#[event_cpi]
#[derive(Accounts)]
pub struct WithdrawDeadlock<'info> {
    pub party: Signer<'info>,
    #[account(mut)]
    pub deal: Box<Account<'info, Deal>>,
}

pub fn handle_withdraw_deadlock(ctx: Context<WithdrawDeadlock>) -> Result<()> {
    let deal = &mut ctx.accounts.deal;
    deal.require_state(DealState::Deadlocked)?;
    require!(
        ctx.accounts.party.key() == deal.deadlock_raised_by,
        EscrowError::OnlyRaiserMayWithdraw
    );

    let now = Clock::get()?.unix_timestamp;
    deal.state = DealState::Active;
    deal.deadlock_raised_at = 0;
    deal.deadlock_raised_by = Pubkey::default();
    deal.last_resumed_at = now;
    // A withdrawn dispute discards any in-flight settlement proposal.
    deal.proposal_active = false;
    deal.proposed_to_payee = 0;
    deal.resolution_approvals = 0;
    deal.tie_breaker_decided = false;

    let seq = deal.next_seq();
    emit_cpi!(DeadlockWithdrawn {
        deal: deal.key(),
        seq,
        withdrawn_by: ctx.accounts.party.key(),
        elapsed_secs: deal.elapsed_at_pause,
        timestamp: now,
    });
    Ok(())
}

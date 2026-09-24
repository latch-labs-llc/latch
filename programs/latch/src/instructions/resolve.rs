use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::BPS_DENOMINATOR;
use crate::error::EscrowError;
use crate::events::{DealResolved, ResolutionSigned};
use crate::payout::transfer_from_vault;
use crate::state::*;

#[event_cpi]
#[derive(Accounts)]
pub struct ResolutionSign<'info> {
    pub signer: Signer<'info>,
    #[account(mut)]
    pub deal: Box<Account<'info, Deal>>,
}

/// A party (or, under the TieBreaker rule, the tie-breaker) signs a proposed payout.
/// Proposing a different amount replaces the proposal and resets prior approvals.
pub fn handle_resolution_sign(ctx: Context<ResolutionSign>, amount_to_payee: u64) -> Result<()> {
    let deal = &mut ctx.accounts.deal;
    deal.require_state(DealState::Deadlocked)?;
    let signer = ctx.accounts.signer.key();
    let now = Clock::get()?.unix_timestamp;

    let is_tie_breaker =
        deal.deadlock_rule == DeadlockRule::TieBreaker && signer == deal.tie_breaker;
    if is_tie_breaker {
        deal.proposal_active = true;
        deal.proposed_to_payee = amount_to_payee;
        deal.resolution_approvals = 0;
        deal.tie_breaker_decided = true;
    } else {
        let idx = deal.require_party(&signer)?;
        let bit = 1u8 << idx;
        if !deal.proposal_active || deal.proposed_to_payee != amount_to_payee {
            deal.proposal_active = true;
            deal.proposed_to_payee = amount_to_payee;
            deal.resolution_approvals = bit;
            deal.tie_breaker_decided = false;
        } else {
            deal.resolution_approvals |= bit;
        }
    }

    let seq = deal.next_seq();
    emit_cpi!(ResolutionSigned {
        deal: deal.key(),
        seq,
        signer,
        proposed_to_payee: amount_to_payee,
        is_tie_breaker,
        timestamp: now,
    });
    Ok(())
}

/// Permissionless crank: applies the deal's resolution once its conditions hold.
#[event_cpi]
#[derive(Accounts)]
pub struct Resolve<'info> {
    #[account(mut, has_one = mint, has_one = vault, has_one = token_program @ EscrowError::TokenProgramMismatch)]
    pub deal: Box<Account<'info, Deal>>,
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(mut)]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = payer_token_account.mint == deal.mint @ EscrowError::TokenAccountMismatch,
        constraint = payer_token_account.owner == deal.parties[deal.payer_idx as usize] @ EscrowError::TokenAccountMismatch,
    )]
    pub payer_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = payee_token_account.mint == deal.mint @ EscrowError::TokenAccountMismatch,
        constraint = payee_token_account.owner == deal.parties[deal.payee_idx as usize] @ EscrowError::TokenAccountMismatch,
    )]
    pub payee_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
}

/// Payout for the deal's time-based rule, or an error for rules with no
/// timeout path. TrueDeadlock never resolves by time, in any timer mode.
fn timeout_payout(deal: &Deal, remaining: u64) -> Result<u64> {
    match deal.deadlock_rule {
        DeadlockRule::TimeoutRelease => Ok(remaining),
        DeadlockRule::TimeoutRefund | DeadlockRule::LongSunset => Ok(0),
        DeadlockRule::AutoSplit => u64::try_from(
            (remaining as u128) * (deal.split_bps as u128) / (BPS_DENOMINATOR as u128),
        )
        .map_err(|_| error!(EscrowError::Overflow)),
        DeadlockRule::TieBreaker | DeadlockRule::TrueDeadlock => {
            err!(EscrowError::ResolutionConditionsNotMet)
        }
    }
}

pub fn handle_resolve(ctx: Context<Resolve>) -> Result<()> {
    let deal = &ctx.accounts.deal;
    let now = Clock::get()?.unix_timestamp;
    let remaining = ctx.accounts.vault.amount;

    let (path, to_payee) = match deal.state {
        // FromActivation mode: the clock runs while the deal is Active and the
        // deal auto-resolves on expiry with no deadlock ever raised.
        DealState::Active => {
            require!(
                deal.timer_mode == TimerMode::FromActivation,
                EscrowError::InvalidState
            );
            let payee_amount = timeout_payout(deal, remaining)?;
            let elapsed = deal
                .elapsed_at_pause
                .checked_add(now.saturating_sub(deal.last_resumed_at))
                .ok_or(EscrowError::Overflow)?;
            require!(elapsed >= deal.timeout_secs, EscrowError::TimeoutNotElapsed);
            (ResolutionPath::Timeout, payee_amount)
        }
        DealState::Deadlocked => {
            let mutual =
                deal.proposal_active && deal.resolution_approvals == deal.all_parties_mask();
            let tie_broken =
                deal.deadlock_rule == DeadlockRule::TieBreaker && deal.tie_breaker_decided;
            if mutual {
                require!(deal.proposed_to_payee <= remaining, EscrowError::PayoutExceedsVault);
                (ResolutionPath::Mutual, deal.proposed_to_payee)
            } else if tie_broken {
                require!(deal.proposed_to_payee <= remaining, EscrowError::PayoutExceedsVault);
                (ResolutionPath::TieBreaker, deal.proposed_to_payee)
            } else {
                let payee_amount = timeout_payout(deal, remaining)?;
                // FromActivation: a raised dispute pauses the clock — no
                // timeout resolution until the raiser withdraws.
                require!(
                    deal.timer_mode == TimerMode::FromDeadlock,
                    EscrowError::TimeoutPausedByDispute
                );
                let deadline = deal
                    .deadlock_raised_at
                    .checked_add(deal.timeout_secs)
                    .ok_or(EscrowError::Overflow)?;
                require!(now >= deadline, EscrowError::TimeoutNotElapsed);
                (ResolutionPath::Timeout, payee_amount)
            }
        }
        _ => return err!(EscrowError::InvalidState),
    };

    let to_payer = remaining.checked_sub(to_payee).ok_or(EscrowError::Overflow)?;
    transfer_from_vault(
        &ctx.accounts.deal,
        &ctx.accounts.mint,
        &ctx.accounts.vault,
        &ctx.accounts.payee_token_account,
        &ctx.accounts.token_program,
        to_payee,
    )?;
    transfer_from_vault(
        &ctx.accounts.deal,
        &ctx.accounts.mint,
        &ctx.accounts.vault,
        &ctx.accounts.payer_token_account,
        &ctx.accounts.token_program,
        to_payer,
    )?;

    let deal = &mut ctx.accounts.deal;
    deal.state = DealState::Completed;
    let seq = deal.next_seq();
    emit_cpi!(DealResolved {
        deal: deal.key(),
        seq,
        path,
        paid_to_payee: to_payee,
        refunded_to_payer: to_payer,
        timestamp: now,
    });
    Ok(())
}

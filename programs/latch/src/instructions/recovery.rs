use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::error::EscrowError;
use crate::events::{DealResolved, RecoverySigned};
use crate::payout::transfer_from_vault;
use crate::state::*;

fn require_post_funding(deal: &Deal) -> Result<()> {
    require!(
        matches!(deal.state, DealState::Funded | DealState::Active | DealState::Deadlocked),
        EscrowError::InvalidState
    );
    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct RecoverySign<'info> {
    pub signer: Signer<'info>,
    #[account(mut)]
    pub deal: Box<Account<'info, Deal>>,
}

/// A party-chosen recovery signer signs a proposed payout (court order, key loss,
/// incapacity). Signing a different amount replaces the proposal and resets
/// prior signatures.
pub fn handle_recovery_sign(ctx: Context<RecoverySign>, amount_to_payee: u64) -> Result<()> {
    let deal = &mut ctx.accounts.deal;
    require_post_funding(deal)?;

    let signer = ctx.accounts.signer.key();
    let idx = deal.recovery_index(&signer).ok_or(EscrowError::NotARecoverySigner)?;
    let bit = 1u8 << idx;
    if !deal.recovery_proposal_active || deal.recovery_proposed_to_payee != amount_to_payee {
        deal.recovery_proposal_active = true;
        deal.recovery_proposed_to_payee = amount_to_payee;
        deal.recovery_approvals = bit;
        deal.recovery_ready_at = 0;
    } else {
        deal.recovery_approvals |= bit;
    }

    let now = Clock::get()?.unix_timestamp;
    let signatures = deal.recovery_approvals.count_ones() as u8;
    // The notice clock starts the first time this proposal meets the threshold.
    if signatures >= deal.recovery_threshold && deal.recovery_ready_at == 0 {
        deal.recovery_ready_at = now;
    }
    let executable_at = if deal.recovery_ready_at > 0 {
        deal.recovery_ready_at
            .checked_add(deal.recovery_delay_secs)
            .ok_or(EscrowError::Overflow)?
    } else {
        0
    };

    let seq = deal.next_seq();
    emit_cpi!(RecoverySigned {
        deal: deal.key(),
        seq,
        signer,
        proposed_to_payee: amount_to_payee,
        signatures,
        threshold: deal.recovery_threshold,
        executable_at,
        timestamp: now,
    });
    Ok(())
}

/// Permissionless crank: executes the recovery payout once M-of-N recovery
/// signatures are in. Funds can only go to the parties themselves — even a
/// compromised recovery set cannot pay a stranger.
#[event_cpi]
#[derive(Accounts)]
pub struct RecoveryExecute<'info> {
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

pub fn handle_recovery_execute(ctx: Context<RecoveryExecute>) -> Result<()> {
    let deal = &ctx.accounts.deal;
    require_post_funding(deal)?;
    require!(deal.recovery_proposal_active, EscrowError::NoActiveProposal);
    require!(
        deal.recovery_approvals.count_ones() >= deal.recovery_threshold as u32,
        EscrowError::InsufficientRecoverySignatures
    );
    // Notice period: the proposal is public on-chain from recovery_ready_at,
    // giving parties a window to object or settle before it can execute.
    let now = Clock::get()?.unix_timestamp;
    let executable_at = deal
        .recovery_ready_at
        .checked_add(deal.recovery_delay_secs)
        .ok_or(EscrowError::Overflow)?;
    require!(
        deal.recovery_ready_at > 0 && now >= executable_at,
        EscrowError::RecoveryDelayNotElapsed
    );

    let remaining = ctx.accounts.vault.amount;
    let to_payee = deal.recovery_proposed_to_payee;
    require!(to_payee <= remaining, EscrowError::PayoutExceedsVault);
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
    let now = Clock::get()?.unix_timestamp;
    let seq = deal.next_seq();
    emit_cpi!(DealResolved {
        deal: deal.key(),
        seq,
        path: ResolutionPath::Recovery,
        paid_to_payee: to_payee,
        refunded_to_payer: to_payer,
        timestamp: now,
    });
    Ok(())
}

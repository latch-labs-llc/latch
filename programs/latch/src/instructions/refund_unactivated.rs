use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::ACTIVATION_WINDOW_SECS;
use crate::error::EscrowError;
use crate::events::ActivationLapsed;
use crate::payout::transfer_from_vault;
use crate::state::*;

/// A fully funded deal that has not become Active within the activation window
/// returns its whole vault to the payer. Permissionless like every other crank:
/// anyone may submit it, but the only possible destination is the payer's own
/// token account for the deal's mint.
#[event_cpi]
#[derive(Accounts)]
pub struct RefundUnactivated<'info> {
    pub cranker: Signer<'info>,
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
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_refund_unactivated(ctx: Context<RefundUnactivated>) -> Result<()> {
    let deal = &ctx.accounts.deal;
    deal.require_state(DealState::Funded)?;

    let now = Clock::get()?.unix_timestamp;
    let lapses_at = deal
        .funded_at
        .checked_add(ACTIVATION_WINDOW_SECS)
        .ok_or(EscrowError::Overflow)?;
    require!(now >= lapses_at, EscrowError::ActivationWindowOpen);

    let refund = ctx.accounts.vault.amount;
    transfer_from_vault(
        &ctx.accounts.deal,
        &ctx.accounts.mint,
        &ctx.accounts.vault,
        &ctx.accounts.payer_token_account,
        &ctx.accounts.token_program,
        refund,
    )?;

    let deal = &mut ctx.accounts.deal;
    deal.state = DealState::Cancelled;
    let seq = deal.next_seq();
    emit_cpi!(ActivationLapsed {
        deal: deal.key(),
        seq,
        refunded_to_payer: refund,
        funded_at: deal.funded_at,
        timestamp: now,
    });
    Ok(())
}

use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::error::EscrowError;
use crate::events::PayeeRefunded;
use crate::payout::transfer_from_vault;
use crate::state::*;

/// The payee alone returns everything still in the vault to the payer — the
/// "I can't deliver, here's your money back" exit. It only ever moves funds
/// toward the payer, so it needs no one else's consent. Valid whenever the
/// vault can hold the payer's deposit: a partially funded Signed deal, Funded,
/// Active, or Deadlocked.
#[event_cpi]
#[derive(Accounts)]
pub struct RefundByPayee<'info> {
    pub payee: Signer<'info>,
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

pub fn handle_refund_by_payee(ctx: Context<RefundByPayee>) -> Result<()> {
    let deal = &ctx.accounts.deal;
    require!(
        ctx.accounts.payee.key() == deal.parties[deal.payee_idx as usize],
        EscrowError::OnlyPayeeMayRefund
    );
    match deal.state {
        DealState::Funded | DealState::Active | DealState::Deadlocked => {}
        DealState::Signed if deal.deposited > 0 => {}
        _ => return err!(EscrowError::InvalidState),
    }

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
    // Nothing released yet: the deal never happened. Otherwise earlier
    // milestones stand and the deal concludes with the remainder returned.
    deal.state = if deal.released_total == 0 {
        DealState::Cancelled
    } else {
        DealState::Completed
    };
    let now = Clock::get()?.unix_timestamp;
    let seq = deal.next_seq();
    emit_cpi!(PayeeRefunded {
        deal: deal.key(),
        seq,
        payee: ctx.accounts.payee.key(),
        refunded_to_payer: refund,
        timestamp: now,
    });
    Ok(())
}

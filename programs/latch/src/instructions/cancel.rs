use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::error::EscrowError;
use crate::events::{CancelSigned, DealCancelled};
use crate::payout::transfer_from_vault;
use crate::state::*;

/// In Draft (before all parties have signed) any single party can walk away.
#[event_cpi]
#[derive(Accounts)]
pub struct CancelDraft<'info> {
    pub party: Signer<'info>,
    #[account(mut)]
    pub deal: Box<Account<'info, Deal>>,
}

pub fn handle_cancel_draft(ctx: Context<CancelDraft>) -> Result<()> {
    let deal = &mut ctx.accounts.deal;
    deal.require_state(DealState::Draft)?;
    deal.require_party(&ctx.accounts.party.key())?;

    deal.state = DealState::Cancelled;
    let now = Clock::get()?.unix_timestamp;
    let seq = deal.next_seq();
    emit_cpi!(DealCancelled { deal: deal.key(), seq, refunded_to_payer: 0, timestamp: now });
    Ok(())
}

/// After signing, cancellation is mutual: every party signs, then any deposit
/// refunds to the payer. Valid in Signed and Funded (before performance begins).
#[event_cpi]
#[derive(Accounts)]
pub struct CancelSign<'info> {
    pub party: Signer<'info>,
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

pub fn handle_cancel_sign(ctx: Context<CancelSign>) -> Result<()> {
    let deal = &mut ctx.accounts.deal;
    require!(
        matches!(deal.state, DealState::Signed | DealState::Funded),
        EscrowError::InvalidState
    );
    let idx = deal.require_party(&ctx.accounts.party.key())?;
    deal.cancel_approvals |= 1u8 << idx;
    let all_signed = deal.cancel_approvals == deal.all_parties_mask();

    let now = Clock::get()?.unix_timestamp;
    let seq = deal.next_seq();
    emit_cpi!(CancelSigned {
        deal: deal.key(),
        seq,
        party: ctx.accounts.party.key(),
        all_signed,
        timestamp: now,
    });

    if all_signed {
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
        emit_cpi!(DealCancelled { deal: deal.key(), seq, refunded_to_payer: refund, timestamp: now });
    }
    Ok(())
}

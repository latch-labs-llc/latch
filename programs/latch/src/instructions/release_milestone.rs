use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::error::EscrowError;
use crate::events::MilestoneReleased;
use crate::payout::transfer_from_vault;
use crate::state::*;

/// Permissionless crank: once approvals meet the threshold, anyone can execute
/// the release. The platform is never required to be in the flow of funds.
#[event_cpi]
#[derive(Accounts)]
pub struct ReleaseMilestone<'info> {
    #[account(mut, has_one = mint, has_one = vault, has_one = token_program @ EscrowError::TokenProgramMismatch)]
    pub deal: Box<Account<'info, Deal>>,
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(mut)]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = payee_token_account.mint == deal.mint @ EscrowError::TokenAccountMismatch,
        constraint = payee_token_account.owner == deal.parties[deal.payee_idx as usize] @ EscrowError::TokenAccountMismatch,
    )]
    pub payee_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_release_milestone(ctx: Context<ReleaseMilestone>, index: u8) -> Result<()> {
    let deal = &ctx.accounts.deal;
    deal.require_state(DealState::Active)?;
    require!(index < deal.num_milestones, EscrowError::MilestoneOutOfRange);

    let milestone = &deal.milestones[index as usize];
    require!(!milestone.released, EscrowError::MilestoneAlreadyReleased);
    require!(
        milestone.approvals.count_ones() >= deal.approval_threshold as u32,
        EscrowError::InsufficientApprovals
    );
    // Milestones release in order: partial performance pays out progressively.
    for prior in &deal.milestones[..index as usize] {
        require!(prior.released, EscrowError::MilestonesOutOfOrder);
    }
    let amount = milestone.amount;

    transfer_from_vault(
        &ctx.accounts.deal,
        &ctx.accounts.mint,
        &ctx.accounts.vault,
        &ctx.accounts.payee_token_account,
        &ctx.accounts.token_program,
        amount,
    )?;

    let deal = &mut ctx.accounts.deal;
    deal.milestones[index as usize].released = true;
    deal.released_total = deal.released_total.checked_add(amount).ok_or(EscrowError::Overflow)?;

    let completed = deal.milestones[..deal.num_milestones as usize].iter().all(|m| m.released);
    let now = Clock::get()?.unix_timestamp;
    if completed {
        deal.state = DealState::Completed;
    }

    let seq = deal.next_seq();
    emit_cpi!(MilestoneReleased {
        deal: deal.key(),
        seq,
        milestone_index: index,
        amount,
        released_total: deal.released_total,
        completed,
        timestamp: now,
    });
    Ok(())
}

use anchor_lang::prelude::*;

use crate::error::EscrowError;
use crate::events::MilestoneApproved;
use crate::state::*;

#[event_cpi]
#[derive(Accounts)]
pub struct ApproveMilestone<'info> {
    pub party: Signer<'info>,
    #[account(mut)]
    pub deal: Box<Account<'info, Deal>>,
}

pub fn handle_approve_milestone(ctx: Context<ApproveMilestone>, index: u8) -> Result<()> {
    let deal = &mut ctx.accounts.deal;
    deal.require_state(DealState::Active)?;
    require!(index < deal.num_milestones, EscrowError::MilestoneOutOfRange);

    let idx = deal.require_party(&ctx.accounts.party.key())?;
    let milestone = &mut deal.milestones[index as usize];
    require!(!milestone.released, EscrowError::MilestoneAlreadyReleased);
    milestone.approvals |= 1u8 << idx;
    let approvals = milestone.approvals;

    let now = Clock::get()?.unix_timestamp;
    let seq = deal.next_seq();
    emit_cpi!(MilestoneApproved {
        deal: deal.key(),
        seq,
        milestone_index: index,
        party: ctx.accounts.party.key(),
        approvals,
        timestamp: now,
    });
    Ok(())
}

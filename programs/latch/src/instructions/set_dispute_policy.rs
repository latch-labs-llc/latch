use anchor_lang::prelude::*;

use crate::constants::{MAX_DISPUTE_WINDOW_SECS, MIN_DISPUTE_WINDOW_SECS};
use crate::error::EscrowError;
use crate::events::DisputePolicySet;
use crate::state::*;

/// The creator picks what an unresolved dispute turns into (FromActivation
/// deals only). Allowed only in Draft before anyone has signed, so every party
/// signs over the chosen policy like any other term. Deals that never call
/// this keep the default, NeverExpire.
#[event_cpi]
#[derive(Accounts)]
pub struct SetDisputePolicy<'info> {
    pub creator: Signer<'info>,
    #[account(mut)]
    pub deal: Box<Account<'info, Deal>>,
}

pub fn handle_set_dispute_policy(
    ctx: Context<SetDisputePolicy>,
    policy: DisputePolicy,
    window_secs: u32,
) -> Result<()> {
    let deal = &mut ctx.accounts.deal;
    deal.require_state(DealState::Draft)?;
    require!(
        ctx.accounts.creator.key() == deal.creator,
        EscrowError::OnlyCreatorMaySetPolicy
    );
    require!(deal.signed == 0, EscrowError::DisputePolicyLocked);
    require!(
        deal.timer_mode == TimerMode::FromActivation,
        EscrowError::InvalidTimerMode
    );
    require!(
        (MIN_DISPUTE_WINDOW_SECS..=MAX_DISPUTE_WINDOW_SECS).contains(&window_secs),
        EscrowError::InvalidDisputeWindow
    );

    deal.dispute_policy = policy;
    deal.dispute_window_secs = window_secs;

    let now = Clock::get()?.unix_timestamp;
    let seq = deal.next_seq();
    emit_cpi!(DisputePolicySet {
        deal: deal.key(),
        seq,
        policy,
        window_secs,
        timestamp: now,
    });
    Ok(())
}

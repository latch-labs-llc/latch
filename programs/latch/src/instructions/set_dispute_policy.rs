use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use solana_instructions_sysvar::{load_current_index_checked, load_instruction_at_checked};

use crate::constants::{MAX_DISPUTE_WINDOW_SECS, MIN_DISPUTE_WINDOW_SECS};
use crate::error::EscrowError;
use crate::events::DisputePolicySet;
use crate::state::*;

/// The creator picks what an unresolved dispute turns into (FromActivation
/// deals only). It must run in the SAME transaction as this deal's
/// `create_deal`, before anything else touches the deal: no one can see a deal
/// before its creating transaction lands, so the policy a counterparty reviews
/// is always the policy they sign — it can never be switched between viewing
/// and signing. Deals that never call this keep the default, NeverExpire.
#[event_cpi]
#[derive(Accounts)]
pub struct SetDisputePolicy<'info> {
    pub creator: Signer<'info>,
    #[account(mut)]
    pub deal: Box<Account<'info, Deal>>,
    /// CHECK: pinned to the instructions sysvar and read only through its
    /// checked loaders.
    #[account(address = solana_instructions_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
}

pub fn handle_set_dispute_policy(
    ctx: Context<SetDisputePolicy>,
    policy: DisputePolicy,
    window_secs: u32,
) -> Result<()> {
    let deal = &ctx.accounts.deal;
    deal.require_state(DealState::Draft)?;
    require!(
        ctx.accounts.creator.key() == deal.creator,
        EscrowError::OnlyCreatorMaySetPolicy
    );
    require!(deal.signed == 0, EscrowError::DisputePolicyLocked);
    // Nothing but DealCreated has happened to this deal...
    require!(deal.event_seq == 1, EscrowError::PolicyMustBeSetAtCreation);
    // ...and it was created earlier in this very transaction.
    let ixs = ctx.accounts.instructions.to_account_info();
    let current = load_current_index_checked(&ixs)? as usize;
    let deal_key = deal.key();
    let created_here = (0..current).any(|i| {
        load_instruction_at_checked(i, &ixs).is_ok_and(|ix| {
            ix.program_id == crate::ID
                && ix.data.get(..8) == Some(crate::instruction::CreateDeal::DISCRIMINATOR)
                && ix.accounts.get(1).map(|m| m.pubkey) == Some(deal_key)
        })
    });
    require!(created_here, EscrowError::PolicyMustBeSetAtCreation);
    require!(
        deal.timer_mode == TimerMode::FromActivation,
        EscrowError::InvalidTimerMode
    );
    require!(
        (MIN_DISPUTE_WINDOW_SECS..=MAX_DISPUTE_WINDOW_SECS).contains(&window_secs),
        EscrowError::InvalidDisputeWindow
    );

    let deal = &mut ctx.accounts.deal;
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

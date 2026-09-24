use anchor_lang::prelude::*;

use crate::error::EscrowError;
use crate::events::PartyReady;
use crate::state::*;

#[event_cpi]
#[derive(Accounts)]
pub struct ConfirmReady<'info> {
    pub party: Signer<'info>,
    #[account(mut)]
    pub deal: Box<Account<'info, Deal>>,
}

pub fn handle_confirm_ready(ctx: Context<ConfirmReady>) -> Result<()> {
    let deal = &mut ctx.accounts.deal;
    deal.require_state(DealState::Funded)?;

    let idx = deal.require_party(&ctx.accounts.party.key())?;
    let bit = 1u8 << idx;
    require!(deal.ready & bit == 0, EscrowError::AlreadySigned);
    deal.ready |= bit;

    let now = Clock::get()?.unix_timestamp;
    let all_ready = deal.ready == deal.all_parties_mask();
    if all_ready {
        deal.state = DealState::Active;
        deal.activated_at = now;
        // FromActivation timer mode: the clock starts running now.
        deal.last_resumed_at = now;
    }

    let seq = deal.next_seq();
    emit_cpi!(PartyReady {
        deal: deal.key(),
        seq,
        party: ctx.accounts.party.key(),
        all_ready,
        timestamp: now,
    });
    Ok(())
}

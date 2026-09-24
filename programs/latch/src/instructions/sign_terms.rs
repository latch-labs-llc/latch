use anchor_lang::prelude::*;

use crate::error::EscrowError;
use crate::events::TermsSigned;
use crate::state::*;

#[event_cpi]
#[derive(Accounts)]
pub struct SignTerms<'info> {
    pub party: Signer<'info>,
    #[account(mut)]
    pub deal: Box<Account<'info, Deal>>,
}

pub fn handle_sign_terms(ctx: Context<SignTerms>, consent_true_deadlock: bool) -> Result<()> {
    let deal = &mut ctx.accounts.deal;
    deal.require_state(DealState::Draft)?;

    let idx = deal.require_party(&ctx.accounts.party.key())?;
    let bit = 1u8 << idx;
    require!(deal.signed & bit == 0, EscrowError::AlreadySigned);

    if deal.deadlock_rule == DeadlockRule::TrueDeadlock {
        require!(consent_true_deadlock, EscrowError::TrueDeadlockConsentRequired);
        deal.deadlock_consent |= bit;
    }

    let now = Clock::get()?.unix_timestamp;
    deal.signed |= bit;
    deal.parties_signed_at[idx as usize] = now;

    let all_signed = deal.signed == deal.all_parties_mask();
    if all_signed {
        // From here every fund-flow parameter is frozen: no instruction in this
        // program can modify parties, thresholds, milestones, the deadlock rule,
        // or the recovery set.
        deal.state = DealState::Signed;
        deal.signed_at = now;
    }

    let seq = deal.next_seq();
    emit_cpi!(TermsSigned {
        deal: deal.key(),
        seq,
        party: ctx.accounts.party.key(),
        party_index: idx,
        terms_hash: deal.terms_hash,
        all_signed,
        timestamp: now,
    });
    Ok(())
}

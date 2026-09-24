use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::constants::DEAL_SEED;
use crate::state::Deal;

/// Transfer `amount` out of the vault, signed by the deal PDA. No-op for zero.
pub fn transfer_from_vault<'info>(
    deal: &Account<'info, Deal>,
    mint: &InterfaceAccount<'info, Mint>,
    vault: &InterfaceAccount<'info, TokenAccount>,
    to: &InterfaceAccount<'info, TokenAccount>,
    token_program: &Interface<'info, TokenInterface>,
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let creator = deal.creator;
    let deal_id = deal.deal_id.to_le_bytes();
    let bump = [deal.bump];
    let seeds: &[&[u8]] = &[DEAL_SEED, creator.as_ref(), &deal_id, &bump];
    token_interface::transfer_checked(
        CpiContext::new_with_signer(
            token_program.key(),
            TransferChecked {
                from: vault.to_account_info(),
                mint: mint.to_account_info(),
                to: to.to_account_info(),
                authority: deal.to_account_info(),
            },
            &[seeds],
        ),
        amount,
        mint.decimals,
    )
}

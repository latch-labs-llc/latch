use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::error::EscrowError;
use crate::events::DepositReceived;
use crate::state::*;

#[event_cpi]
#[derive(Accounts)]
pub struct Deposit<'info> {
    pub payer: Signer<'info>,
    #[account(mut, has_one = mint, has_one = vault, has_one = token_program @ EscrowError::TokenProgramMismatch)]
    pub deal: Box<Account<'info, Deal>>,
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(mut)]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut, constraint = payer_token_account.mint == deal.mint @ EscrowError::TokenAccountMismatch)]
    pub payer_token_account: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let deal = &ctx.accounts.deal;
    deal.require_state(DealState::Signed)?;
    require!(amount > 0, EscrowError::ZeroDeposit);

    let payer_idx = deal.require_party(&ctx.accounts.payer.key())?;
    require!(payer_idx == deal.payer_idx, EscrowError::OnlyPayerMayDeposit);

    let vault_before = ctx.accounts.vault.amount;
    token_interface::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            TransferChecked {
                from: ctx.accounts.payer_token_account.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        ),
        amount,
        ctx.accounts.mint.decimals,
    )?;
    ctx.accounts.vault.reload()?;
    // Credit what the vault actually received, not what was sent — transfer-fee
    // mints deliver less than the instruction amount.
    let credited = ctx
        .accounts
        .vault
        .amount
        .checked_sub(vault_before)
        .ok_or(EscrowError::Overflow)?;

    let deal = &mut ctx.accounts.deal;
    deal.deposited = deal.deposited.checked_add(credited).ok_or(EscrowError::Overflow)?;
    require!(deal.deposited <= deal.total_amount, EscrowError::OverFunded);

    let now = Clock::get()?.unix_timestamp;
    let fully_funded = deal.deposited == deal.total_amount;
    if fully_funded {
        deal.state = DealState::Funded;
        deal.funded_at = now;
    }

    let seq = deal.next_seq();
    emit_cpi!(DepositReceived {
        deal: deal.key(),
        seq,
        from: ctx.accounts.payer.key(),
        amount_credited: credited,
        deposited_total: deal.deposited,
        fully_funded,
        timestamp: now,
    });
    Ok(())
}

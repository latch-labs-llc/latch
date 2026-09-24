use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::error::EscrowError;
use crate::events::DealCreated;
use crate::mint_guard::vet_mint;
use crate::state::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateDealParams {
    pub deal_id: u64,
    pub parties: Vec<Pubkey>,
    pub payer_idx: u8,
    pub payee_idx: u8,
    pub approval_threshold: u8,
    pub terms_hash: [u8; 32],
    pub price_risk_term: u8,
    pub total_amount: u64,
    pub milestone_amounts: Vec<u64>,
    pub deadlock_rule: DeadlockRule,
    pub timer_mode: TimerMode,
    pub timeout_secs: i64,
    pub split_bps: u16,
    pub tie_breaker: Pubkey,
    pub recovery_signers: Vec<Pubkey>,
    pub recovery_threshold: u8,
    pub recovery_delay_secs: i64,
    pub accepted_risk_flags: u16,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(params: CreateDealParams)]
pub struct CreateDeal<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        init,
        payer = creator,
        space = 8 + Deal::INIT_SPACE,
        seeds = [DEAL_SEED, creator.key().as_ref(), &params.deal_id.to_le_bytes()],
        bump
    )]
    pub deal: Box<Account<'info, Deal>>,
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        init,
        payer = creator,
        associated_token::mint = mint,
        associated_token::authority = deal,
        associated_token::token_program = token_program,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_create_deal(ctx: Context<CreateDeal>, params: CreateDealParams) -> Result<()> {
    let n = params.parties.len();
    require!(n >= 2 && n <= MAX_PARTIES, EscrowError::InvalidParties);
    for i in 0..n {
        for j in (i + 1)..n {
            require!(params.parties[i] != params.parties[j], EscrowError::DuplicateParty);
        }
    }
    require!(
        params.parties.contains(&ctx.accounts.creator.key()),
        EscrowError::CreatorNotParty
    );
    require!(
        (params.payer_idx as usize) < n
            && (params.payee_idx as usize) < n
            && params.payer_idx != params.payee_idx,
        EscrowError::InvalidParties
    );
    require!(
        params.approval_threshold >= 1 && (params.approval_threshold as usize) <= n,
        EscrowError::InvalidThreshold
    );

    let m = params.milestone_amounts.len();
    require!(m >= 1 && m <= MAX_MILESTONES, EscrowError::InvalidMilestones);
    require!(params.total_amount > 0, EscrowError::InvalidMilestones);
    let mut sum: u64 = 0;
    for &amount in &params.milestone_amounts {
        require!(amount > 0, EscrowError::InvalidMilestones);
        sum = sum.checked_add(amount).ok_or(EscrowError::Overflow)?;
    }
    require!(sum == params.total_amount, EscrowError::MilestoneSumMismatch);

    match params.deadlock_rule {
        DeadlockRule::TimeoutRelease
        | DeadlockRule::TimeoutRefund
        | DeadlockRule::AutoSplit
        | DeadlockRule::LongSunset => {
            require!(params.timeout_secs > 0, EscrowError::InvalidDeadlockParams);
        }
        DeadlockRule::TieBreaker => {
            require!(
                params.tie_breaker != Pubkey::default()
                    && !params.parties.contains(&params.tie_breaker),
                EscrowError::InvalidDeadlockParams
            );
        }
        DeadlockRule::TrueDeadlock => {}
    }
    // Rules with no timeout path have no clock; FromActivation would be
    // meaningless for TieBreaker and must never exist for TrueDeadlock (no
    // configuration may create a timeout backdoor into a true deadlock).
    if matches!(params.deadlock_rule, DeadlockRule::TieBreaker | DeadlockRule::TrueDeadlock) {
        require!(params.timer_mode == TimerMode::FromDeadlock, EscrowError::InvalidTimerMode);
    }
    require!(params.recovery_delay_secs >= 0, EscrowError::InvalidRecoveryConfig);
    if params.deadlock_rule == DeadlockRule::AutoSplit {
        require!(params.split_bps as u64 <= BPS_DENOMINATOR, EscrowError::InvalidDeadlockParams);
    }

    let r = params.recovery_signers.len();
    require!(r >= 1 && r <= MAX_RECOVERY_SIGNERS, EscrowError::InvalidRecoveryConfig);
    require!(
        params.recovery_threshold >= 1 && (params.recovery_threshold as usize) <= r,
        EscrowError::InvalidRecoveryConfig
    );
    for i in 0..r {
        for j in (i + 1)..r {
            require!(
                params.recovery_signers[i] != params.recovery_signers[j],
                EscrowError::InvalidRecoveryConfig
            );
        }
    }

    let flags = vet_mint(&ctx.accounts.mint.to_account_info())?;
    require!(flags & !params.accepted_risk_flags == 0, EscrowError::RiskFlagsNotAccepted);

    let now = Clock::get()?.unix_timestamp;
    let deal = &mut ctx.accounts.deal;
    deal.version = STATE_VERSION;
    deal.bump = ctx.bumps.deal;
    deal.deal_id = params.deal_id;
    deal.creator = ctx.accounts.creator.key();
    deal.state = DealState::Draft;
    deal.num_parties = n as u8;
    deal.parties[..n].copy_from_slice(&params.parties);
    deal.payer_idx = params.payer_idx;
    deal.payee_idx = params.payee_idx;
    deal.approval_threshold = params.approval_threshold;
    deal.terms_hash = params.terms_hash;
    deal.price_risk_term = params.price_risk_term;
    deal.mint = ctx.accounts.mint.key();
    deal.token_program = ctx.accounts.token_program.key();
    deal.vault = ctx.accounts.vault.key();
    deal.total_amount = params.total_amount;
    deal.risk_flags = flags;
    deal.accepted_risk_flags = params.accepted_risk_flags;
    deal.num_milestones = m as u8;
    for (i, &amount) in params.milestone_amounts.iter().enumerate() {
        deal.milestones[i] = Milestone { amount, approvals: 0, released: false };
    }
    deal.deadlock_rule = params.deadlock_rule;
    deal.timer_mode = params.timer_mode;
    deal.timeout_secs = params.timeout_secs;
    deal.split_bps = params.split_bps;
    deal.tie_breaker = params.tie_breaker;
    deal.num_recovery = r as u8;
    deal.recovery_threshold = params.recovery_threshold;
    deal.recovery_delay_secs = params.recovery_delay_secs;
    deal.recovery_signers[..r].copy_from_slice(&params.recovery_signers);
    deal.created_at = now;

    let seq = deal.next_seq();
    emit_cpi!(DealCreated {
        deal: deal.key(),
        seq,
        deal_id: params.deal_id,
        creator: deal.creator,
        mint: deal.mint,
        total_amount: deal.total_amount,
        num_parties: deal.num_parties,
        num_milestones: deal.num_milestones,
        terms_hash: deal.terms_hash,
        risk_flags: flags,
        timestamp: now,
    });
    Ok(())
}

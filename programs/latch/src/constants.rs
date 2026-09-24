use anchor_lang::prelude::*;

#[constant]
pub const DEAL_SEED: &[u8] = b"deal";

pub const MAX_PARTIES: usize = 8;
pub const MAX_MILESTONES: usize = 16;
pub const MAX_RECOVERY_SIGNERS: usize = 3;

pub const BPS_DENOMINATOR: u64 = 10_000;

pub const STATE_VERSION: u8 = 1;

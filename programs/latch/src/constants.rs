use anchor_lang::prelude::*;

#[constant]
pub const DEAL_SEED: &[u8] = b"deal";

pub const MAX_PARTIES: usize = 8;
pub const MAX_MILESTONES: usize = 16;
pub const MAX_RECOVERY_SIGNERS: usize = 3;

pub const BPS_DENOMINATOR: u64 = 10_000;

/// Upper bound on `timeout_secs` (10 years). A timeout so distant it can
/// never fire would make a time-based rule a de facto true deadlock without
/// the explicit consent TrueDeadlock requires; deals meant to run longer
/// should use LongSunset or TrueDeadlock.
pub const MAX_TIMEOUT_SECS: i64 = 315_360_000;

pub const STATE_VERSION: u8 = 1;

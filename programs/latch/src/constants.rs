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

/// How long a fully funded deal may wait for every party to confirm ready
/// (3 days). If it has not become Active by then, anyone may return the whole
/// vault to the payer: without this, a counterparty who never confirms could
/// hold the payer's deposit indefinitely, since every other exit from Funded
/// needs that counterparty's signature.
#[constant]
pub const ACTIVATION_WINDOW_SECS: i64 = 259_200;

pub const STATE_VERSION: u8 = 1;

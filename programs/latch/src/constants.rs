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

/// How long a deal holding the payer's deposit may wait to become Active
/// (3 days from full funding, or from signing if only partly funded). After
/// that anyone may return the whole vault to the payer: otherwise a
/// counterparty who never confirms could hold the deposit indefinitely.
#[constant]
pub const ACTIVATION_WINDOW_SECS: i64 = 259_200;

/// Bounds on the dispute window a deal may choose when it opts into a dispute
/// policy other than NeverExpire (1–365 days of cumulative dispute time).
#[constant]
pub const MIN_DISPUTE_WINDOW_SECS: u32 = 86_400;
#[constant]
pub const MAX_DISPUTE_WINDOW_SECS: u32 = 31_536_000;

pub const STATE_VERSION: u8 = 1;

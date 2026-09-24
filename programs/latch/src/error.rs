use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Invalid party configuration")]
    InvalidParties,
    #[msg("Duplicate party pubkey")]
    DuplicateParty,
    #[msg("Creator must be one of the parties")]
    CreatorNotParty,
    #[msg("Invalid milestone configuration")]
    InvalidMilestones,
    #[msg("Milestone amounts must sum to the total amount")]
    MilestoneSumMismatch,
    #[msg("Invalid approval threshold")]
    InvalidThreshold,
    #[msg("Invalid deadlock rule parameters")]
    InvalidDeadlockParams,
    #[msg("Invalid recovery configuration")]
    InvalidRecoveryConfig,
    #[msg("Signer is not a party to this deal")]
    NotAParty,
    #[msg("Signer is not a recovery signer for this deal")]
    NotARecoverySigner,
    #[msg("Instruction not valid in the deal's current state")]
    InvalidState,
    #[msg("Party has already signed")]
    AlreadySigned,
    #[msg("True-deadlock rule requires explicit consent from every party")]
    TrueDeadlockConsentRequired,
    #[msg("Only the payer may deposit")]
    OnlyPayerMayDeposit,
    #[msg("Deposit amount must be greater than zero")]
    ZeroDeposit,
    #[msg("Deposit exceeds the deal's total amount")]
    OverFunded,
    #[msg("Milestone index out of range")]
    MilestoneOutOfRange,
    #[msg("Milestone already released")]
    MilestoneAlreadyReleased,
    #[msg("Previous milestones must be released first")]
    MilestonesOutOfOrder,
    #[msg("Not enough approvals to release this milestone")]
    InsufficientApprovals,
    #[msg("Resolution conditions are not met")]
    ResolutionConditionsNotMet,
    #[msg("Timeout has not elapsed")]
    TimeoutNotElapsed,
    #[msg("This deadlock rule never resolves by timeout")]
    NoTimeoutPath,
    #[msg("Proposed payout exceeds the vault balance")]
    PayoutExceedsVault,
    #[msg("Not enough recovery signatures")]
    InsufficientRecoverySignatures,
    #[msg("No active proposal")]
    NoActiveProposal,
    #[msg("Token account does not match the deal")]
    TokenAccountMismatch,
    #[msg("Mint does not match the deal")]
    MintMismatch,
    #[msg("Token program does not match the deal")]
    TokenProgramMismatch,
    #[msg("Mint is non-transferable; escrow is impossible")]
    NonTransferableMint,
    #[msg("Mint's default account state is frozen; escrow vault would be unusable")]
    DefaultFrozenMint,
    #[msg("Mint has an active transfer hook program; not supported")]
    ActiveTransferHook,
    #[msg("Mint carries an unknown Token-2022 extension; rejected by default")]
    UnknownMintExtension,
    #[msg("Parties have not accepted this mint's risk flags")]
    RiskFlagsNotAccepted,
    #[msg("Timeout clock is paused while a dispute is open")]
    TimeoutPausedByDispute,
    #[msg("Only the party who raised the deadlock may withdraw it")]
    OnlyRaiserMayWithdraw,
    #[msg("This timer mode is not valid for the chosen deadlock rule")]
    InvalidTimerMode,
    #[msg("Recovery notice delay has not elapsed")]
    RecoveryDelayNotElapsed,
    #[msg("Arithmetic overflow")]
    Overflow,
}

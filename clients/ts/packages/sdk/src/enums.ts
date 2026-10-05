/**
 * Anchor represents Rust enums as single-key objects (e.g. `{ draft: {} }`).
 * These helpers give them ergonomic names on both directions.
 */

export const DeadlockRule = {
  TimeoutRelease: { timeoutRelease: {} },
  TimeoutRefund: { timeoutRefund: {} },
  AutoSplit: { autoSplit: {} },
  TieBreaker: { tieBreaker: {} },
  LongSunset: { longSunset: {} },
  TrueDeadlock: { trueDeadlock: {} },
} as const;
export type DeadlockRuleName = keyof typeof DeadlockRule;

export const TimerMode = {
  /** Clock starts when a party raises a deadlock. */
  FromDeadlock: { fromDeadlock: {} },
  /** Clock runs from activation; auto-resolves while Active; disputes pause it. */
  FromActivation: { fromActivation: {} },
} as const;
export type TimerModeName = keyof typeof TimerMode;

/**
 * What an unresolved FromActivation dispute turns into (set before anyone
 * signs via setDisputePolicy). Default: NeverExpire — the funds stay locked
 * until the parties settle, recovery acts, or the payee refunds.
 */
export const DisputePolicy = {
  NeverExpire: { neverExpire: {} },
  /** After the window, anyone may lift the dispute; the signed rule resumes. */
  ResumeRule: { resumeRule: {} },
  /** After the window, the vault splits (the deal's split under AutoSplit, else 50/50). */
  Split: { split: {} },
  /** After the window, the vault returns to the payer. */
  RefundPayer: { refundPayer: {} },
} as const;
export type DisputePolicyName = keyof typeof DisputePolicy;

export type DealStateName =
  | "Draft"
  | "Signed"
  | "Funded"
  | "Active"
  | "Deadlocked"
  | "Completed"
  | "Cancelled";

/** Name of an anchor enum value, e.g. `{ draft: {} }` → "Draft". */
export function enumName(value: object): string {
  const key = Object.keys(value)[0] ?? "unknown";
  return key.charAt(0).toUpperCase() + key.slice(1);
}

export function dealStateName(state: object): DealStateName {
  return enumName(state) as DealStateName;
}

/** Mint risk flags — must be covered by `acceptedRiskFlags` at creation. */
export const RiskFlags = {
  FREEZE_AUTHORITY: 1 << 0, // issuer can freeze the vault (USDC/USDT)
  PERMANENT_DELEGATE: 1 << 1, // issuer can seize from the vault (PYUSD)
  DORMANT_TRANSFER_HOOK: 1 << 2,
  TRANSFER_FEE: 1 << 3,
  CONFIDENTIAL_CAPABLE: 1 << 4,
  INTEREST_BEARING: 1 << 5,
  MINT_CLOSE_AUTHORITY: 1 << 6,
  /** Accept every flag (use knowingly — this is the parties' risk consent). */
  ALL: 0xffff,
} as const;

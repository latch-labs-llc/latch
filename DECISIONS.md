# Decision log

Design decisions and defaults, with reasoning. Kept current so reviewers and
integrators can see why the program behaves the way it does.

## Core design decisions

- **Recovery role is party-chosen M-of-N, never held by the maintainers.**
  The protocol is non-custodial by construction: no key held by the
  maintainers can move escrowed funds, and there is deliberately no
  instruction gated on the upgrade authority or any maintainer key.
- **Fund-flow parameters freeze at full signature.** Once every party has
  signed, no instruction can modify parties, thresholds, milestone amounts,
  the resolution rule, or the recovery set for that deal.
- **No fees, no token, no staking in this program.** Any commercial layer
  belongs outside the open-source protocol.
- **`release_milestone`, `resolve`, and `recovery_execute` are permissionless
  cranks** — once on-chain conditions are met, anyone can execute the outcome.
  The maintainers are never required in the flow of funds, and a timeout
  outcome cannot be blocked by any party going dark.
- **`recovery_execute` can only pay the parties** (any payer/payee split). A
  compromised recovery set can mis-split funds between the parties, never pay
  a third address.
- **Recovery notice delay:** each deal sets a delay between the recovery
  threshold being met and the payout becoming executable. The pending proposal
  and its `executable_at` are visible on-chain, giving parties a window to
  object or settle.
- **Timer modes:** time-based resolution rules run either from the moment a
  dispute is raised (`FromDeadlock`) or from activation with auto-resolution
  while Active and dispute-pause semantics (`FromActivation`). `FromActivation`
  is rejected at creation for TieBreaker and TrueDeadlock deals so no
  configuration can create a timeout backdoor into a true deadlock.
- **TrueDeadlock never resolves by time, in any mode.** Funds stay locked until
  mutual sign-off or the parties' own recovery path. Every party must set an
  explicit consent flag at signing to enter such a deal.
- **Mutual resolution is always available from Deadlocked under every rule**,
  via matching `resolution_sign(amount_to_payee)` from all parties. A changed
  proposal resets prior approvals (and any tie-breaker decision).
- **Tie-breaker decides any split**, not just release-or-refund; the
  tie-breaker must be a non-party.
- **Milestones release strictly in order** (partial performance pays
  progressively).
- **Legal terms stay off-chain.** Only a 32-byte hash of the agreement, plus
  signature and consent events, go on-chain. Never terms text or personal data.
- **Identity is out of the program.** Parties are pubkeys.

## Token safety posture

- **Default-deny:** unknown Token-2022 extensions are rejected, as are
  non-transferable, default-frozen, and active-transfer-hook mints (escrow
  cannot function safely on them).
- **Flag-and-consent:** freeze authority (USDC/USDT have it), permanent
  delegate (PYUSD has it), dormant transfer hooks, transfer fees,
  confidential-transfer capability, and interest-bearing configs are recorded
  as risk flags at creation and must be explicitly covered by
  `accepted_risk_flags` before a deal can proceed. The flags are emitted in
  the creation event as a durable disclosure record.
- **Deposits credit by vault balance delta**, not instruction amount, so
  transfer-fee mints cannot corrupt milestone accounting.
- **One token program per deal**; `transfer_checked` everywhere.

## Structural defaults

- Program maxima: **8 parties, 16 milestones, 3 recovery signers** (bitmapped
  approvals in a single ~1KB deal account; per-party PDAs deferred to a future
  confidential-balance phase as a new account type).
- All deadlines are absolute unix timestamps from the Clock sysvar — no slot
  math (slot duration is changing under Alpenglow).
- `emit_cpi!` (CPI events) for every state change with a per-deal monotonic
  sequence number; indexers read inner instructions or Geyser, never logs.
- Deal accounts and vaults are not closed in v0.1 — the account is the durable
  record alongside events; revisit rent reclamation before mainnet.
- ZK-readiness reservations: version byte + 64 reserved bytes in `Deal`, raw
  u64 amounts everywhere, `confidential_capable` recorded per mint. Real ZK
  proof verification is exercised in the test suite (`tests/test_zk_proofs.rs`);
  see `ZK_NOTES.md`.

## Build/tooling decisions

- **Anchor 1.2.0, Solana SDK 3.x crates, LiteSVM Rust tests.**
- **SBPF arch v0** (`anchor build --arch v0`, via the Makefile): Anchor 1.2
  defaults to SBPF v3, which current cluster deployment and LiteSVM both
  reject ("invalid file header"). Revisit when the v3 loader activates.
- **Boxed accounts in every context** — the ~1KB `Deal` plus several
  `InterfaceAccount`s overflowed the 4KB SBF stack frame in generated
  `try_accounts` before boxing. Treat cargo-build-sbf frame-size warnings as
  fatal: the binary they produce fails verification at load.
- ZK ElGamal builtin in tests: after LiteSVM `add_builtin`, the program account
  must be re-owned to the native loader or invocation fails with
  "Unsupported program id".
- Upgrade authority: single keypair on devnet; moves to a Squads v4 multisig
  via Safe Authority Transfer before mainnet (with verifiable builds).

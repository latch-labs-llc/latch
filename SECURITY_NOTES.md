# Security notes — known risks for a future audit

Devnet prototype, not audited. This lists what we already know needs scrutiny.

## Program-level

1. **Permissionless cranks (`release_milestone`, `resolve`, `recovery_execute`).**
   Intended design. Audit focus: no path where a cranker can influence *where*
   funds go — destinations are constrained to the recorded payer/payee owners'
   token accounts. Note: constraints check the token account's `owner` field, so a
   party can be paid into ANY token account they own, chosen by the cranker. Risk
   accepted for v0.1 (funds still reach the right party); consider pinning exact
   payout accounts at formation.
2. **Timeout rules vs. externally frozen vaults.** If an issuer freezes the vault
   (USDC/USDT can), `resolve`/`release` fail with a token error while the clock
   keeps running. Rights aren't consumed by a failed attempt (re-cranking works
   after a thaw), but a party could time a freeze request adversarially. Documented;
   revisit with issuer-freeze playbooks.
3. **Deadlock stalling incentives.** Timeout rules favor whoever the clock helps;
   they can simply go quiet. Product-layer disclosure, not fixable in-program.
4. **`resolution_sign` proposal races.** A changed proposal resets approvals — a
   malicious party can grief by re-proposing forever, but can never move funds
   without the counterparty (or the timeout). Griefing accepted.
5. **Recovery signers hold real power** (any payer/payee split of remaining funds,
   from any post-funding state, ignoring milestones/deadlock rule). This is the
   court-order hook working as designed; parties must choose signers carefully.
   M-of-N and pay-parties-only bound the damage.
6. **`recovery_sign` proposal is first-come:** any single recovery signer can reset
   the pending proposal. With threshold ≥ 2 this only griefs, never pays out.
7. **Arithmetic:** checked ops everywhere; split math uses u128 intermediate;
   overflow-checks = true in release profile. Audit the bps rounding (floor to
   payee, remainder to payer).
8. **Bitmap logic:** party/recovery indices < 8/3 enforced at creation;
   `all_parties_mask` uses u16 intermediate to avoid `1u8 << 8` overflow at 8 parties.
9. **PDA/seed hygiene:** deal PDA seeds `[b"deal", creator, deal_id]`; vault is the
   deal's ATA; `has_one` checks bind mint/vault/token_program on every fund-moving
   instruction. Audit for missing `has_one` on signer-only instructions (they
   mutate only bitmaps but verify party membership against the deal account itself).
10. **Token-2022 surface:** `mint_guard` is allow/flag/deny by extension list;
    unknown extensions reject. The list must be re-reviewed against the pinned
    spl-token-2022 version at every dependency bump (new extensions default to
    reject — safe, but silently blocks new mint types).
11. **Deposit over-funding is rejected post-CPI** (whole transaction reverts) —
    verify no state persists from the failed path.
12. **Events:** `emit_cpi!` everywhere; `event_seq` monotonic. Indexers must read
    inner instructions, not logs.

## Operational

13. **Upgrade authority** is a single local keypair on devnet. Before anything real:
    Squads v4 multisig via Safe Authority Transfer, then a timelock policy. The
    program has no admin instructions, but an upgrade can change anything —
    per-contract immutability ultimately depends on upgrade governance
    (verifiable builds + multisig + timelock + eventually immutability).
14. **No fee logic in-program** — keep it that way; anything commercial
    belongs outside the open-source protocol.
15. **Terms hash** is opaque 32 bytes; the program cannot verify what was hashed.
    Any signing ceremony built on top must bind identity ↔ wallet ↔ document;
    that binding is an integrator concern, outside this program.

## Out of scope for v0.1 (revisit before mainnet)

- Vault close / rent reclamation (withheld transfer fees can block `close_account`).
- Confidential transfers (layout reserved; proof program re-enabled June 2026).
- Transfer-hook mints with a live hook program (rejected today).
- Multi-mint deals; partial deposits from multiple payers.
- Compute-unit budgeting/benchmarks (add Mollusk CU regression tests).

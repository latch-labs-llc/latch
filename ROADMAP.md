# Roadmap

Latch is early and the protocol is deliberately small. This is the build
order as it stands; issues tagged `good first issue` are concrete entry
points into most of these.

## Shipped (devnet)

- Anchor program: 14 instructions, six resolution rules, two timer modes,
  party-chosen recovery, Token-2022 vetting, CPI events — 45 LiteSVM tests,
  verified reproducible build.
- `@latch-labs/sdk` + `@latch-labs/cli` + `@latch-labs/checkout` on npm;
  local-validator and devnet E2E suites in CI.
- Reference app at https://latchlabs.org/ — deal tooling, two demo
  storefronts on the checkout package, self-verifying agreements, and a
  printable evidence packet per deal.

## Next

- **Independent security audit**, then mainnet. Upgrade authority moves to a
  Squads multisig with a timelock before any real funds; vault close / rent
  reclamation and a terminal-state residue sweep land in the same pass.
- **P2P payment requests** — a "request $X for [thing]" quick-create that
  presets sane defaults and shares as a link in any chat. The deal links
  already work this way; this is a consumer-shaped surface over them.
- **Checkout package hardening** — React bindings, wallet-path polish,
  configurable mints/timers, richer order events.
- **Reference integrations + indexer** — webhook-style event delivery and
  integration examples beyond the two demo stores.

## Exploring

- **Documentary settlement** — releases triggered by documents or
  attestations (inspection certificates, delivery records) with the
  tie-breaker role acting as examiner; the shape trade workflows need.
- **Confidential amounts** — Token-2022 confidential transfers for deal
  amounts (see ZK_NOTES.md; account layout already reserves space).
- **Hosted session API** — the `@latch-labs/checkout` interface served over
  HTTP with webhooks, for merchants who want zero on-chain code. The
  protocol stays free and permissionless either way: nothing in this
  repository ever requires a hosted service.

Nothing here changes the invariants: no fees in the protocol, no operator
key that can move funds, every deal settles even if the maintainers vanish.

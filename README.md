# Latch

[![CI](https://github.com/latch-labs-llc/latch/actions/workflows/ci.yml/badge.svg)](https://github.com/latch-labs-llc/latch/actions/workflows/ci.yml) [![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE) [![npm sdk](https://img.shields.io/npm/v/%40latch-labs%2Fsdk?label=%40latch-labs%2Fsdk)](https://www.npmjs.com/package/@latch-labs/sdk) [![npm cli](https://img.shields.io/npm/v/%40latch-labs%2Fcli?label=%40latch-labs%2Fcli)](https://www.npmjs.com/package/@latch-labs/cli)

**Split-control escrow for Solana.** Two or more parties lock tokenized
consideration (stablecoins first) in a program-owned vault that no single
party — and no company — can move. Funds release only on the parties'
sign-offs, a resolution rule the parties chose at formation, or a
party-designated recovery role for key loss and court orders. The hash of the
parties' off-chain agreement, and every signature and consent, is recorded
on-chain so a complete, independently verifiable execution record can be
reconstructed.

> ⚠️ **Unaudited, experimental, devnet only. Do not use with real funds.**
> An independent security review is planned; see [SECURITY.md](SECURITY.md).

Maintained by Latch Labs LLC (in formation). Apache-2.0.

## Devnet deployment

| | |
| --- | --- |
| Program ID | `BT8tr7YXYLQPsLMq3qCipY1fBKyjT3gmGQB5amV2v5yv` |
| Network | Solana devnet |
| Explorer | https://explorer.solana.com/address/BT8tr7YXYLQPsLMq3qCipY1fBKyjT3gmGQB5amV2v5yv?cluster=devnet |
| Verified hash | `fa6d6982dd563133a6363deb40bedd0885f800c1484eca80887b60dea3934f82` |

**Try it live (devnet):** https://latch-labs-llc.github.io/latch/ — connect Phantom, Solflare, or a zero-setup burner wallet; built-in test-token faucet.

**Demo videos:** [Full sale lifecycle](https://youtu.be/69Psn335b_I) · [Dispute raised and settled on-chain](https://youtu.be/lbIxBCt56W0)

### Verify the deployed bytecode yourself

The on-chain program is a deterministic build of this repository (CI's
`verified-build` job produces it on every push). To check, with
[`solana-verify`](https://github.com/Ellipsis-Labs/solana-verifiable-build)
installed and Docker available (x86-64 host):

```sh
solana-verify get-program-hash -u https://api.devnet.solana.com \
  BT8tr7YXYLQPsLMq3qCipY1fBKyjT3gmGQB5amV2v5yv
# → compare against the hash produced from source:
solana-verify build --library-name latch -b solanafoundation/solana-verifiable-build:4.2.2
solana-verify get-executable-hash target/deploy/latch.so
```

Both must print the verified hash above. (No Docker? The same hash is in the
`latch-verified-build` artifact of every green CI run.)

## Why

Marketplace and P2P payments protect at most one side: buyers can be scammed
on push payments, sellers eat chargebacks. Split-control escrow protects both
at once — the buyer can't be taken for their money, the seller gets final
settlement — with courts as the backstop, not the counterparty's goodwill.
Latch is the open, non-custodial primitive for that: a structured approval
workflow over a vault, designed so the operator of any product built on it
never holds or controls user funds.

## How it works

```
Draft ──all parties sign terms hash──▶ Signed ──deposit──▶ Funded ──all ready──▶ Active
  │                                      │                                        │ milestones approve/release
  └─cancel (any party)                   └─cancel (mutual, refund)                ▼
                                                        Completed ◀── final release
                                                            ▲
                              Deadlocked ──rule / mutual sign-off / recovery──────┘
```

- **Parties & thresholds:** up to 8 parties, M-of-N milestone approvals,
  up to 16 strictly-ordered milestones with partial releases.
- **Resolution rules** (fixed at formation): timeout release, timeout refund,
  automatic split, named tie-breaker, long sunset, or true deadlock (locked
  until mutual sign-off; requires every party's explicit consent flag).
  Time-based rules run in one of two per-deal timer modes: clock-from-dispute,
  or clock-from-activation with auto-resolution and dispute-pause.
- **Recovery:** a party-chosen M-of-N signer set for key loss, incapacity, or
  court orders — payouts can only go to the parties, and only after a per-deal
  on-chain notice delay.
- **Permissionless cranks:** once conditions are met, anyone can execute a
  release or resolution. No operator sits in the flow of funds.
- **Token safety:** SPL Token and Token-2022. Mints are vetted at creation —
  escrow-impossible extensions are rejected (non-transferable, default-frozen,
  active transfer hooks, unknown extensions); custody-affecting ones (freeze
  authority, permanent delegate, transfer fees) are flagged and must be
  explicitly accepted by the parties, on the record.
- **Events:** every state change emits a CPI event with a per-deal sequence
  number — enough to rebuild a full history and signature certificate.

## Quickstart

Prereqs: Rust (stable), Solana CLI (Agave 4.x), Anchor 1.2.x.

```sh
make build     # anchor build --arch v0  (required flag — see note)
make test      # 45 LiteSVM tests: lifecycle, every failure path, all six
               # rules & both timer modes, recovery, hostile Token-2022
               # mints, and real ZK proof verification
```

> **Note:** always build via `make` — Anchor 1.2 defaults to SBPF v3, which
> current devnet deployment and LiteSVM both reject.

Run the full lifecycle against devnet (needs ~0.5 devnet SOL in
`~/.config/solana/id.json`):

```sh
cargo run --example devnet_e2e     # scripted: happy path + deadlock scenario
make demo                          # interactive walkthrough → http://localhost:7878
```

The interactive demo walks a marketplace sale end to end — agreement drafting,
a signing ceremony over the document's SHA-256 digest, escrow funding,
delivery confirmation and release (or a dispute settled on-chain) — and ends
with a print-ready signature certificate reconstructed entirely from on-chain
data.

## TypeScript SDK & CLI

```sh
npm install @latch-labs/sdk     # typed client: all 14 instructions, events, PDAs
npm install -g @latch-labs/cli  # full-lifecycle `latch` command
```

See `clients/ts/packages/*/README.md` for quickstarts, and the
[SDK API reference](https://latch-labs-llc.github.io/latch/api/) (typedoc,
rebuilt on every push).

## Repository

- `programs/latch/` — the Anchor program
- `programs/latch/tests/` — LiteSVM test suite
- `programs/latch/examples/devnet_e2e.rs` — scripted devnet lifecycle
- `app/` — wallet-connected reference web app (live at the link above)
- `demo/` — scripted reference demo server (marketplace sale + signature certificate)
- `DECISIONS.md` — design decision log with reasoning
- `SECURITY_NOTES.md` — assumptions, trust boundaries, known risks (for reviewers)
- `ZK_NOTES.md` — confidential-transfer status and phase-2 design

## License

Apache License 2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE).

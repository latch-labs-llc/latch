# @latch-labs/sdk

TypeScript SDK for [Latch](https://github.com/latch-labs-llc/latch) — the
split-control escrow primitive on Solana.

> ⚠️ **Unaudited, experimental, devnet only. Do not use with real funds.**

```sh
npm install @latch-labs/sdk
```

## Quickstart

```ts
import { AnchorProvider, Wallet } from "@anchor-lang/core";
import { Connection, Keypair } from "@solana/web3.js";
import {
  LatchClient, DeadlockRule, TimerMode, RiskFlags, BN,
} from "@latch-labs/sdk";

const provider = new AnchorProvider(
  new Connection("https://api.devnet.solana.com", "confirmed"),
  new Wallet(payerKeypair),
  { commitment: "confirmed" }
);
const client = LatchClient.fromProvider(provider);

// Create a two-party deal: 1,000 tokens, two milestones, both must approve.
const dealId = Math.floor(Date.now() / 1000);
await client.createDeal({
  dealId,
  parties: [buyer.publicKey, seller.publicKey],
  payerIdx: 0,
  payeeIdx: 1,
  approvalThreshold: 2,
  termsHash: sha256OfYourAgreementDocument,       // 32 bytes
  milestoneAmounts: [new BN(600_000_000), new BN(400_000_000)],
  deadlockRule: DeadlockRule.TimeoutRefund,
  timerMode: TimerMode.FromDeadlock,
  timeoutSecs: 7 * 24 * 3600,
  recoverySigners: [recoveryAgent.publicKey],     // party-chosen, never an operator
  recoveryThreshold: 1,
  recoveryDelaySecs: 3 * 24 * 3600,               // on-chain notice window
  acceptedRiskFlags: RiskFlags.FREEZE_AUTHORITY,  // explicit mint-risk consent
  mint: usdcMint,
}).rpc();

const deal = client.dealPda(provider.wallet.publicKey, dealId);
await client.signTerms(deal).rpc();               // each party, own wallet
// ... deposit, confirmReady, approveMilestone, releaseMilestone — see the
// API reference: https://latch-labs-llc.github.io/latch/api/

const info = await client.fetchDeal(deal);        // typed account + state name
```

Every lifecycle method returns Anchor's `MethodsBuilder` — chain `.rpc()` to
send, `.instruction()` to compose into your own transaction, or
`.transaction()` to sign elsewhere (wallet adapters).

## Event history

Latch emits a CPI event for every state change (inner-instruction data — not
logs, which RPC providers truncate). Rebuild a deal's complete, independently
verifiable history:

```ts
import { fetchEventHistory } from "@latch-labs/sdk";
const events = await fetchEventHistory(client.program, connection, deal);
// [{ name: "dealCreated", data: {...}, signature, blockTime }, ...]
```

This is the primitive behind signature certificates and adoption metrics.

## What the program guarantees

- No operator key can move escrowed funds — there is no admin instruction.
- Funds move only on party sign-offs, the resolution rule fixed at formation,
  or the parties' own recovery signers (only to the parties, only after the
  on-chain notice delay).
- Releases and resolutions are permissionless cranks once conditions are met.
- Mints are vetted at creation; custody-affecting Token-2022 extensions must
  be explicitly accepted via `acceptedRiskFlags`.

Apache-2.0. Program ID (devnet): `BT8tr7YXYLQPsLMq3qCipY1fBKyjT3gmGQB5amV2v5yv`.

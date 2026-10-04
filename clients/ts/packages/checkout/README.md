# @latch-labs/checkout

Escrow checkout sessions for [Latch](https://github.com/latch-labs-llc/latch) —
the client interface of escrow-as-a-service on Solana.

A merchant creates a checkout session; the buyer pays from a Solana wallet or
as a guest (an invisible wallet created for them); the package drives the
escrow deal end to end — created, signed by both sides, funded, activated —
deriving every step from on-chain state, so an interrupted checkout resumes
from exactly where it stopped. The merchant consumes order events,
webhook-style.

> ⚠ Latch is unaudited, experimental, devnet-only software. Do not use with
> real funds.

## Install

```sh
npm install @latch-labs/checkout
```

## Escrow checkout in ten lines

```ts
import { LatchCheckout, LocalMerchantAdapter, watchDeal } from "@latch-labs/checkout";

const checkout = new LatchCheckout({ connection, mint, decimals: 6, merchant, funding });

const session = checkout.createSession({
  item: { id: "desk-01", name: "Vintage writing desk" },
  price: 120,
  subject: "Purchase of: Vintage writing desk — from Your Store",
  guest: Keypair.generate(), // or `buyer: wallet.publicKey` for wallet checkout
});
persist(session.state);                                   // resume-safe from here
await session.advance(null, { onProgress: console.log }); // → escrow Active
watchDeal(merchantClient, session.deal, (e) => e.state === "Active" && ship());
```

Interrupted mid-flight? `checkout.restoreSession(persisted).advance(...)`
continues from on-chain state — never from a step counter.

## The two adapters (where production slots in)

**`MerchantAdapter`** is the seller's side — countersign the agreement,
confirm readiness. On devnet, `LocalMerchantAdapter` runs these SDK calls
in-process from a keypair (in the browser for demos, or server-side in
Node). In production the same interface is served by the merchant's backend
or a hosted API: the buyer flow doesn't change.

**`FundingAdapter`** gets the buyer in a position to pay. On devnet that's a
simulated card that delivers test tokens. On mainnet a licensed onramp
partner implements this interface — the partner holds the money-transmission
relationship; no Latch operator ever holds funds.

## What the buyer signs

`session.agreementText()` returns a deterministic agreement generated from
the deal's on-chain fields (template v1). Its SHA-256 goes on-chain as the
terms hash; any client can regenerate the text and verify it — the deal is
self-verifying. Buyer protection is a `TimeoutRelease` rule: funds
auto-release to the seller after `protectionDays` (default 7) unless the
buyer disputes; release before that takes both parties' approvals. A 2-of-2
buyer+seller recovery role with an on-chain notice delay covers lost keys.

## Order events

`watchDeal(client, deal, onEvent)` polls chain state and fires on every
state transition and milestone release — the webhook contract, served
directly from the chain. A hosted API can deliver the same events over HTTP
without changing merchant code structure.

## See it live

Two demo storefronts run on this package against Solana devnet:
https://latchlabs.org/ — including guest card checkout with
zero wallets and a mid-flight interruption test in CI.

Apache-2.0.

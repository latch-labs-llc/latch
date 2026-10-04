# Latch reference web app

Wallet-connected split-control escrow on Solana **devnet**, built on
[`@latch-labs/sdk`](https://www.npmjs.com/package/@latch-labs/sdk).
Live: https://latchlabs.org/

Two surfaces:

- **Deal app** (`#/`) — create/join deals by share link, hash-verified signing
  ceremony, full lifecycle including dispute/settlement/recovery, printable
  signature certificate rebuilt from on-chain events.
- **Latch Checkout** (`#/store`) — Apple Pay-style pay sheet on a demo
  merchant storefront. Wallet path, or a simulated-card guest path (invisible
  wallet). Checkout is a state-driven resumable machine: progress is derived
  from on-chain state, so interrupted checkouts continue exactly where they
  stopped (`src/lib/checkout.ts`).

## Honest architecture notes (demo-grade by design)

- **Embedded devnet keypairs are deliberate.** The LDD test-token mint
  authority, the SOL tap, and the demo merchant all ship their secret keys in
  the bundle — devnet play money, worthless by construction. Never do this
  with real keys.
- **The "merchant" runs client-side** here, making exactly the SDK calls a
  real integrator's server would (`signTerms`, `confirmReady`,
  `approveMilestone`, `releaseMilestone`). In production those calls move
  server-side behind the merchant's own key management.
- **Guest wallets and orders live in `localStorage`.** Clearing site data
  loses them (and strands their demo tokens). A production guest flow would
  use embedded-wallet infrastructure or a licensed onramp partner's account
  system.
- **The card flow is simulated** and labeled as such in the UI. On mainnet it
  is a licensed onramp partner (MoonPay/Stripe-class) converting the charge to
  stablecoins; nothing about the escrow changes.

## Develop

```sh
npm install
npm run dev          # http://localhost:5173
npm run build
```

End-to-end judge simulations (need a funded devnet keypair at
`~/.config/solana/id.json` and the dev server running):

```sh
node e2e/two-judges.mjs       # two burner wallets, full deal, certificate
node e2e/dispute-path.mjs     # dispute → joint settlement → resolve
node e2e/checkout-guest.mjs   # storefront card path, zero wallets connected
node e2e/checkout-resume.mjs  # kills checkout mid-flight, proves resume
```

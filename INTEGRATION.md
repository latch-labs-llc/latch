# Integrating Latch

Three surfaces, one primitive — pick the layer that fits:

| You are building… | Use | Docs |
| --- | --- | --- |
| A store / marketplace checkout | [`@latch-labs/checkout`](clients/ts/packages/checkout/README.md) | Sessions, adapters, order events |
| Anything else on the protocol | [`@latch-labs/sdk`](clients/ts/packages/sdk/README.md) | All 17 instructions, events, PDAs — [API reference](https://latchlabs.org/api/) |
| Scripts / ops | [`@latch-labs/cli`](clients/ts/packages/cli/README.md) | Full lifecycle from a terminal |

## Escrow checkout in ten lines

```ts
import { LatchCheckout, watchDeal } from "@latch-labs/checkout";

const checkout = new LatchCheckout({ connection, mint, decimals: 6, merchant, funding });
const session = checkout.createSession({
  item: { id: "desk-01", name: "Vintage writing desk" },
  price: 120,
  subject: "Purchase of: Vintage writing desk — from Your Store",
  guest: Keypair.generate(),
});
persist(session.state);                                   // resume-safe from here
await session.advance(null, { onProgress: console.log }); // → escrow Active
watchDeal(merchantClient, session.deal, (e) => e.state === "Active" && ship());
```

The merchant side and buyer funding sit behind two interfaces
(`MerchantAdapter`, `FundingAdapter`): in-process with a keypair on devnet,
a hosted backend and a licensed onramp partner on mainnet — the storefront
code is identical. Both demo storefronts in [`app/`](app/) are consumers of
this package; the smaller one is ~50 lines of integration.

## Notes for integrators

- **Unsigned Drafts naming a user are unsolicited invitations.** Anyone can
  create a deal naming any pubkey as counterparty; surface Drafts only from
  known counterparties (see SECURITY_NOTES.md §13).
- **Events are CPI events** — decode from inner instructions (the SDK's
  `fetchEventHistory`), never from logs.
- **Self-verifying agreements:** regenerate the agreement text client-side
  and check its SHA-256 against the on-chain terms hash before asking anyone
  to sign (`buildAgreement` / `sha256` are exported).
- Latch is **devnet-only and unaudited** — do not use with real funds.

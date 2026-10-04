/**
 * @latch-labs/checkout — escrow checkout sessions for Latch.
 *
 * This package is the client interface of escrow-as-a-service: a merchant
 * creates a checkout session, the buyer pays (wallet or guest), the deal is
 * created / signed by both sides / funded / activated on Solana, and the
 * merchant consumes order events. On devnet the merchant and funding sides
 * run in-process behind adapters; on mainnet the same interfaces are served
 * by a hosted API and a licensed onramp partner.
 *
 * ```ts
 * const checkout = new LatchCheckout({ connection, mint, decimals, merchant, funding });
 * const session = checkout.createSession({ item, price: 120, subject, guest: Keypair.generate() });
 * persist(session.state); // resume-safe from this moment
 * await session.advance(null, { onProgress: console.log }); // → escrow Active
 * watchDeal(merchantClient, session.deal, (e) => { if (e.state === "Active") ship(); });
 * ```
 *
 * ⚠ Latch is unaudited, experimental, devnet-only software. Do not use with
 * real funds.
 */
export { buildAgreement, sha256, hex } from "./agreement";
export type { AgreementParams } from "./agreement";
export { keypairClient, keypairWallet, LocalMerchantAdapter } from "./merchant";
export { withRetry } from "./retry";
export { CheckoutSession, LatchCheckout } from "./session";
export type { CreateSessionParams } from "./session";
export type {
  CheckoutConfig,
  CheckoutItem,
  CheckoutSessionState,
  FundingAdapter,
  MerchantAdapter,
} from "./types";
export { watchDeal } from "./watch";
export type { DealEvent, WatchHandle } from "./watch";

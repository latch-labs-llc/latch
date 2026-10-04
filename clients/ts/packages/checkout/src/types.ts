import { LatchClient } from "@latch-labs/sdk";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";

/** The thing being bought — display metadata only; price travels separately. */
export interface CheckoutItem {
  id: string;
  name: string;
}

/**
 * Serializable session state. Persist this (localStorage, a database, a
 * cookie) before the first transaction and a checkout interrupted at ANY
 * point can be restored and resumed — progress is always derived from
 * on-chain state, never from this record.
 */
export interface CheckoutSessionState {
  itemId: string;
  itemName: string;
  /** Price in UI units of the configured mint (e.g. dollars for a 6-decimal stable). */
  price: number;
  /** Agreement subject line, e.g. "Purchase of: Desk — from Example Store". */
  subject: string;
  dealId: number;
  /** Deal account address (base58). */
  deal: string;
  /** Buyer wallet (base58). */
  buyer: string;
  /** Present only for guest checkout: the invisible wallet's secret key. */
  guestSecret?: number[];
}

/**
 * The merchant's side of a checkout. In production this is served by the
 * merchant's backend (or Latch Labs' hosted API) — the buyer's flow calls
 * these when it needs the seller's signature. On devnet the demo implements
 * it in-browser with a demo keypair; the interface is identical.
 */
export interface MerchantAdapter {
  publicKey: PublicKey;
  /** Countersign the agreement for this deal (merchant's terms signature). */
  countersign(deal: PublicKey): Promise<void>;
  /** Confirm readiness to perform once the deal is funded. */
  confirmReady(deal: PublicKey): Promise<void>;
}

/**
 * Gets the buyer in a position to pay: on devnet a simulated card / test
 * faucet; on mainnet a licensed onramp partner slots in behind this exact
 * interface. Implementations must be idempotent — called before create and
 * again before deposit.
 */
export interface FundingAdapter {
  ensureFunds(args: {
    connection: Connection;
    buyer: PublicKey;
    /** Price still owed, in UI units. */
    price: number;
    /** Guest wallet keypair when this is a guest checkout (it signs its own txs). */
    guestKeypair?: Keypair;
    /** The buyer's LatchClient (its provider can send transactions for wallet buyers). */
    client: LatchClient;
    onProgress: (message: string) => void;
  }): Promise<void>;
}

export interface CheckoutConfig {
  connection: Connection;
  /** Settlement mint (e.g. a stablecoin; LDD on devnet). */
  mint: PublicKey;
  decimals: number;
  merchant: MerchantAdapter;
  funding?: FundingAdapter;
  /** Buyer-protection window: auto-release to the seller after this many days unless disputed. */
  protectionDays?: number;
  /** On-chain notice delay for the 2-of-2 recovery role, seconds. */
  recoveryDelaySecs?: number;
}

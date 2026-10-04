/**
 * Merchant-side helpers. `LocalMerchantAdapter` runs the merchant's SDK calls
 * in-process from a Keypair — the devnet demo shape, also usable server-side
 * in Node. A hosted implementation (Latch Labs' API, or the merchant's own
 * backend) serves the same `MerchantAdapter` interface over HTTP.
 */
import { AnchorProvider } from "@anchor-lang/core";
import { LatchClient } from "@latch-labs/sdk";
import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  VersionedTransaction,
} from "@solana/web3.js";
import { MerchantAdapter } from "./types";
import { withRetry } from "./retry";

/** Minimal wallet around a Keypair (works in browsers; anchor's Wallet is node-only). */
export function keypairWallet(kp: Keypair) {
  const sign = <T extends Transaction | VersionedTransaction>(tx: T): T => {
    if (tx instanceof VersionedTransaction) tx.sign([kp]);
    else tx.partialSign(kp);
    return tx;
  };
  return {
    publicKey: kp.publicKey,
    signTransaction: async <T extends Transaction | VersionedTransaction>(tx: T) => sign(tx),
    signAllTransactions: async <T extends Transaction | VersionedTransaction>(txs: T[]) =>
      txs.map(sign),
  };
}

export function keypairClient(connection: Connection, kp: Keypair): LatchClient {
  const provider = new AnchorProvider(connection, keypairWallet(kp) as any, {
    commitment: "confirmed",
  });
  return LatchClient.fromProvider(provider);
}

export class LocalMerchantAdapter implements MerchantAdapter {
  readonly publicKey: PublicKey;
  private readonly client: () => LatchClient;

  constructor(connection: Connection, keypair: Keypair) {
    this.publicKey = keypair.publicKey;
    this.client = () => keypairClient(connection, keypair);
  }

  async countersign(deal: PublicKey): Promise<void> {
    await withRetry("merchant countersign", () => this.client().signTerms(deal, true).rpc());
  }

  async confirmReady(deal: PublicKey): Promise<void> {
    await withRetry("merchant ready", () => this.client().confirmReady(deal).rpc());
  }
}

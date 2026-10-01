/**
 * Demo merchant for the storefront — "Walnut & Oak Furniture Co."
 *
 * In a real integration the merchant side runs server-side with the SDK: the
 * merchant's service watches for new deals, countersigns the agreement, and
 * confirms readiness after accepting the order. This demo performs those same
 * SDK calls client-side with an embedded devnet keypair so judges can watch
 * the full two-party flow without us hosting a backend. The secret is
 * deliberately public — devnet play money only.
 */
import { AnchorProvider } from "@anchor-lang/core";
import { LatchClient } from "@latch-labs/sdk";
import { Connection, Keypair, PublicKey, Transaction, VersionedTransaction } from "@solana/web3.js";

export const MERCHANT_NAME = "Walnut & Oak Furniture Co.";

const MERCHANT_SECRET = new Uint8Array([
  192, 15, 102, 9, 56, 132, 118, 177, 148, 2, 79, 10, 69, 12, 203, 172, 171,
  97, 217, 34, 20, 244, 243, 242, 127, 86, 217, 0, 148, 224, 127, 47, 4, 96,
  103, 0, 119, 87, 97, 32, 142, 36, 62, 13, 177, 43, 22, 249, 45, 100, 125,
  209, 199, 184, 253, 199, 16, 246, 66, 28, 235, 175, 73, 227,
]);

export function merchantKeypair(): Keypair {
  return Keypair.fromSecretKey(MERCHANT_SECRET);
}
export const MERCHANT_PUBKEY = merchantKeypair().publicKey;

/** Minimal in-browser wallet around a Keypair (anchor Wallet is node-only). */
export function keypairWallet(kp: Keypair) {
  return {
    publicKey: kp.publicKey,
    signTransaction: async <T extends Transaction | VersionedTransaction>(tx: T): Promise<T> => {
      if (tx instanceof VersionedTransaction) tx.sign([kp]);
      else tx.partialSign(kp);
      return tx;
    },
    signAllTransactions: async <T extends Transaction | VersionedTransaction>(txs: T[]): Promise<T[]> => {
      for (const tx of txs) {
        if (tx instanceof VersionedTransaction) tx.sign([kp]);
        else tx.partialSign(kp);
      }
      return txs;
    },
  };
}

export function merchantClient(connection: Connection): LatchClient {
  const provider = new AnchorProvider(connection, keypairWallet(merchantKeypair()) as any, {
    commitment: "confirmed",
  });
  return LatchClient.fromProvider(provider);
}

/** The merchant's order-acceptance routine: countersign + confirm ready. */
export async function merchantAccept(connection: Connection, deal: PublicKey): Promise<void> {
  const m = merchantClient(connection);
  await m.signTerms(deal, true).rpc();
  // confirm_ready becomes valid only once the deal is Funded; the caller
  // invokes acceptReady() after the buyer's deposit lands.
}

export async function merchantReady(connection: Connection, deal: PublicKey): Promise<void> {
  const m = merchantClient(connection);
  await m.confirmReady(deal).rpc();
}

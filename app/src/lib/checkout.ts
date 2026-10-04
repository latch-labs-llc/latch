/**
 * Walnut & Oak's integration of @latch-labs/checkout — the app-specific
 * wiring is just three choices: the settlement mint (LDD test dollars), the
 * merchant adapter (demo keypair in-browser; a real store runs the same
 * interface server-side), and the funding adapter (simulated card on devnet;
 * a licensed onramp partner on mainnet).
 */
import {
  CheckoutSessionState,
  FundingAdapter,
  LatchCheckout,
  LocalMerchantAdapter,
} from "@latch-labs/checkout";
import { LatchClient } from "@latch-labs/sdk";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { LDD_DECIMALS, LDD_MINT, buildLddFaucetTx, lddAta, tokenUiBalance } from "./ldd";
import { MERCHANT_NAME, merchantKeypair } from "./merchant";
import { tapSol } from "./soltap";

export { withRetry } from "@latch-labs/checkout";
export type PendingOrder = CheckoutSessionState;
export const PROTECTION_DAYS = 7;

/** Devnet "card": SOL tap for fees + LDD faucet for the purchase amount. */
export const simulatedCard: FundingAdapter = {
  async ensureFunds({ connection, buyer, price, guestKeypair, client, onProgress }) {
    const sol = await connection.getBalance(buyer);
    if (sol < 0.02 * 1e9) {
      await tapSol(connection, buyer);
      onProgress("  · devnet fee money added");
    }
    const bal = await tokenUiBalance(connection, lddAta(buyer));
    if (bal < price) {
      const { tx, authority } = buildLddFaucetTx(buyer, price);
      if (guestKeypair) {
        tx.feePayer = buyer;
        tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
        tx.sign(guestKeypair, authority);
        const sig = await connection.sendRawTransaction(tx.serialize());
        await connection.confirmTransaction(sig, "confirmed");
      } else {
        const provider = client.program.provider as any;
        await provider.sendAndConfirm(tx, [authority]);
      }
      onProgress(
        guestKeypair ? "  · simulated card charged → test dollars delivered" : "  · demo balance topped up"
      );
    }
  },
};

export function makeCheckout(connection: Connection): LatchCheckout {
  return new LatchCheckout({
    connection,
    mint: LDD_MINT,
    decimals: LDD_DECIMALS,
    merchant: new LocalMerchantAdapter(connection, merchantKeypair()),
    funding: simulatedCard,
    protectionDays: PROTECTION_DAYS,
  });
}

// ---- pending-order persistence (host-side concern, not the package's) ----

const PENDING_KEY = "latch-pending";
export const loadPending = (): PendingOrder | null => {
  const raw = JSON.parse(localStorage.getItem(PENDING_KEY) ?? "null");
  if (raw && raw.productId && !raw.itemId) {
    // migrate records persisted by the pre-package engine
    raw.itemId = raw.productId;
    raw.itemName = raw.name;
  }
  return raw;
};
export const savePending = (p: PendingOrder | null) =>
  p ? localStorage.setItem(PENDING_KEY, JSON.stringify(p)) : localStorage.removeItem(PENDING_KEY);

// ---- Store.tsx-facing wrappers over the package API ----

export function newPending(
  connection: Connection,
  product: { id: string; name: string; price: number },
  buyer: PublicKey,
  guest?: Keypair
): PendingOrder {
  return makeCheckout(connection).createSession({
    item: { id: product.id, name: product.name },
    price: product.price,
    subject: `Purchase of: ${product.name} — from ${MERCHANT_NAME}`,
    buyer: guest ? undefined : buyer,
    guest,
  }).state;
}

export function buyerClientFor(
  connection: Connection,
  pending: PendingOrder,
  connected: LatchClient | null
): { client: LatchClient; kp?: Keypair } {
  const { client, guestKeypair } = makeCheckout(connection)
    .restoreSession(pending)
    .buyerClient(connected);
  return { client, kp: guestKeypair };
}

export async function advanceCheckout(
  connection: Connection,
  pending: PendingOrder,
  client: LatchClient,
  _kp: Keypair | undefined,
  push: (m: string) => void
): Promise<void> {
  await makeCheckout(connection).restoreSession(pending).advance(client, { onProgress: push });
}

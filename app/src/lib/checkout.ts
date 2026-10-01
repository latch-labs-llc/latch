/**
 * State-driven checkout engine. Every step is derived from ON-CHAIN state, so
 * a checkout interrupted at any point (RPC hiccup, closed tab) resumes from
 * exactly where it stopped: the pending order is persisted before the first
 * transaction, and `advanceCheckout` simply keeps moving the deal forward
 * until it is Active. All RPC calls retry with backoff.
 */
import { AnchorProvider, BN } from "@anchor-lang/core";
import { DeadlockRule, LATCH_PROGRAM_ID, LatchClient, RiskFlags, TimerMode, dealPda } from "@latch-labs/sdk";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { buildAgreement, sha256 } from "./agreement";
import { LDD_DECIMALS, LDD_MINT, buildLddFaucetTx, lddAta, tokenUiBalance } from "./ldd";
import { MERCHANT_PUBKEY, keypairWallet, merchantClient } from "./merchant";
import { tapSol } from "./soltap";

export const PROTECTION_DAYS = 7;

export interface PendingOrder {
  productId: string;
  name: string;
  price: number; // ui units
  subject: string;
  dealId: number;
  deal: string;
  buyer: string;
  guestSecret?: number[];
}

const PENDING_KEY = "latch-pending";
export const loadPending = (): PendingOrder | null =>
  JSON.parse(localStorage.getItem(PENDING_KEY) ?? "null");
export const savePending = (p: PendingOrder | null) =>
  p ? localStorage.setItem(PENDING_KEY, JSON.stringify(p)) : localStorage.removeItem(PENDING_KEY);

export async function withRetry<T>(label: string, fn: () => Promise<T>, tries = 3): Promise<T> {
  let last: any;
  for (let i = 0; i < tries; i++) {
    try {
      return await fn();
    } catch (e: any) {
      last = e;
      const msg = String(e?.message ?? e);
      // Don't retry deterministic program rejections — only transient transport.
      if (msg.includes("Error Code") || msg.includes("custom program error")) throw e;
      if (i < tries - 1) await new Promise((r) => setTimeout(r, 1500 * (i + 1)));
    }
  }
  throw new Error(`${label} failed after ${tries} attempts: ${last?.message ?? last}`);
}

export function buyerClientFor(connection: Connection, pending: PendingOrder, connected: LatchClient | null): { client: LatchClient; kp?: Keypair } {
  if (pending.guestSecret) {
    const kp = Keypair.fromSecretKey(Uint8Array.from(pending.guestSecret));
    const provider = new AnchorProvider(connection, keypairWallet(kp) as any, { commitment: "confirmed" });
    return { client: LatchClient.fromProvider(provider), kp };
  }
  if (!connected || connected.program.provider.publicKey!.toBase58() !== pending.buyer) {
    throw new Error(`connect the wallet this order was started with (${pending.buyer.slice(0, 6)}…)`);
  }
  return { client: connected };
}

export function checkoutAgreement(pending: PendingOrder): ReturnType<typeof buildAgreement> {
  return buildAgreement({
    deal: new PublicKey(pending.deal),
    dealId: String(pending.dealId),
    buyer: new PublicKey(pending.buyer),
    seller: MERCHANT_PUBKEY,
    mint: LDD_MINT,
    decimals: LDD_DECIMALS,
    milestoneAmounts: [BigInt(pending.price) * BigInt(10 ** LDD_DECIMALS)],
    ruleName: "TimeoutRelease",
    timerModeName: "FromActivation",
    timeoutSecs: String(PROTECTION_DAYS * 86400),
    recoverySigners: [new PublicKey(pending.buyer), MERCHANT_PUBKEY],
    recoveryThreshold: 2,
    recoveryDelaySecs: "259200",
    subject: pending.subject,
  });
}

export function newPending(product: { id: string; name: string; price: number }, buyer: PublicKey, guest?: Keypair): PendingOrder {
  const dealId = Math.floor(Date.now() / 1000);
  const subject = `Purchase of: ${product.name} — from Walnut & Oak Furniture Co.`;
  return {
    productId: product.id,
    name: product.name,
    price: product.price,
    subject,
    dealId,
    deal: dealPda(buyer, dealId, LATCH_PROGRAM_ID).toBase58(),
    buyer: buyer.toBase58(),
    guestSecret: guest ? Array.from(guest.secretKey) : undefined,
  };
}

/**
 * Drive the deal to Active from whatever state it is in. Calls `push` with
 * progress lines. Returns when Active. Safe to call repeatedly.
 */
export async function advanceCheckout(
  connection: Connection,
  pending: PendingOrder,
  client: LatchClient,
  kp: Keypair | undefined,
  push: (m: string) => void
): Promise<void> {
  const deal = new PublicKey(pending.deal);
  const buyer = new PublicKey(pending.buyer);
  const raw = BigInt(pending.price) * BigInt(10 ** LDD_DECIMALS);
  const merchant = () => merchantClient(connection);

  const ensureBalances = async () => {
    const sol = await withRetry("balance check", () => connection.getBalance(buyer));
    if (sol < 0.02 * 1e9) {
      await withRetry("SOL tap", () => tapSol(connection, buyer));
      push("  · devnet fee money added");
    }
    const ata = lddAta(buyer);
    const bal = await tokenUiBalance(connection, ata);
    if (bal < pending.price) {
      await withRetry("test-dollar delivery", async () => {
        const { tx, authority } = buildLddFaucetTx(buyer, pending.price);
        if (kp) {
          tx.feePayer = buyer;
          tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
          tx.sign(kp, authority);
          const sig = await connection.sendRawTransaction(tx.serialize());
          await connection.confirmTransaction(sig, "confirmed");
        } else {
          const provider = client.program.provider as AnchorProvider;
          await provider.sendAndConfirm!(tx, [authority]);
        }
      });
      push(kp ? "  · simulated card charged → test dollars delivered" : "  · demo balance topped up");
    }
  };

  for (let guard = 0; guard < 12; guard++) {
    let state = "Missing";
    let account: any = null;
    try {
      const d = await withRetry("deal lookup", () => client.fetchDeal(deal));
      state = d.state;
      account = d.account;
    } catch {
      /* not created yet */
    }

    if (state === "Missing") {
      push("Preparing buyer balances…");
      await ensureBalances();
      push("Creating the escrow deal…");
      const termsHash = await sha256(checkoutAgreement(pending));
      await withRetry("create deal", () =>
        client
          .createDeal({
            dealId: pending.dealId,
            parties: [buyer, MERCHANT_PUBKEY],
            payerIdx: 0,
            payeeIdx: 1,
            approvalThreshold: 2,
            termsHash,
            milestoneAmounts: [new BN(raw.toString())],
            deadlockRule: DeadlockRule.TimeoutRelease,
            timerMode: TimerMode.FromActivation,
            timeoutSecs: new BN(PROTECTION_DAYS * 86400),
            recoverySigners: [buyer, MERCHANT_PUBKEY],
            recoveryThreshold: 2,
            recoveryDelaySecs: new BN(259200),
            acceptedRiskFlags: RiskFlags.ALL,
            mint: LDD_MINT,
          })
          .rpc()
      );
    } else if (state === "Draft") {
      if (!(account.signed & 1)) {
        push("Signing the agreement (your signature)…");
        await withRetry("buyer signature", () => client.signTerms(deal, true).rpc());
      } else if (!(account.signed & 2)) {
        push("Merchant countersigning (server-side SDK call)…");
        await withRetry("merchant signature", () => merchant().signTerms(deal, true).rpc());
      }
    } else if (state === "Signed") {
      push(`Placing $${pending.price}.00 into escrow…`);
      await ensureBalances();
      await withRetry("deposit", async () =>
        (await client.deposit(deal, new BN(raw.toString()), lddAta(buyer))).rpc()
      );
    } else if (state === "Funded") {
      if (!(account.ready & 1)) {
        push("Confirming ready (buyer)…");
        await withRetry("buyer ready", () => client.confirmReady(deal).rpc());
      } else {
        push("Confirming ready (merchant)…");
        await withRetry("merchant ready", () => merchant().confirmReady(deal).rpc());
      }
    } else if (state === "Active") {
      return; // done
    } else {
      throw new Error(`unexpected deal state during checkout: ${state}`);
    }
  }
  throw new Error("checkout did not converge — press Resume to continue");
}

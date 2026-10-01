/**
 * Demo merchant storefront + "Latch Checkout" pay sheet.
 *
 * The Apple Pay-style experience over split-control escrow: a buyer pays with
 * a connected Solana wallet, or with a (clearly simulated, devnet-only) card
 * flow in which a guest wallet is created invisibly — the "never touch
 * crypto" path. Real fiat on/off-ramps are a mainnet roadmap item via
 * licensed onramp partners; nothing here pretends otherwise.
 */
import { AnchorProvider, BN } from "@anchor-lang/core";
import { DeadlockRule, LATCH_PROGRAM_ID, LatchClient, RiskFlags, TimerMode, dealPda } from "@latch-labs/sdk";
import {
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { Keypair, PublicKey } from "@solana/web3.js";
import { useEffect, useState } from "react";
import { buildAgreement, sha256 } from "../lib/agreement";
import { LDD_DECIMALS, LDD_MINT, buildLddFaucetTx, lddAta, tokenUiBalance } from "../lib/ldd";
import { MERCHANT_NAME, MERCHANT_PUBKEY, keypairWallet, merchantClient } from "../lib/merchant";
import { tapSol } from "../lib/soltap";
import { short, useLatch } from "../lib/useLatch";

const PRODUCTS = [
  { id: "desk", emoji: "🪵", name: "Vintage mahogany writing desk", price: 120, blurb: "1920s, restored. Ships freight." },
  { id: "chair", emoji: "🪑", name: "Walnut captain's chair", price: 45, blurb: "Hand-finished, solid walnut." },
  { id: "lamp", emoji: "💡", name: "Oak & brass reading lamp", price: 30, blurb: "Rewired, warm glow included." },
] as const;
type Product = (typeof PRODUCTS)[number];

const PROTECTION_DAYS = 7;
interface Order {
  deal: string;
  subject: string;
  guestSecret?: number[];
}
const loadOrders = (): Order[] => JSON.parse(localStorage.getItem("latch-orders") ?? "[]");
const saveOrder = (o: Order) => localStorage.setItem("latch-orders", JSON.stringify([o, ...loadOrders()]));

export default function Store() {
  const [checkout, setCheckout] = useState<Product | null>(null);
  const [orders, setOrders] = useState<Order[]>(loadOrders());

  return (
    <div className="store">
      <div className="store-hero">
        <h1>🌳 {MERCHANT_NAME}</h1>
        <p>Fine furniture from strangers on the internet — protected by Latch escrow, not by promises.</p>
        <p className="muted">
          Demo storefront on Solana devnet. Prices in LDD test dollars. Merchant wallet:{" "}
          <code>{short(MERCHANT_PUBKEY.toBase58(), 4)}</code>
        </p>
      </div>
      <div className="products">
        {PRODUCTS.map((p) => (
          <div className="product" key={p.id}>
            <div className="pimg">{p.emoji}</div>
            <h3>{p.name}</h3>
            <p className="muted">{p.blurb}</p>
            <div className="row spread">
              <b>${p.price}.00</b>
              <button className="latchpay" onClick={() => setCheckout(p)}>
                ⟟ Pay with Latch
              </button>
            </div>
          </div>
        ))}
      </div>
      {orders.length > 0 && <Orders orders={orders} refresh={() => setOrders(loadOrders())} />}
      {checkout && (
        <CheckoutSheet
          product={checkout}
          onClose={() => setCheckout(null)}
          onOrdered={() => setOrders(loadOrders())}
        />
      )}
      <p className="center muted">
        This is the embeddable "Latch Checkout" experience — any merchant adds it with the{" "}
        <a href="https://www.npmjs.com/package/@latch-labs/sdk" target="_blank" rel="noreferrer">SDK</a>. Funds sit in
        split-control escrow: the merchant can't take them early, the buyer can't claw them back after delivery.
      </p>
    </div>
  );
}

// ------------------------------------------------------------------ sheet

function CheckoutSheet({ product, onClose, onOrdered }: { product: Product; onClose: () => void; onOrdered: () => void }) {
  const { connection } = useConnection();
  const { publicKey } = useWallet();
  const connected = useLatch();
  const [mode, setMode] = useState<"options" | "card" | "processing" | "done">("options");
  const [agree, setAgree] = useState(false);
  const [log, setLog] = useState<string[]>([]);
  const [error, setError] = useState("");
  const [dealAddr, setDealAddr] = useState("");
  const subject = `Purchase of: ${product.name} — from ${MERCHANT_NAME}`;

  const push = (m: string) => setLog((l) => [...l, m]);

  async function runCheckout(buyerClient: LatchClient, buyerKp?: Keypair) {
    setMode("processing");
    setError("");
    try {
      const buyer = buyerClient.program.provider.publicKey!;
      const raw = BigInt(product.price) * BigInt(10 ** LDD_DECIMALS);

      push("Preparing buyer balances…");
      const sol = await connection.getBalance(buyer);
      if (sol < 0.02 * 1e9) {
        await tapSol(connection, buyer);
        push("  · devnet fee money added");
      }
      const ata = lddAta(buyer);
      const bal = await tokenUiBalance(connection, ata);
      if (bal < product.price) {
        const { tx, authority } = buildLddFaucetTx(buyer, product.price);
        if (buyerKp) {
          tx.feePayer = buyer;
          tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
          tx.sign(buyerKp, authority);
          const sig = await connection.sendRawTransaction(tx.serialize());
          await connection.confirmTransaction(sig, "confirmed");
        } else {
          const provider = buyerClient.program.provider as AnchorProvider;
          await provider.sendAndConfirm!(tx, [authority]);
        }
        push(buyerKp ? "  · simulated card charged → test dollars delivered" : "  · demo balance topped up");
      }

      push("Creating the escrow deal…");
      const dealId = Math.floor(Date.now() / 1000);
      const deal = dealPda(buyer, dealId, LATCH_PROGRAM_ID);
      const text = buildAgreement({
        deal,
        dealId: String(dealId),
        buyer,
        seller: MERCHANT_PUBKEY,
        mint: LDD_MINT,
        decimals: LDD_DECIMALS,
        milestoneAmounts: [raw],
        ruleName: "TimeoutRelease",
        timerModeName: "FromActivation",
        timeoutSecs: String(PROTECTION_DAYS * 86400),
        recoverySigners: [buyer, MERCHANT_PUBKEY],
        recoveryThreshold: 2,
        recoveryDelaySecs: "259200",
        subject,
      });
      const termsHash = await sha256(text);
      await buyerClient
        .createDeal({
          dealId,
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
        .rpc();

      push("Signing the agreement (your signature)…");
      await buyerClient.signTerms(deal, true).rpc();

      push("Merchant countersigning (server-side SDK call)…");
      await merchantClient(connection).signTerms(deal, true).rpc();

      push(`Placing $${product.price}.00 into escrow…`);
      await (await buyerClient.deposit(deal, new BN(raw.toString()), ata)).rpc();

      push("Activating the deal (both sides ready)…");
      await buyerClient.confirmReady(deal).rpc();
      await merchantClient(connection).confirmReady(deal).rpc();

      saveOrder({ deal: deal.toBase58(), subject, guestSecret: buyerKp ? Array.from(buyerKp.secretKey) : undefined });
      setDealAddr(deal.toBase58());
      setMode("done");
      onOrdered();
    } catch (e: any) {
      setError(e?.error?.errorMessage ?? e?.message ?? String(e));
      setMode("options");
    }
  }

  const payWithWallet = () => {
    if (!connected) {
      setError("Connect a wallet first (top right) — or use the card option for guest checkout.");
      return;
    }
    runCheckout(connected);
  };

  const payWithCard = () => {
    // Guest checkout: an invisible wallet is created for the buyer. On
    // mainnet this is where a licensed onramp partner converts the card
    // charge to stablecoins; on devnet we simulate it with test dollars.
    const kp = Keypair.generate();
    const provider = new AnchorProvider(connection, keypairWallet(kp) as any, { commitment: "confirmed" });
    runCheckout(LatchClient.fromProvider(provider), kp);
  };

  return (
    <div className="sheet-backdrop" onClick={mode === "processing" ? undefined : onClose}>
      <div className="sheet" onClick={(e) => e.stopPropagation()}>
        {mode !== "done" && (
          <>
            <div className="row spread">
              <h3 style={{ margin: 0 }}>⟟ Latch Checkout</h3>
              {mode !== "processing" && <button className="ghost" onClick={onClose}>✕</button>}
            </div>
            <div className="sheet-item">
              <span className="pemoji">{product.emoji}</span>
              <div>
                <b>{product.name}</b>
                <div className="muted">{MERCHANT_NAME}</div>
              </div>
              <div className="price">${product.price}.00</div>
            </div>
            <div className="protection">
              🛡 <b>Latch Buyer–Seller Protection.</b> Your payment is held in escrow that neither you nor the
              merchant can take alone. It releases when you confirm delivery — or automatically after{" "}
              {PROTECTION_DAYS} days unless you open a dispute (a dispute pauses the clock). Every signature is
              recorded on-chain.
            </div>
          </>
        )}

        {mode === "options" && (
          <>
            <label className="consent">
              <input type="checkbox" checked={agree} onChange={(e) => setAgree(e.target.checked)} /> I agree to the
              escrow agreement for this purchase and consent to sign electronically. (The full agreement is hashed
              on-chain; view it on the deal page after checkout.)
            </label>
            <button className="primary wide" disabled={!agree} onClick={payWithWallet}>
              Pay with Solana wallet{publicKey ? ` (${short(publicKey.toBase58())})` : ""}
            </button>
            <button className="wide" disabled={!agree} onClick={() => setMode("card")}>
              💳 Pay with card
            </button>
            {error && <div className="error">{error}</div>}
          </>
        )}

        {mode === "card" && (
          <>
            <div className="simbanner">
              SIMULATED — devnet only. In production this is a licensed onramp partner (MoonPay / Stripe); your
              card buys stablecoins that go straight into escrow. Here it delivers test dollars.
            </div>
            <label>Card number<input defaultValue="4242 4242 4242 4242" /></label>
            <div className="row">
              <label style={{ flex: 1 }}>Expiry<input defaultValue="04/42" /></label>
              <label style={{ flex: 1 }}>CVC<input defaultValue="042" /></label>
            </div>
            <label>Name on card<input defaultValue="Jane Judge" /></label>
            <button className="primary wide" onClick={payWithCard}>
              Pay ${product.price}.00
            </button>
            <p className="muted center">Guest checkout — a wallet is created for you invisibly. No extension needed.</p>
          </>
        )}

        {mode === "processing" && (
          <div className="processing">
            {log.map((l, i) => (
              <div key={i}>{l}</div>
            ))}
            <div className="spinline">⏳ working on devnet…</div>
          </div>
        )}

        {mode === "done" && (
          <div className="center">
            <h2>🎉 Order placed</h2>
            <p>
              <b>${product.price}.00</b> is in split-control escrow for “{product.name}.”
              The merchant has countersigned; the deal is active on Solana devnet.
            </p>
            <p>
              <a className="btnlike" href={`#/deal/${dealAddr}?s=${encodeURIComponent(subject)}`}>View deal</a>{" "}
              <a className="btnlike gold" href={`#/cert/${dealAddr}?s=${encodeURIComponent(subject)}`}>Certificate</a>
            </p>
            <button className="wide" onClick={onClose}>Back to the store</button>
          </div>
        )}
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ orders

function Orders({ orders, refresh }: { orders: Order[]; refresh: () => void }) {
  const { connection } = useConnection();
  const connected = useLatch();
  const [states, setStates] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState("");

  const load = async () => {
    const anyClient = connected ?? merchantClient(connection);
    const out: Record<string, string> = {};
    for (const o of orders) {
      try {
        out[o.deal] = (await anyClient.fetchDeal(new PublicKey(o.deal))).state;
      } catch {
        out[o.deal] = "?";
      }
    }
    setStates(out);
  };
  useEffect(() => {
    load();
  }, [orders.length, connected]);

  const confirmDelivery = async (o: Order) => {
    setBusy(o.deal);
    setError("");
    try {
      const deal = new PublicKey(o.deal);
      const buyerClient = o.guestSecret
        ? LatchClient.fromProvider(
            new AnchorProvider(
              connection,
              keypairWallet(Keypair.fromSecretKey(Uint8Array.from(o.guestSecret))) as any,
              { commitment: "confirmed" }
            )
          )
        : connected;
      if (!buyerClient) throw new Error("connect the wallet you bought with");
      await buyerClient.approveMilestone(deal, 0).rpc();
      const m = merchantClient(connection);
      await m.approveMilestone(deal, 0).rpc();
      const d = await m.fetchDeal(deal);
      const merchantAta = getAssociatedTokenAddressSync(d.account.mint, MERCHANT_PUBKEY, true, d.account.tokenProgram);
      await (await m.releaseMilestone(deal, 0, merchantAta))
        .preInstructions([
          createAssociatedTokenAccountIdempotentInstruction(MERCHANT_PUBKEY, merchantAta, MERCHANT_PUBKEY, d.account.mint, d.account.tokenProgram),
        ])
        .rpc();
      await load();
    } catch (e: any) {
      setError(e?.error?.errorMessage ?? e?.message ?? String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="card">
      <h3>Your orders</h3>
      {orders.map((o) => (
        <div className="milestone" key={o.deal}>
          <div>
            {o.subject.replace("Purchase of: ", "").split(" — ")[0]} ·{" "}
            <span className="badge">{states[o.deal] ?? "…"}</span>{" "}
            <a href={`#/deal/${o.deal}?s=${encodeURIComponent(o.subject)}`}>deal</a> ·{" "}
            <a href={`#/cert/${o.deal}?s=${encodeURIComponent(o.subject)}`}>certificate</a>
          </div>
          {states[o.deal] === "Active" && (
            <button className="primary" disabled={!!busy} onClick={() => confirmDelivery(o)}>
              {busy === o.deal ? "…" : "Confirm delivery — release payment"}
            </button>
          )}
        </div>
      ))}
      {error && <div className="error">{error}</div>}
    </div>
  );
}

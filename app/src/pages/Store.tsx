/**
 * Demo merchant storefront + "Latch Checkout" pay sheet.
 *
 * The Apple Pay-style experience over split-control escrow. Checkout is a
 * state-driven, resumable machine (see lib/checkout.ts): progress is persisted
 * before the first transaction and derived from on-chain state, so an
 * interrupted checkout resumes exactly where it stopped. Real fiat on/off
 * ramps are a mainnet roadmap item via licensed partners; the card path here
 * is clearly labeled as simulated.
 */
import { AnchorProvider } from "@anchor-lang/core";
import { LatchClient } from "@latch-labs/sdk";
import {
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { Keypair, PublicKey } from "@solana/web3.js";
import { useEffect, useState } from "react";
import {
  PendingOrder,
  advanceCheckout,
  buyerClientFor,
  loadPending,
  newPending,
  savePending,
  withRetry,
} from "../lib/checkout";
import { MERCHANT_NAME, MERCHANT_PUBKEY, keypairWallet, merchantClient } from "../lib/merchant";
import { short, useLatch, useLatchOrReadonly } from "../lib/useLatch";

const PRODUCTS = [
  { id: "desk", emoji: "🪵", name: "Vintage mahogany writing desk", price: 120, blurb: "1920s, restored. Ships freight." },
  { id: "chair", emoji: "🪑", name: "Walnut captain's chair", price: 45, blurb: "Hand-finished, solid walnut." },
  { id: "lamp", emoji: "💡", name: "Oak & brass reading lamp", price: 30, blurb: "Rewired, warm glow included." },
] as const;
type Product = (typeof PRODUCTS)[number];

interface Order {
  deal: string;
  subject: string;
  guestSecret?: number[];
}
const loadOrders = (): Order[] => JSON.parse(localStorage.getItem("latch-orders") ?? "[]");
const saveOrder = (o: Order) => localStorage.setItem("latch-orders", JSON.stringify([o, ...loadOrders()]));

export default function Store() {
  const [checkout, setCheckout] = useState<Product | null>(null);
  const [resume, setResume] = useState<PendingOrder | null>(loadPending());
  const [orders, setOrders] = useState<Order[]>(loadOrders());

  const ordered = () => {
    setOrders(loadOrders());
    setResume(loadPending());
  };

  return (
    <div className="store">
      <div className="store-hero">
        <h1>🌳 {MERCHANT_NAME}</h1>
        <p>Fine furniture from strangers on the internet — protected by Latch, not by promises.</p>
        <p className="muted">
          Demo storefront on Solana devnet. Prices in LDD test dollars. Merchant wallet:{" "}
          <code>{short(MERCHANT_PUBKEY.toBase58(), 4)}</code>
        </p>
      </div>

      {resume && !checkout && (
        <div className="card promo">
          <div className="row spread">
            <div>
              <b>Unfinished checkout:</b> {resume.itemName} (${resume.price}.00)
              <div className="muted">Interrupted mid-payment — it can continue exactly where it stopped.</div>
            </div>
            <div>
              <button className="primary" onClick={() => setCheckout(PRODUCTS.find((p) => p.id === resume.itemId) ?? PRODUCTS[0])}>
                Resume
              </button>{" "}
              <button
                onClick={() => {
                  savePending(null);
                  setResume(null);
                }}
              >
                Discard
              </button>
            </div>
          </div>
        </div>
      )}

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
      {orders.length > 0 && <Orders orders={orders} />}
      {checkout && (
        <CheckoutSheet
          product={checkout}
          pending={resume && resume.itemId === checkout.id ? resume : null}
          onClose={() => {
            setCheckout(null);
            setResume(loadPending());
          }}
          onOrdered={ordered}
        />
      )}
      <p className="center muted">
        This is the embeddable "Latch Checkout" experience — any merchant adds it with{" "}
        <code>@latch-labs/checkout</code>. Funds sit under split control: the merchant can't take them early,
        the buyer can't claw them back after delivery. See the <a href="#/orbit">other demo merchant</a> running on
        the same package.
      </p>
    </div>
  );
}

// ------------------------------------------------------------------ sheet

function CheckoutSheet({
  product,
  pending: resumable,
  onClose,
  onOrdered,
}: {
  product: Product;
  pending: PendingOrder | null;
  onClose: () => void;
  onOrdered: () => void;
}) {
  const { connection } = useConnection();
  const { publicKey } = useWallet();
  const connected = useLatch();
  const [mode, setMode] = useState<"options" | "card" | "processing" | "done">("options");
  const [agree, setAgree] = useState(!!resumable);
  const [log, setLog] = useState<string[]>([]);
  const [error, setError] = useState("");
  const [dealAddr, setDealAddr] = useState("");
  const [canResume, setCanResume] = useState(!!resumable);

  const push = (m: string) => setLog((l) => [...l, m]);

  async function drive(pending: PendingOrder, client: LatchClient, kp?: Keypair) {
    setMode("processing");
    setError("");
    try {
      savePending(pending);
      await advanceCheckout(connection, pending, client, kp, push);
      saveOrder({ deal: pending.deal, subject: pending.subject, guestSecret: pending.guestSecret });
      savePending(null);
      setDealAddr(pending.deal);
      setMode("done");
      onOrdered();
    } catch (e: any) {
      setError(e?.error?.errorMessage ?? e?.message ?? String(e));
      setCanResume(true);
      setMode("options");
    }
  }

  const resumeNow = () => {
    const pending = loadPending();
    if (!pending) return;
    try {
      const { client, kp } = buyerClientFor(connection, pending, connected);
      drive(pending, client, kp);
    } catch (e: any) {
      setError(e.message);
    }
  };

  const payWithWallet = () => {
    if (!connected || !publicKey) {
      setError("Connect a wallet first (top right) — or use the card option for guest checkout.");
      return;
    }
    drive(newPending(connection, product, publicKey), connected);
  };

  const payWithCard = () => {
    // Guest checkout: an invisible wallet is created for the buyer. On
    // mainnet this is where a licensed onramp partner converts the card
    // charge to stablecoins; on devnet we simulate it with test dollars.
    const kp = Keypair.generate();
    const provider = new AnchorProvider(connection, keypairWallet(kp) as any, { commitment: "confirmed" });
    drive(newPending(connection, product, kp.publicKey, kp), LatchClient.fromProvider(provider), kp);
  };

  const subjectFor = dealAddr
    ? loadOrders().find((o) => o.deal === dealAddr)?.subject ?? ""
    : "";

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
              🛡 <b>Protected payment.</b> Your payment is held so that neither you nor the merchant can take it
              alone. It releases when you confirm delivery — or after 7 days unless you open a dispute; a dispute
              pauses the clock, and then it moves only when you both agree. Every signature is recorded on-chain.
            </div>
          </>
        )}

        {mode === "options" && (
          <>
            {canResume && loadPending() && (
              <button className="primary wide" onClick={resumeNow}>
                ▶ Resume interrupted checkout
              </button>
            )}
            <label className="consent">
              <input type="checkbox" checked={agree} onChange={(e) => setAgree(e.target.checked)} /> I agree to the
              purchase agreement and consent to sign electronically. (The full agreement is hashed
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
              card buys stablecoins that go straight into the deal's on-chain vault. Here it delivers test dollars.
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
            <p className="muted center">
              Guest checkout — a wallet is created for you invisibly and kept in this browser's storage. Clearing
              site data loses access to it (demo tokens only). No extension needed.
            </p>
          </>
        )}

        {mode === "processing" && (
          <div className="processing">
            {log.map((l, i) => (
              <div key={i}>{l}</div>
            ))}
            <div className="spinline">⏳ working on devnet… (interruptions are safe — checkout resumes)</div>
          </div>
        )}

        {mode === "done" && (
          <div className="center">
            <h2>🎉 Order placed</h2>
            <p>
              <b>${product.price}.00</b> is held under split control for “{product.name}.”
              The merchant has countersigned; the deal is active on Solana devnet.
            </p>
            <p>
              <a className="btnlike" href={`#/deal/${dealAddr}?s=${encodeURIComponent(subjectFor)}`}>View deal</a>{" "}
              <a className="btnlike gold" href={`#/cert/${dealAddr}?s=${encodeURIComponent(subjectFor)}`}>Certificate</a>
            </p>
            <button className="wide" onClick={onClose}>Back to the store</button>
          </div>
        )}
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ orders

function Orders({ orders }: { orders: Order[] }) {
  const { connection } = useConnection();
  const connected = useLatch();
  const viewer = useLatchOrReadonly();
  const [states, setStates] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState("");

  const load = async () => {
    const out: Record<string, string> = {};
    for (const o of orders) {
      try {
        out[o.deal] = (await viewer.fetchDeal(new PublicKey(o.deal))).state;
      } catch {
        out[o.deal] = "?";
      }
    }
    setStates(out);
  };
  useEffect(() => {
    load();
  }, [orders.length, viewer]);

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
      const m = merchantClient(connection);
      const d = await withRetry("deal lookup", () => m.fetchDeal(deal));
      if (!(d.account.milestones[0].approvals & 1)) {
        await withRetry("buyer approval", () => buyerClient.approveMilestone(deal, 0).rpc());
      }
      if (!(d.account.milestones[0].approvals & 2)) {
        await withRetry("merchant approval", () => m.approveMilestone(deal, 0).rpc());
      }
      const merchantAta = getAssociatedTokenAddressSync(d.account.mint, MERCHANT_PUBKEY, true, d.account.tokenProgram);
      await withRetry("release", async () =>
        (await m.releaseMilestone(deal, 0, merchantAta))
          .preInstructions([
            createAssociatedTokenAccountIdempotentInstruction(MERCHANT_PUBKEY, merchantAta, MERCHANT_PUBKEY, d.account.mint, d.account.tokenProgram),
          ])
          .rpc()
      );
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
      <p className="muted">
        Orders (and guest wallets) live in this browser's storage — clearing site data loses them. Demo tokens only.
      </p>
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

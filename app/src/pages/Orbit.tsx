/**
 * Second demo merchant — "Orbit Supply Co." — kept intentionally tiny to
 * show the real cost of adopting Latch Checkout: the escrow integration is
 * the ~15 lines inside `buy()` below. Different merchant, different brand,
 * same @latch-labs/checkout package as Walnut & Oak.
 */
import { Keypair } from "@solana/web3.js";
import { useConnection } from "@solana/wallet-adapter-react";
import { useState } from "react";
import { LatchCheckout, LocalMerchantAdapter } from "@latch-labs/checkout";
import { LDD_DECIMALS, LDD_MINT } from "../lib/ldd";
import { simulatedCard } from "../lib/checkout";

// Orbit's demo keypair — devnet play money only, deliberately public.
const ORBIT_SECRET = new Uint8Array([
  104, 217, 107, 45, 245, 180, 2, 42, 173, 87, 70, 198, 132, 253, 44, 58, 55,
  140, 117, 10, 240, 9, 103, 34, 40, 165, 9, 15, 213, 87, 162, 78, 50, 145,
  227, 0, 188, 255, 104, 149, 145, 9, 164, 21, 190, 98, 171, 117, 150, 137,
  137, 51, 158, 138, 64, 121, 126, 204, 43, 196, 134, 131, 138, 224,
]);

const ITEM = { id: "fieldkit", name: "Expedition field kit" };
const PRICE = 60;

export default function Orbit() {
  const { connection } = useConnection();
  const [log, setLog] = useState<string[]>([]);
  const [deal, setDeal] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  async function buy() {
    setBusy(true);
    setError("");
    setLog([]);
    try {
      // --- the entire Latch integration -----------------------------------
      const checkout = new LatchCheckout({
        connection,
        mint: LDD_MINT,
        decimals: LDD_DECIMALS,
        merchant: new LocalMerchantAdapter(connection, Keypair.fromSecretKey(ORBIT_SECRET)),
        funding: simulatedCard,
      });
      const session = checkout.createSession({
        item: ITEM,
        price: PRICE,
        subject: `Purchase of: ${ITEM.name} — from Orbit Supply Co.`,
        guest: Keypair.generate(),
      });
      localStorage.setItem("orbit-pending", JSON.stringify(session.state));
      await session.advance(null, { onProgress: (m) => setLog((l) => [...l, m]) });
      // ---------------------------------------------------------------------
      localStorage.removeItem("orbit-pending");
      setDeal(session.deal.toBase58());
    } catch (e: any) {
      setError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  const subject = encodeURIComponent(`Purchase of: ${ITEM.name} — from Orbit Supply Co.`);
  return (
    <div className="store orbit">
      <div className="store-hero orbit-hero">
        <h1>🛰 Orbit Supply Co.</h1>
        <p>Gear for people who read the manual. A different merchant — the same protected-payment rails.</p>
        <p className="muted">
          This whole storefront's Latch integration is ~15 lines of{" "}
          <a href="https://github.com/latch-labs-llc/latch/blob/main/app/src/pages/Orbit.tsx" target="_blank" rel="noreferrer">
            code
          </a>{" "}
          using <code>@latch-labs/checkout</code>.
        </p>
      </div>

      <div className="products">
        <div className="product">
          <div className="pimg">🎒</div>
          <h3>{ITEM.name}</h3>
          <p className="muted">Headlamp, multitool, dry bag, resolve.</p>
          <div className="row spread">
            <b>${PRICE}.00</b>
            <button className="latchpay" onClick={buy} disabled={busy}>
              {busy ? "Processing…" : "💳 Buy protected"}
            </button>
          </div>
        </div>
      </div>

      {log.length > 0 && !deal && (
        <div className="card">
          {log.map((m, i) => (
            <div key={i} className="muted mono">{m}</div>
          ))}
        </div>
      )}
      {error && <div className="card error">{error}</div>}
      {deal && (
        <div className="card">
          ✅ <b>Order placed.</b> Payment is held under split control.{" "}
          <a href={`#/deal/${deal}?s=${subject}`}>View the deal</a> ·{" "}
          <a href={`#/cert/${deal}?s=${subject}`}>certificate</a>
        </div>
      )}
      <p className="center muted">
        <a href="#/store">← Walnut &amp; Oak Furniture Co.</a> — the other demo merchant on the same package.
      </p>
    </div>
  );
}

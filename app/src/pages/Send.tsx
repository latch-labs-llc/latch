/**
 * Send — protection is the dial. One form, one spectrum: instant transfer
 * at zero (plain SPL, no Latch — the familiar baseline), protected send
 * (TimeoutRelease + 7-day window preset over createDeal), custom (the full
 * deal form). The dial maps 1:1 onto formation parameters that already
 * exist; this page is presets, not new primitives.
 */
import { BN, DeadlockRule, RiskFlags, TimerMode, dealPda, LATCH_PROGRAM_ID } from "@latch-labs/sdk";
import {
  createAssociatedTokenAccountIdempotentInstruction,
  createTransferCheckedInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { PublicKey, Transaction } from "@solana/web3.js";
import { useState } from "react";
import { buildAgreement, sha256 } from "../lib/agreement";
import { LDD_DECIMALS, LDD_MINT } from "../lib/ldd";
import { short, useLatch } from "../lib/useLatch";

type Dial = "instant" | "protected";

export default function Send() {
  const { connection } = useConnection();
  const { publicKey, sendTransaction } = useWallet();
  const latch = useLatch();

  const [dial, setDial] = useState<Dial>("protected");
  const [to, setTo] = useState("");
  const [amount, setAmount] = useState("25");
  const [memo, setMemo] = useState("Payment for the thing we agreed on");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [done, setDone] = useState<{ kind: Dial; sig?: string; deal?: string; subject?: string } | null>(null);

  const run = async () => {
    setBusy(true);
    setError("");
    setDone(null);
    try {
      const recipient = new PublicKey(to.trim());
      const raw = BigInt(Math.round(parseFloat(amount) * 10 ** LDD_DECIMALS));
      if (raw <= 0n) throw new Error("amount must be positive");
      if (!publicKey) throw new Error("connect a wallet first (Burner Wallet works)");

      if (dial === "instant") {
        const fromAta = getAssociatedTokenAddressSync(LDD_MINT, publicKey, true);
        const toAta = getAssociatedTokenAddressSync(LDD_MINT, recipient, true);
        const tx = new Transaction().add(
          createAssociatedTokenAccountIdempotentInstruction(publicKey, toAta, recipient, LDD_MINT),
          createTransferCheckedInstruction(fromAta, LDD_MINT, toAta, publicKey, raw, LDD_DECIMALS)
        );
        const sig = await sendTransaction(tx, connection);
        await connection.confirmTransaction(sig, "confirmed");
        setDone({ kind: "instant", sig });
      } else {
        if (!latch) throw new Error("connect a wallet first");
        const dealId = Math.floor(Date.now() / 1000);
        const deal = dealPda(publicKey, dealId, LATCH_PROGRAM_ID);
        const subject = `Protected payment: ${memo.trim() || "as agreed"}`;
        const text = buildAgreement({
          deal,
          dealId: String(dealId),
          buyer: publicKey,
          seller: recipient,
          mint: LDD_MINT,
          decimals: LDD_DECIMALS,
          milestoneAmounts: [raw],
          ruleName: "TimeoutRelease",
          timerModeName: "FromActivation",
          timeoutSecs: String(7 * 86400),
          recoverySigners: [publicKey, recipient],
          recoveryThreshold: 2,
          recoveryDelaySecs: "259200",
          subject,
        });
        const termsHash = await sha256(text);
        await latch
          .createDeal({
            dealId,
            parties: [publicKey, recipient],
            payerIdx: 0,
            payeeIdx: 1,
            approvalThreshold: 2,
            termsHash,
            milestoneAmounts: [new BN(raw.toString())],
            deadlockRule: DeadlockRule.TimeoutRelease,
            timerMode: TimerMode.FromActivation,
            timeoutSecs: new BN(7 * 86400),
            recoverySigners: [publicKey, recipient],
            recoveryThreshold: 2,
            recoveryDelaySecs: new BN(259200),
            acceptedRiskFlags: RiskFlags.ALL,
            mint: LDD_MINT,
          })
          .rpc();
        await latch.signTerms(deal, true).rpc();
        setDone({ kind: "protected", deal: deal.toBase58(), subject });
      }
    } catch (e: any) {
      setError(e?.error?.errorMessage ?? e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  };

  const shareLink = done?.deal
    ? `${window.location.origin}${window.location.pathname}#/deal/${done.deal}?s=${encodeURIComponent(done.subject ?? "")}`
    : "";

  return (
    <>
      <div className="card center">
        <h1 style={{ marginBottom: 6 }}>Send money. Protection is the dial.</h1>
        <p className="muted" style={{ maxWidth: 560, margin: "0 auto" }}>
          Instant with people you trust. Held-until-delivery with strangers. Full milestones and arbiters when it's
          serious — all the same rails underneath.
        </p>
      </div>

      <div className="card">
        <div className="dial">
          <button className={dial === "instant" ? "on" : ""} onClick={() => setDial("instant")}>
            ⚡ Instant
            <span>Plain transfer, final immediately. For people you trust.</span>
          </button>
          <button className={dial === "protected" ? "on" : ""} onClick={() => setDial("protected")}>
            🛡 Protected
            <span>Held until you confirm — or 7 days after you both start. A dispute pauses the clock; then it moves only when you both agree.</span>
          </button>
          <a className="dialmore" href="#/">
            ⚙️ Custom
            <span>Milestones, arbiters, timers — the full deal form.</span>
          </a>
        </div>

        <label>
          Recipient wallet
          <input value={to} onChange={(e) => setTo(e.target.value)} placeholder="base58 address" />
        </label>
        <div className="row">
          <label style={{ flex: 1 }}>
            Amount (LDD)
            <input value={amount} onChange={(e) => setAmount(e.target.value)} type="number" min="0" />
          </label>
          {dial === "protected" && (
            <label style={{ flex: 2 }}>
              What is this payment for?
              <input value={memo} onChange={(e) => setMemo(e.target.value)} maxLength={90} />
            </label>
          )}
        </div>
        {dial === "protected" && (
          <p className="muted">
            This creates the deal and signs your side. Send the link to the recipient — they countersign the
            hashed agreement, then you fund it. Every signature lands on-chain; the order ends with a
            court-readable certificate.
          </p>
        )}
        {dial === "instant" && (
          <p className="muted">
            ⚠ Instant means instant: this is a plain token transfer with no protection, no record beyond the
            transfer, and no undo. That's the point of the dial.
          </p>
        )}
        <button className="primary wide" disabled={busy || !to || !amount} onClick={run}>
          {busy ? "Sending…" : dial === "instant" ? `Send ${amount || "0"} LDD instantly` : `Create protected payment of ${amount || "0"} LDD`}
        </button>
        {!publicKey && <p className="muted center" style={{ marginTop: 8 }}>Connect a wallet above — Burner Wallet works for trying it.</p>}
        {error && <div className="error">{error}</div>}
      </div>

      {done?.kind === "instant" && (
        <div className="ok">
          ⚡ Sent — final.{" "}
          <a href={`https://explorer.solana.com/tx/${done.sig}?cluster=devnet`} target="_blank" rel="noreferrer">
            transaction →
          </a>
        </div>
      )}
      {done?.kind === "protected" && (
        <div className="card">
          <h3>🛡 Protected payment created &amp; signed by you</h3>
          <p className="muted">
            Deal <code>{short(done.deal!, 6)}</code>. Send the recipient this link to countersign; you'll fund it
            once they have.
          </p>
          <div className="row">
            <input readOnly value={shareLink} onFocus={(e) => e.target.select()} />
            <button onClick={() => navigator.clipboard?.writeText(shareLink)}>Copy</button>
          </div>
          <p className="muted" style={{ marginTop: 8 }}>
            <a href={`#/deal/${done.deal}?s=${encodeURIComponent(done.subject ?? "")}`}>Open the deal page →</a>
          </p>
        </div>
      )}
    </>
  );
}

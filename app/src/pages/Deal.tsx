import { ACTIVATION_WINDOW_SECS, BN, DealInfo, enumName } from "@latch-labs/sdk";
import {
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { PublicKey } from "@solana/web3.js";
import { useEffect, useMemo, useState } from "react";
import { buildAgreement, hex, sha256 } from "../lib/agreement";
import { LDD_DECIMALS, LDD_MINT } from "../lib/ldd";
import { short, useLatchOrReadonly } from "../lib/useLatch";

function ui(raw: BN | bigint | string, decimals: number): string {
  const s = BigInt(raw.toString()).toString().padStart(decimals + 1, "0");
  const whole = s.slice(0, -decimals);
  const frac = s.slice(-decimals).replace(/0+$/, "");
  return frac ? `${whole}.${frac}` : whole;
}

function bit(mask: number, i: number): boolean {
  return (mask & (1 << i)) !== 0;
}

export default function Deal({ address, subject }: { address: string; subject: string }) {
  const { connection } = useConnection();
  const { publicKey } = useWallet();
  const latch = useLatchOrReadonly();
  const deal = useMemo(() => new PublicKey(address), [address]);

  const [info, setInfo] = useState<DealInfo | null>(null);
  const [vaultUi, setVaultUi] = useState("0");
  const [decimals, setDecimals] = useState(LDD_DECIMALS);
  const [agreement, setAgreement] = useState("");
  const [hashOk, setHashOk] = useState<boolean | null>(null);
  const [consent, setConsent] = useState(false);
  const [settleAmt, setSettleAmt] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [notFound, setNotFound] = useState(false);

  const refresh = async () => {
    if (!latch) return;
    try {
      const d = await latch.fetchDeal(deal);
      setInfo(d);
      setNotFound(false);
      try {
        const b = await connection.getTokenAccountBalance(d.vault);
        setVaultUi(b.value.uiAmountString ?? "0");
        setDecimals(b.value.decimals);
      } catch {
        setVaultUi("0");
      }
    } catch {
      setNotFound(true);
    }
  };

  useEffect(() => {
    refresh();
    const t = setInterval(refresh, 9000 + Math.random() * 3000);
    return () => clearInterval(t);
  }, [latch, address]);

  // Regenerate the agreement from on-chain fields + the link's subject, and
  // verify its SHA-256 against the on-chain terms hash.
  useEffect(() => {
    (async () => {
      if (!info) return;
      const a = info.account;
      const dec = a.mint.equals(LDD_MINT) ? LDD_DECIMALS : decimals;
      const text = buildAgreement({
        deal,
        dealId: a.dealId.toString(),
        buyer: a.parties[a.payerIdx],
        seller: a.parties[a.payeeIdx],
        mint: a.mint,
        decimals: dec,
        milestoneAmounts: a.milestones
          .slice(0, a.numMilestones)
          .map((m: any) => BigInt(m.amount.toString())),
        ruleName: enumName(a.deadlockRule as object),
        timerModeName: enumName(a.timerMode as object),
        timeoutSecs: a.timeoutSecs.toString(),
        recoverySigners: a.recoverySigners.slice(0, a.numRecovery),
        recoveryThreshold: a.recoveryThreshold,
        recoveryDelaySecs: a.recoveryDelaySecs.toString(),
        subject,
      });
      setAgreement(text);
      const digest = await sha256(text);
      setHashOk(hex(digest) === hex(a.termsHash as number[]));
    })();
  }, [info?.account.termsHash?.toString(), subject, decimals]);

  if (notFound) return <div className="card">Deal not found on devnet: <code>{address}</code></div>;
  if (!info || !latch) return <div className="card center">Loading deal…</div>;

  const a = info.account;
  const me = publicKey ?? PublicKey.default; // read-only viewer when no wallet
  const myIdx = a.parties.slice(0, a.numParties).findIndex((p: PublicKey) => p.equals(me));
  const isParty = myIdx >= 0;
  const isPayer = myIdx === a.payerIdx;
  const isPayee = myIdx === a.payeeIdx;
  const buyer = a.parties[a.payerIdx] as PublicKey;
  const seller = a.parties[a.payeeIdx] as PublicKey;
  const recIdx = a.recoverySigners
    .slice(0, a.numRecovery)
    .findIndex((p: PublicKey) => p.equals(me));
  const tp = a.tokenProgram as PublicKey;
  const buyerAta = getAssociatedTokenAddressSync(a.mint, buyer, true, tp);
  const sellerAta = getAssociatedTokenAddressSync(a.mint, seller, true, tp);
  const state = info.state;
  const shareLink = `${window.location.origin}${window.location.pathname}#/deal/${address}${subject ? `?s=${encodeURIComponent(subject)}` : ""}`;
  const certLink = `#/cert/${address}${subject ? `?s=${encodeURIComponent(subject)}` : ""}`;

  const run = (label: string, fn: () => Promise<any>) => async () => {
    setBusy(label);
    setError("");
    try {
      await fn();
      await refresh();
    } catch (e: any) {
      setError(e?.error?.errorMessage ?? e?.message ?? String(e));
    } finally {
      setBusy(null);
    }
  };

  const ataIx = (owner: PublicKey, ata: PublicKey) =>
    createAssociatedTokenAccountIdempotentInstruction(me, ata, owner, a.mint, tp);

  const remaining = BigInt(a.totalAmount.toString()) - BigInt(a.deposited.toString());
  const lapsesAt = Number(a.fundedAt.toString()) + ACTIVATION_WINDOW_SECS;
  const lapsed = Date.now() / 1000 >= lapsesAt;
  const activationTimer = enumName(a.timerMode as object) === "FromActivation";
  const policy = enumName(a.disputePolicy as object);
  const windowDays = Number(a.disputeWindowSecs) / 86400;
  const windowText = `${windowDays} day${windowDays === 1 ? "" : "s"}`;
  const disputedSecs =
    Number(a.disputedSecs) + (state === "Deadlocked" ? Math.max(0, Date.now() / 1000 - Number(a.deadlockRaisedAt)) : 0);
  const disputeExpired = activationTimer && policy !== "NeverExpire" && disputedSecs >= Number(a.disputeWindowSecs);
  const policyLabel =
    policy === "NeverExpire"
      ? "stay locked until both sides agree"
      : policy === "ResumeRule"
        ? `after ${windowText} of disputes the clock resumes`
        : policy === "Split"
          ? `after ${windowText} of disputes the funds split`
          : `after ${windowText} of disputes the buyer is refunded`;

  return (
    <>
      <div className="card">
        <div className="row spread">
          <h2 style={{ margin: 0 }}>
            Deal <code>{short(address, 6)}</code> <span className={`badge s-${state}`}>{state}</span>
          </h2>
          <div>
            <button onClick={() => navigator.clipboard.writeText(shareLink)}>Copy share link</button>{" "}
            <a className="btnlike" href={certLink}>Certificate</a>
          </div>
        </div>
        {subject && <p className="subject">“{subject}”</p>}
        <div className="row balances">
          <div>Escrow vault: <b>{vaultUi}</b></div>
          <div>Deposited: <b>{ui(a.deposited, decimals)}</b> / {ui(a.totalAmount, decimals)}</div>
          <div>Released: <b>{ui(a.releasedTotal, decimals)}</b></div>
        </div>
        <table className="parties">
          <tbody>
            {a.parties.slice(0, a.numParties).map((p: PublicKey, i: number) => (
              <tr key={i} className={p.equals(me) ? "me" : ""}>
                <td>{i === a.payerIdx ? "Buyer" : i === a.payeeIdx ? "Seller" : `Party ${i}`}{p.equals(me) ? " (you)" : ""}</td>
                <td><code>{short(p.toBase58(), 6)}</code></td>
                <td>{bit(a.signed, i) ? "✍️ signed" : "not signed"}</td>
                <td>{bit(a.ready, i) ? "✅ ready" : ""}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="muted">
          Rule: <b>{enumName(a.deadlockRule as object)}</b> · timer {enumName(a.timerMode as object)} ·{" "}
          timeout {a.timeoutSecs.toString()}s · recovery {a.recoveryThreshold}-of-{a.numRecovery} (notice{" "}
          {a.recoveryDelaySecs.toString()}s){activationTimer && <> · unresolved disputes {policyLabel}</>}
        </p>
      </div>

      {state === "Draft" && (
        <div className="card">
          <h3>The agreement</h3>
          {hashOk === true && <div className="ok">✓ Regenerated text matches the on-chain SHA-256 digest.</div>}
          {hashOk === false && (
            <div className="error">
              ✗ Text does not match the on-chain digest — the share link's subject line may be missing or altered.
              Do not sign unless the sender re-shares the correct link.
            </div>
          )}
          <pre className="agreement">{agreement}</pre>
          {isParty && !bit(a.signed, myIdx) && (
            <>
              {enumName(a.deadlockRule as object) === "TrueDeadlock" && (
                <div className="error">
                  <b>⚠ TRUE-DEADLOCK DEAL — read before signing.</b> This deal has <b>no timeout and no
                  arbiter</b>. If the parties never agree, the escrowed funds stay locked <b>forever</b>; only
                  mutual sign-off or the recovery signers ({a.recoveryThreshold}-of-{a.numRecovery}, after the
                  on-chain notice period) can ever move them. Your signature records your explicit, informed
                  consent to that rule.
                </div>
              )}
              <label className="consent">
                <input type="checkbox" checked={consent} onChange={(e) => setConsent(e.target.checked)} /> I have read
                the agreement and consent to sign and transact electronically. My wallet signature over the document
                digest constitutes my execution of this agreement.
              </label>
              <button
                className="primary"
                disabled={!!busy || !consent || hashOk === false}
                onClick={run("sign", () => latch.signTerms(deal, true).rpc())}
              >
                {busy === "sign" ? "Signing…" : "Sign the agreement"}
              </button>
            </>
          )}
          {isParty && bit(a.signed, myIdx) && <p>You have signed. Waiting for the other part{a.numParties > 2 ? "ies" : "y"}.</p>}
          {isParty && (
            <button disabled={!!busy} onClick={run("cancel", () => latch.cancelDraft(deal).rpc())}>
              Cancel draft
            </button>
          )}
        </div>
      )}

      {state === "Signed" && (
        <div className="card">
          <h3>Fund the escrow</h3>
          {isPayer ? (
            <button
              className="primary"
              disabled={!!busy}
              onClick={run("deposit", async () =>
                (await latch.deposit(deal, new BN(remaining.toString()), buyerAta)).rpc()
              )}
            >
              {busy === "deposit" ? "Depositing…" : `Deposit ${ui(remaining, decimals)} into escrow`}
            </button>
          ) : (
            <p>Waiting for the buyer to deposit {ui(remaining, decimals)}.</p>
          )}
          {isParty && (
            <button
              disabled={!!busy}
              onClick={run("cancelSign", async () => (await latch.cancelSign(deal, buyerAta)).preInstructions([ataIx(buyer, buyerAta)]).rpc())}
            >
              Sign mutual cancellation {bit(a.cancelApprovals, myIdx) ? "(you have signed)" : ""}
            </button>
          )}
        </div>
      )}

      {state === "Funded" && (
        <div className="card">
          <h3>Confirm ready to perform</h3>
          {isParty && !bit(a.ready, myIdx) ? (
            <button className="primary" disabled={!!busy} onClick={run("ready", () => latch.confirmReady(deal).rpc())}>
              {busy === "ready" ? "Confirming…" : "I'm ready — begin the deal"}
            </button>
          ) : (
            <p>Waiting for all parties to confirm.</p>
          )}
          {isParty && (
            <button
              disabled={!!busy}
              onClick={run("cancelSign", async () => (await latch.cancelSign(deal, buyerAta)).preInstructions([ataIx(buyer, buyerAta)]).rpc())}
            >
              Sign mutual cancellation {bit(a.cancelApprovals, myIdx) ? "(you have signed)" : ""}
            </button>
          )}
          {lapsed ? (
            <div style={{ marginTop: 12 }}>
              <p>
                Not every party confirmed within 3 days of funding, so the buyer's full deposit can now be returned.
                Anyone can submit this; the money can only go back to the buyer.
              </p>
              {publicKey ? (
                <button
                  className="primary"
                  disabled={!!busy}
                  onClick={run("lapse", async () => (await latch.refundUnactivated(deal, buyerAta)).preInstructions([ataIx(buyer, buyerAta)]).rpc())}
                >
                  {busy === "lapse" ? "Returning…" : "Return the deposit to the buyer"}
                </button>
              ) : (
                <p className="muted">Connect any wallet to submit the return.</p>
              )}
            </div>
          ) : (
            <p className="muted" style={{ marginTop: 12 }}>
              If not every party confirms by {new Date(lapsesAt * 1000).toLocaleString()}, the buyer's full deposit
              can be returned to them — no one can hold it.
            </p>
          )}
        </div>
      )}

      {state === "Active" && (
        <div className="card">
          <h3>Milestones</h3>
          {a.milestones.slice(0, a.numMilestones).map((m: any, i: number) => {
            const approvals = (m.approvals as number).toString(2).split("1").length - 1;
            const releasable =
              !m.released &&
              approvals >= a.approvalThreshold &&
              a.milestones.slice(0, i).every((p: any) => p.released);
            return (
              <div className="milestone" key={i}>
                <div>
                  #{i + 1} — {ui(m.amount, decimals)} {m.released ? "· RELEASED ✅" : `· approvals ${approvals}/${a.approvalThreshold}`}
                </div>
                {!m.released && isParty && !bit(m.approvals, myIdx) && (
                  <button disabled={!!busy} onClick={run(`approve${i}`, () => latch.approveMilestone(deal, i).rpc())}>
                    {busy === `approve${i}` ? "…" : "Approve"}
                  </button>
                )}
                {releasable && (
                  <button
                    className="primary"
                    disabled={!!busy}
                    onClick={run(`release${i}`, async () =>
                      (await latch.releaseMilestone(deal, i, sellerAta)).preInstructions([ataIx(seller, sellerAta)]).rpc()
                    )}
                  >
                    {busy === `release${i}` ? "…" : "Release to seller"}
                  </button>
                )}
              </div>
            );
          })}
          {isParty && (
            <button className="danger" disabled={!!busy} onClick={run("dispute", () => latch.raiseDeadlock(deal).rpc())}>
              Raise a dispute
            </button>
          )}
        </div>
      )}

      {state === "Deadlocked" && (
        <div className="card">
          <h3>⚖️ Dispute open</h3>
          <p className="muted">
            Raised by <code>{short((a.deadlockRaisedBy as PublicKey).toBase58(), 6)}</code>. The pre-agreed rule (
            {enumName(a.deadlockRule as object)}) governs; the parties can also settle jointly at any time.
          </p>
          {activationTimer && policy === "NeverExpire" && (
            <p className="muted">
              The clock is paused. The funds stay put until you both settle, your recovery signers act, or the seller
              refunds the buyer.
            </p>
          )}
          {activationTimer && policy !== "NeverExpire" && (
            <p className="muted">
              Dispute time used: {(disputedSecs / 86400).toFixed(1)} of {windowText} — {policyLabel}.
              {disputeExpired && policy !== "ResumeRule" && " The window has passed: anyone can execute the resolution below."}
            </p>
          )}
          {disputeExpired && policy === "ResumeRule" && publicKey && (
            <button className="primary" disabled={!!busy} onClick={run("lift", () => latch.withdrawDeadlock(deal).rpc())}>
              {busy === "lift" ? "Lifting…" : "Lift the expired dispute (anyone can) — the clock resumes"}
            </button>
          )}
          {a.proposalActive && (
            <p>
              Current settlement proposal: <b>{ui(a.proposedToPayee, decimals)}</b> to the seller, remainder to the
              buyer. Signed by {((a.resolutionApprovals as number).toString(2).split("1").length - 1)} of {a.numParties}.
            </p>
          )}
          {Boolean(a.tieBreakerDecided) && (
            <p>
              ⚖️ The pre-agreed arbiter has ruled: <b>{ui(a.tieBreakerAmount, decimals)}</b> to the seller, remainder
              to the buyer. Anyone can execute it — unless all parties jointly settle on different terms first.
            </p>
          )}
          {isParty && (
            <div className="row">
              <input
                placeholder="amount to seller"
                value={settleAmt}
                type="number"
                onChange={(e) => setSettleAmt(e.target.value)}
              />
              <button
                disabled={!!busy || !settleAmt}
                onClick={run("settle", () =>
                  latch
                    .resolutionSign(deal, new BN(BigInt(Math.round(parseFloat(settleAmt) * 10 ** decimals)).toString()))
                    .rpc()
                )}
              >
                {busy === "settle" ? "…" : "Sign settlement"}
              </button>
            </div>
          )}
          <div className="row">
            <button
              className="primary"
              disabled={!!busy}
              onClick={run("resolve", async () =>
                (await latch.resolve(deal, buyerAta, sellerAta))
                  .preInstructions([ataIx(buyer, buyerAta), ataIx(seller, sellerAta)])
                  .rpc()
              )}
            >
              {busy === "resolve" ? "…" : "Execute resolution (anyone can)"}
            </button>
            {me.equals(a.deadlockRaisedBy as PublicKey) && (
              <button disabled={!!busy} onClick={run("withdraw", () => latch.withdrawDeadlock(deal).rpc())}>
                Withdraw my dispute
              </button>
            )}
          </div>
          {recIdx >= 0 && (
            <div className="recovery">
              <h4>Recovery signer actions</h4>
              <div className="row">
                <input placeholder="amount to seller" id="recAmt" type="number" onChange={(e) => setSettleAmt(e.target.value)} />
                <button
                  disabled={!!busy}
                  onClick={run("recSign", () =>
                    latch
                      .recoverySign(deal, new BN(BigInt(Math.round(parseFloat(settleAmt || "0") * 10 ** decimals)).toString()))
                      .rpc()
                  )}
                >
                  Recovery-sign
                </button>
                <button
                  disabled={!!busy}
                  onClick={run("recExec", async () =>
                    (await latch.recoveryExecute(deal, buyerAta, sellerAta))
                      .preInstructions([ataIx(buyer, buyerAta), ataIx(seller, sellerAta)])
                      .rpc()
                  )}
                >
                  Execute recovery
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {isPayee &&
        (["Funded", "Active", "Deadlocked"].includes(state) ||
          (state === "Signed" && BigInt(a.deposited.toString()) > 0n)) && (
          <div className="card">
            <h3>Can't deliver?</h3>
            <p className="muted">
              As the seller you can return everything still held for this deal to the buyer, on your own — no
              dispute needed. This ends the deal and can't be undone.
            </p>
            <button
              disabled={!!busy}
              onClick={() => {
                if (!window.confirm("Return the buyer's full remaining payment and end this deal?")) return;
                run("payeeRefund", async () => (await latch.refundByPayee(deal, buyerAta)).preInstructions([ataIx(buyer, buyerAta)]).rpc())();
              }}
            >
              {busy === "payeeRefund" ? "Refunding…" : "Refund the buyer"}
            </button>
          </div>
        )}

      {(state === "Completed" || state === "Cancelled") && (
        <div className="card center">
          <h3>{state === "Completed" ? "🎉 Deal completed" : "Deal cancelled"}</h3>
          <p>
            Every signature and payout is on the public chain.{" "}
            <a className="btnlike gold" href={certLink}>Open the Signature &amp; Escrow Certificate</a>
          </p>
        </div>
      )}

      {error && <div className="card error">{error}</div>}
      {!publicKey && (
        <p className="muted center">Viewing read-only — connect a wallet (top right) to act on this deal.</p>
      )}
      {publicKey && !isParty && recIdx < 0 && state !== "Completed" && state !== "Cancelled" && (
        <p className="muted center">
          You are viewing as <code>{short(me.toBase58(), 6)}</code> — not a party to this deal. Release and resolve
          buttons still work for anyone once conditions are met (that's the point).
        </p>
      )}
    </>
  );
}

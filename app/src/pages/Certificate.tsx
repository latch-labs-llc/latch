import { DealInfo, enumName, fetchEventHistory } from "@latch-labs/sdk";
import { useConnection } from "@solana/wallet-adapter-react";
import { PublicKey } from "@solana/web3.js";
import { useEffect, useMemo, useState } from "react";
import { buildAgreement, hex, sha256 } from "../lib/agreement";
import { LDD_DECIMALS, LDD_MINT } from "../lib/ldd";
import { useLatchOrReadonly } from "../lib/useLatch";

function ts(t: number | null): string {
  return t ? new Date(t * 1000).toUTCString() : "—";
}

export default function Certificate({ address, subject }: { address: string; subject: string }) {
  const { connection } = useConnection();
  const latch = useLatchOrReadonly();
  const deal = useMemo(() => new PublicKey(address), [address]);
  const [info, setInfo] = useState<DealInfo | null>(null);
  const [events, setEvents] = useState<any[]>([]);
  const [agreement, setAgreement] = useState("");
  const [hashOk, setHashOk] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      if (!latch) return;
      setLoading(true);
      const d = await latch.fetchDeal(deal);
      setInfo(d);
      const a = d.account;
      const dec = a.mint.equals(LDD_MINT) ? LDD_DECIMALS : 6;
      const text = buildAgreement({
        deal,
        dealId: a.dealId.toString(),
        buyer: a.parties[a.payerIdx],
        seller: a.parties[a.payeeIdx],
        mint: a.mint,
        decimals: dec,
        milestoneAmounts: a.milestones.slice(0, a.numMilestones).map((m: any) => BigInt(m.amount.toString())),
        ruleName: enumName(a.deadlockRule as object),
        timerModeName: enumName(a.timerMode as object),
        timeoutSecs: a.timeoutSecs.toString(),
        recoverySigners: a.recoverySigners.slice(0, a.numRecovery),
        recoveryThreshold: a.recoveryThreshold,
        recoveryDelaySecs: a.recoveryDelaySecs.toString(),
        subject,
      });
      setAgreement(text);
      setHashOk(hex(await sha256(text)) === hex(a.termsHash as number[]));
      setEvents(await fetchEventHistory(latch.program, connection, deal));
      setLoading(false);
    })();
  }, [latch, address]);

  if (loading || !info) return <div className="card center">Reconstructing certificate from on-chain events…</div>;

  const a = info.account;
  return (
    <div className="cert">
      <h1>Signature &amp; Escrow Certificate</h1>
      <p className="demo">DEMONSTRATION — Solana devnet. No real funds or obligations.</p>
      <table className="meta">
        <tbody>
          <tr><td>Deal account</td><td><code>{address}</code></td></tr>
          <tr><td>State</td><td>{info.state}</td></tr>
          <tr><td>Document digest (SHA-256)</td><td><code>{hex(a.termsHash as number[])}</code></td></tr>
          <tr>
            <td>Digest verification</td>
            <td>{hashOk === true ? "✓ regenerated agreement text matches on-chain digest" : hashOk === false ? "✗ MISMATCH — subject line missing or altered in link" : "…"}</td>
          </tr>
          <tr><td>Generated</td><td>{new Date().toUTCString()}</td></tr>
        </tbody>
      </table>

      <h2>Execution record</h2>
      <table>
        <thead><tr><th>Party</th><th>Wallet</th><th>Signed (cluster time)</th></tr></thead>
        <tbody>
          {a.parties.slice(0, a.numParties).map((p: PublicKey, i: number) => (
            <tr key={i}>
              <td>{i === a.payerIdx ? "Buyer" : i === a.payeeIdx ? "Seller" : `Party ${i}`}</td>
              <td><code>{p.toBase58()}</code></td>
              <td>{a.partiesSignedAt[i].toString() !== "0" ? ts(Number(a.partiesSignedAt[i])) : "not signed"}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <h2>Event history ({events.length} events, from inner instructions)</h2>
      <table>
        <thead><tr><th>Seq</th><th>Event</th><th>Time</th><th>Transaction</th></tr></thead>
        <tbody>
          {events.map((e, i) => (
            <tr key={i}>
              <td>{e.data?.seq?.toString?.() ?? "—"}</td>
              <td>{e.name}</td>
              <td>{ts(e.blockTime)}</td>
              <td>
                <a href={`https://explorer.solana.com/tx/${e.signature}?cluster=devnet`} target="_blank" rel="noreferrer">
                  <code>{e.signature.slice(0, 16)}…</code>
                </a>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <h2>Independent verification</h2>
      <ol>
        <li>The agreement below is regenerated deterministically from on-chain fields plus the link's subject; hash it (SHA-256) and compare to the digest above.</li>
        <li>Open the deal account on the <a href={`https://explorer.solana.com/address/${address}?cluster=devnet`} target="_blank" rel="noreferrer">Solana Explorer</a> and confirm each transaction is signed by the listed wallet.</li>
        <li>No fact on this page is asserted by any company — everything is reconstructed from the public chain.</li>
      </ol>

      <h2>How this record was produced (methodology)</h2>
      <ol>
        <li>
          Every fact above was read from the Solana blockchain at deal account <code>{address}</code> via public
          RPC. Nothing comes from a private database or any company's records; the operator of this page holds no
          information a third party cannot retrieve independently.
        </li>
        <li>
          The execution record reflects the deal account's on-chain state. The event history is decoded from the
          inner instructions of the listed transactions; each row links to the public transaction that produced it.
        </li>
        <li>
          The agreement text in the appendix was regenerated deterministically (template v1) from the deal's
          on-chain fields plus the subject string carried in the link, and its SHA-256 digest was compared against
          the digest the parties signed. Result: {hashOk === true ? "match" : hashOk === false ? "MISMATCH" : "pending"}.
        </li>
        <li>
          The program that enforced these rules is open source (Apache-2.0). Its deployed bytecode can be
          independently reproduced from source and compared against the chain with <code>solana-verify</code>;
          the procedure is published in the repository README.
        </li>
      </ol>

      <h2>Declaration of the person producing this packet</h2>
      <p className="muted noprint">
        For use when this printout is tendered to a court, arbitrator, or auditor: the person who generated it
        completes this by hand. (Demonstration only on devnet — this instance creates no legal obligations.)
      </p>
      <div className="decl">
        <p>
          I, ______________________________________, state that on the date below I retrieved this record using the
          methodology described above, over public Solana RPC, and that this printout accurately reflects what was
          displayed; the verification in step 3 reported the result shown.
        </p>
        <p className="decl-lines">
          Signature: ______________________________________ &nbsp;&nbsp; Date: ______________________
        </p>
      </div>

      <h2>Appendix — agreement text</h2>
      <pre className="agreement">{agreement}</pre>
      <div className="noprint center">
        <button className="primary" onClick={() => window.print()}>Print / save evidence packet (PDF)</button>{" "}
        <a className="btnlike" href={`#/deal/${address}${subject ? `?s=${encodeURIComponent(subject)}` : ""}`}>Back to deal</a>
      </div>
    </div>
  );
}

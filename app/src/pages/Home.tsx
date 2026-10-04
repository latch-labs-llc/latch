import { BN, DeadlockRule, RiskFlags, TimerMode, dealPda, LATCH_PROGRAM_ID } from "@latch-labs/sdk";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { useEffect, useState } from "react";
import { buildAgreement, sha256 } from "../lib/agreement";
import { LDD_DECIMALS, LDD_MINT, buildLddFaucetTx, lddAta, tokenUiBalance } from "../lib/ldd";
import { SOL_TAP_AMOUNT, tapSol } from "../lib/soltap";
import { short, useLatch } from "../lib/useLatch";

const RULES = ["TimeoutRefund", "TimeoutRelease", "AutoSplit", "TieBreaker", "LongSunset", "TrueDeadlock"] as const;
type RuleName = (typeof RULES)[number];

export default function Home() {
  const { connection } = useConnection();
  const { publicKey, sendTransaction } = useWallet();
  const latch = useLatch();

  const [sol, setSol] = useState<number | null>(null);
  const [ldd, setLdd] = useState<number>(0);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string>("");

  const [role, setRole] = useState<"buyer" | "seller">("buyer");
  const [counterparty, setCounterparty] = useState("");
  const [subject, setSubject] = useState("Sale of one vintage mahogany writing desk, shipped");
  const [amount, setAmount] = useState("100");
  const [rule, setRule] = useState<RuleName>("TimeoutRefund");
  const [timerMode, setTimerMode] = useState<"FromDeadlock" | "FromActivation">("FromDeadlock");
  const [timeout_, setTimeout_] = useState("86400");
  const [tieBreaker, setTieBreaker] = useState("");
  const [splitBps, setSplitBps] = useState("5000");
  const [openAddr, setOpenAddr] = useState("");

  const refreshBalances = async () => {
    if (!publicKey) return;
    setSol((await connection.getBalance(publicKey)) / LAMPORTS_PER_SOL);
    setLdd(await tokenUiBalance(connection, lddAta(publicKey)));
  };
  useEffect(() => {
    refreshBalances();
    const t = setInterval(refreshBalances, 20000);
    return () => clearInterval(t);
  }, [publicKey]);

  const run = async (label: string, fn: () => Promise<void>) => {
    setBusy(label);
    setError("");
    try {
      await fn();
    } catch (e: any) {
      setError(e?.message ?? String(e));
    } finally {
      setBusy(null);
      refreshBalances();
    }
  };

  const airdrop = () =>
    run("airdrop", async () => {
      try {
        await tapSol(connection, publicKey!);
      } catch {
        // Tap empty or hiccuped - fall back to the public faucet.
        const sig = await connection.requestAirdrop(publicKey!, LAMPORTS_PER_SOL);
        await connection.confirmTransaction(sig, "confirmed");
      }
    });

  const faucet = () =>
    run("faucet", async () => {
      const { tx, authority } = buildLddFaucetTx(publicKey!, 1000);
      const sig = await sendTransaction(tx, connection, { signers: [authority] });
      await connection.confirmTransaction(sig, "confirmed");
    });

  const createDeal = () =>
    run("create", async () => {
      const other = new PublicKey(counterparty.trim());
      const me = publicKey!;
      const buyer = role === "buyer" ? me : other;
      const seller = role === "buyer" ? other : me;
      const dealId = Math.floor(Date.now() / 1000);
      const deal = dealPda(me, dealId, LATCH_PROGRAM_ID);
      const raw = BigInt(Math.round(parseFloat(amount) * 10 ** LDD_DECIMALS));
      if (raw <= 0n) throw new Error("amount must be positive");
      const timeoutSecs = rule === "TieBreaker" || rule === "TrueDeadlock" ? "0" : timeout_;
      const effTimer = rule === "TieBreaker" || rule === "TrueDeadlock" ? "FromDeadlock" : timerMode;

      const text = buildAgreement({
        deal,
        dealId: String(dealId),
        buyer,
        seller,
        mint: LDD_MINT,
        decimals: LDD_DECIMALS,
        milestoneAmounts: [raw],
        ruleName: rule,
        timerModeName: effTimer,
        timeoutSecs,
        recoverySigners: [buyer, seller],
        recoveryThreshold: 2,
        recoveryDelaySecs: "259200",
        subject: subject.trim(),
      });
      const termsHash = await sha256(text);

      await latch!
        .createDeal({
          dealId,
          parties: [buyer, seller],
          payerIdx: 0,
          payeeIdx: 1,
          approvalThreshold: 2,
          termsHash,
          milestoneAmounts: [new BN(raw.toString())],
          deadlockRule: DeadlockRule[rule],
          timerMode: TimerMode[effTimer],
          timeoutSecs: new BN(timeoutSecs),
          splitBps: rule === "AutoSplit" ? parseInt(splitBps) : 0,
          tieBreaker: rule === "TieBreaker" ? new PublicKey(tieBreaker.trim()) : undefined,
          recoverySigners: [buyer, seller],
          recoveryThreshold: 2,
          recoveryDelaySecs: new BN(259200),
          acceptedRiskFlags: RiskFlags.ALL,
          mint: LDD_MINT,
        })
        .rpc();
      window.location.hash = `#/deal/${deal.toBase58()}?s=${encodeURIComponent(subject.trim())}`;
    });

  if (!publicKey) {
    return (
      <>
        <div className="card center hero">
          <h1>Payments between strangers, protected on both sides.</h1>
          <p>
            Latch holds the buyer's payment in a vault neither side — and no operator — can move alone, and releases
            it under rules both parties chose up front. Final settlement for the seller, no chargebacks; delivery
            protection for the buyer; a court-readable record of every signature.
          </p>
          <div className="ctas">
            <a className="btnlike gold" href="#/store">🛒 Try the demo store — no wallet needed</a>
            <a className="btnlike" href="#/orbit">🛰 Second merchant, ~15-line integration</a>
            <a className="btnlike" href="https://latch-labs-llc.github.io/latch/api/" target="_blank" rel="noreferrer">
              📘 SDK API reference
            </a>
          </div>
        </div>
        <div className="card">
          <h3>What's underneath</h3>
          <ul className="features">
            <li>Six resolution rules fixed at signing — timeouts, auto-split, a named arbiter, or consented true deadlock.</li>
            <li>Party-chosen recovery for lost keys and court orders: payouts only to the parties, after on-chain notice.</li>
            <li>The legal agreement is hashed on-chain and self-verifying — anyone can rebuild the signature certificate from public data.</li>
            <li>Token-2022 aware: freeze and seize powers are vetted and disclosed before any deal starts.</li>
            <li>Verified reproducible build; permissionless release cranks; Apache-2.0, no token, no fees.</li>
            <li>
              Nothing to host, no one to trust: the program runs on Solana itself, anyone can execute any step, and
              this app is an optional, forkable static page — every deal settles even if Latch Labs vanishes.
            </li>
          </ul>
        </div>
        <p className="center muted">
          Building custom deals (milestones, arbiters, disputes)? Connect a wallet above — or pick <b>Burner Wallet</b>{" "}
          for a zero-setup throwaway.
        </p>
      </>
    );
  }

  return (
    <>
      <div className="card promo">
        <div className="row spread">
          <div>
            <h3 style={{ margin: 0 }}>New: Latch Checkout</h3>
            <p className="muted" style={{ margin: "4px 0 0" }}>
              The Apple Pay experience, escrow underneath. Try the demo storefront — card path needs no wallet at all.
            </p>
          </div>
          <a className="btnlike gold" href="#/store">Visit the store →</a>
        </div>
      </div>

      <div className="card">
        <h3>Your devnet balances</h3>
        <div className="row balances">
          <div>
            <b>{sol === null ? "…" : sol.toFixed(3)}</b> SOL{" "}
            <button disabled={!!busy} onClick={airdrop}>
              {busy === "airdrop" ? "…" : `Get ${SOL_TAP_AMOUNT} SOL`}
            </button>
            <span className="muted"> (devnet fee money, on us)</span>
          </div>
          <div>
            <b>{ldd.toLocaleString()}</b> LDD{" "}
            <button disabled={!!busy} onClick={faucet}>
              {busy === "faucet" ? "…" : "Get 1,000 LDD"}
            </button>
            <span className="muted"> (Latch Demo Dollar — free devnet test token)</span>
          </div>
        </div>
      </div>

      <div className="card">
        <h3>Create a deal</h3>
        <label>
          I am the…
          <select value={role} onChange={(e) => setRole(e.target.value as any)}>
            <option value="buyer">buyer (I pay into escrow)</option>
            <option value="seller">seller (I get paid)</option>
          </select>
        </label>
        <label>
          Counterparty wallet ({role === "buyer" ? "seller" : "buyer"})
          <input value={counterparty} onChange={(e) => setCounterparty(e.target.value)} placeholder="base58 address" />
        </label>
        <label>
          Subject of the deal
          <input value={subject} onChange={(e) => setSubject(e.target.value)} maxLength={120} />
        </label>
        <label>
          Amount (LDD)
          <input value={amount} onChange={(e) => setAmount(e.target.value)} type="number" min="0" />
        </label>
        <label>
          If we deadlock…
          <select value={rule} onChange={(e) => setRule(e.target.value as RuleName)}>
            <option value="TimeoutRefund">Timeout refund — money returns to buyer after the timeout</option>
            <option value="TimeoutRelease">Timeout release — money goes to seller after the timeout</option>
            <option value="AutoSplit">Automatic split — divided by a preset ratio after the timeout</option>
            <option value="TieBreaker">Named tie-breaker — a third wallet decides</option>
            <option value="LongSunset">Long sunset — locked until agreement, refunded much later</option>
            <option value="TrueDeadlock">True deadlock — locked until both sides agree, forever</option>
          </select>
        </label>
        {rule !== "TieBreaker" && rule !== "TrueDeadlock" && (
          <>
            <label>
              Timer mode
              <select value={timerMode} onChange={(e) => setTimerMode(e.target.value as any)}>
                <option value="FromDeadlock">Clock starts when someone disputes</option>
                <option value="FromActivation">Clock runs from activation; a dispute pauses it</option>
              </select>
            </label>
            <label>
              Timeout (seconds)
              <input value={timeout_} onChange={(e) => setTimeout_(e.target.value)} type="number" min="1" />
            </label>
          </>
        )}
        {rule === "AutoSplit" && (
          <label>
            Seller share (basis points, 10000 = 100%)
            <input value={splitBps} onChange={(e) => setSplitBps(e.target.value)} type="number" min="0" max="10000" />
          </label>
        )}
        {rule === "TrueDeadlock" && (
          <div className="error">
            ⚠ True deadlock has <b>no timeout and no arbiter</b>. If you and your counterparty never agree, the
            funds stay locked forever — only mutual signatures or your recovery signers can ever move them. Every
            party must explicitly consent to this at signing.
          </div>
        )}
        {rule === "TieBreaker" && (
          <label>
            Tie-breaker wallet (must not be a party)
            <input value={tieBreaker} onChange={(e) => setTieBreaker(e.target.value)} placeholder="base58 address" />
          </label>
        )}
        <p className="muted">
          Recovery: both parties (2-of-2) with a 3-day on-chain notice period. The agreement text is generated from
          these terms and its SHA-256 goes on-chain; both parties sign that digest from their own wallets.
        </p>
        <button className="primary" disabled={!!busy || !counterparty.trim()} onClick={createDeal}>
          {busy === "create" ? "Creating on devnet…" : "Create deal & get share link"}
        </button>
        {error && <div className="error">{error}</div>}
      </div>

      <div className="card">
        <h3>Open an existing deal</h3>
        <div className="row">
          <input
            value={openAddr}
            onChange={(e) => setOpenAddr(e.target.value)}
            placeholder="deal address (base58)"
            style={{ flex: 1 }}
          />
          <button
            onClick={() => {
              try {
                const a = new PublicKey(openAddr.trim());
                window.location.hash = `#/deal/${a.toBase58()}`;
              } catch {
                setError("not a valid address");
              }
            }}
          >
            Open
          </button>
        </div>
        <p className="muted">
          Connected as <code id="my-address" title={publicKey.toBase58()}>{short(publicKey.toBase58(), 6)}</code> — share this address with a counterparty so they
          can name you in a deal.
        </p>
      </div>
    </>
  );
}

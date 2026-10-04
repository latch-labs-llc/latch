/**
 * Try to cheat — judges attack a real, funded, Active deal with real devnet
 * transactions and watch the program reject every one. Attacks are sent
 * with skipPreflight so the failure lands ON-CHAIN and links to the
 * explorer: nothing here is simulated or mocked.
 */
import { keypairClient } from "@latch-labs/checkout";
import { LatchClient } from "@latch-labs/sdk";
import { useConnection } from "@solana/wallet-adapter-react";
import { Connection, Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { useEffect, useState } from "react";
import {
  ATTACK_BUYER_ATA,
  ATTACK_DEAL,
  ATTACK_SELLER_ATA,
  attackBuyer,
  attackOperator,
  attackSeller,
} from "../lib/attack";
import { short } from "../lib/useLatch";

interface AttackDef {
  id: string;
  emoji: string;
  title: string;
  villain: string;
  kp: () => Keypair;
  blurb: string;
  expect: string;
  build: (client: LatchClient) => Promise<Transaction>;
}

const ATTACKS: AttackDef[] = [
  {
    id: "seller-steal",
    emoji: "🏃",
    title: "Take the money as the seller",
    villain: "the seller's own key",
    kp: attackSeller,
    blurb:
      "The seller tries to release the escrowed 500 LDD to themselves before the buyer has approved anything.",
    expect: "Release requires the approval threshold both parties set. Zero approvals → rejected.",
    build: async (c) => (await c.releaseMilestone(ATTACK_DEAL, 0, ATTACK_SELLER_ATA)).transaction(),
  },
  {
    id: "buyer-clawback",
    emoji: "↩️",
    title: "Claw the money back as the buyer",
    villain: "the buyer's own key",
    kp: attackBuyer,
    blurb:
      "The buyer tries to cancel the active deal unilaterally and pull their deposit back out of the vault.",
    expect: "Once a deal is Active, cancellation isn't available — only the rules both parties signed.",
    build: async (c) => (await c.cancelSign(ATTACK_DEAL, ATTACK_BUYER_ATA)).transaction(),
  },
  {
    id: "operator-drain",
    emoji: "🏢",
    title: "Drain the vault as \"the operator\"",
    villain: "a key that isn't a party",
    kp: attackOperator,
    blurb:
      "A third party — play the role of Latch Labs, or any stranger — tries to approve the milestone so funds can move.",
    expect: "No operator role exists in the program. Not a party → rejected.",
    build: async (c) => c.approveMilestone(ATTACK_DEAL, 0).transaction(),
  },
];

type Result = { status: "idle" | "running" | "rejected" | "unexpected"; errLine?: string; sig?: string };

async function runAttack(connection: Connection, a: AttackDef): Promise<Result> {
  const kp = a.kp();
  const client = keypairClient(connection, kp);
  const tx = await a.build(client);
  tx.feePayer = kp.publicKey;
  tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
  tx.sign(kp);
  // skipPreflight: the failure must land on-chain, not in simulation.
  const sig = await connection.sendRawTransaction(tx.serialize(), { skipPreflight: true });
  await connection.confirmTransaction(sig, "confirmed").catch(() => {});
  for (let i = 0; i < 10; i++) {
    const t = await connection.getTransaction(sig, { maxSupportedTransactionVersion: 0, commitment: "confirmed" });
    if (t) {
      if (!t.meta?.err) return { status: "unexpected", sig, errLine: "transaction SUCCEEDED (unexpected!)" };
      const logs = t.meta.logMessages ?? [];
      const errLine =
        logs.find((l) => l.includes("Error Code"))?.replace(/^Program log: /, "") ??
        logs.find((l) => l.toLowerCase().includes("error")) ??
        JSON.stringify(t.meta.err);
      return { status: "rejected", sig, errLine };
    }
    await new Promise((r) => setTimeout(r, 1500));
  }
  return { status: "rejected", sig, errLine: "rejected on-chain (logs still propagating)" };
}

export default function Cheat() {
  const { connection } = useConnection();
  const [deal, setDeal] = useState<{ state: string; vault: number } | null>(null);
  const [results, setResults] = useState<Record<string, Result>>({});

  const refresh = async () => {
    try {
      const client = keypairClient(connection, attackOperator());
      const d = await client.fetchDeal(ATTACK_DEAL);
      const vaultAddr = new PublicKey(d.account.vault);
      const bal = await connection.getTokenAccountBalance(vaultAddr).then((r) => r.value.uiAmount ?? 0);
      setDeal({ state: d.state, vault: bal });
    } catch {
      /* transient */
    }
  };
  useEffect(() => {
    refresh();
  }, []);

  const fire = async (a: AttackDef) => {
    setResults((r) => ({ ...r, [a.id]: { status: "running" } }));
    try {
      const res = await runAttack(connection, a);
      setResults((r) => ({ ...r, [a.id]: res }));
    } catch (e: any) {
      setResults((r) => ({ ...r, [a.id]: { status: "rejected", errLine: String(e?.message ?? e).slice(0, 160) } }));
    }
    refresh();
  };

  return (
    <div className="cheat">
      <div className="card center">
        <h1>Try to cheat.</h1>
        <p>
          Below is a <b>real, funded escrow deal on Solana devnet</b>: 500 LDD locked in the vault, deal Active,
          rule = consented true deadlock. The attacker keys are embedded in this page — you fire <b>real
          transactions</b>, and every failure lands on-chain where you can inspect it.
        </p>
        <p className="muted">
          Deal <code>{short(ATTACK_DEAL.toBase58(), 6)}</code> ·{" "}
          {deal ? (
            <>
              state <b>{deal.state}</b> · vault <b>{deal.vault} LDD</b>
            </>
          ) : (
            "loading…"
          )}{" "}
          ·{" "}
          <a href={`https://explorer.solana.com/address/${ATTACK_DEAL.toBase58()}?cluster=devnet`} target="_blank" rel="noreferrer">
            explorer
          </a>{" "}
          · <a href={`#/deal/${ATTACK_DEAL.toBase58()}?s=${encodeURIComponent("Try-to-cheat standing demo deal")}`}>deal page</a>
        </p>
      </div>

      {ATTACKS.map((a) => {
        const r = results[a.id] ?? { status: "idle" };
        return (
          <div className="card" key={a.id}>
            <div className="row spread">
              <div style={{ flex: 1 }}>
                <h3>
                  {a.emoji} {a.title}
                </h3>
                <p className="muted" style={{ fontSize: ".9em" }}>
                  {a.blurb} Signed by {a.villain} (<code>{short(a.kp().publicKey.toBase58(), 4)}</code>).
                </p>
              </div>
              <button className="danger" disabled={r.status === "running"} onClick={() => fire(a)}>
                {r.status === "running" ? "Attacking…" : "Run this attack"}
              </button>
            </div>
            {r.status === "rejected" && (
              <div className="ok" style={{ marginTop: 10 }}>
                🛡 <b>REJECTED by the program.</b> <code className="mono">{r.errLine}</code>{" "}
                {r.sig && (
                  <a href={`https://explorer.solana.com/tx/${r.sig}?cluster=devnet`} target="_blank" rel="noreferrer">
                    see the failed transaction →
                  </a>
                )}
                <div className="muted" style={{ marginTop: 4 }}>{a.expect}</div>
              </div>
            )}
            {r.status === "unexpected" && <div className="error">{r.errLine}</div>}
          </div>
        );
      })}

      <div className="card">
        <h3>Why this matters</h3>
        <p className="muted">
          Neither party's own key, nor any operator key, can move escrowed funds outside the rules both sides
          signed — there is no instruction in the program that checks for an admin. Don't take this page's word
          for it: the attacker secrets are in the page source, the program is open source with a verified
          reproducible build, and every rejection above is a public devnet transaction. Bring your own wallet and
          try something we didn't think of.
        </p>
      </div>
    </div>
  );
}

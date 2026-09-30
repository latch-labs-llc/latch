#!/usr/bin/env node
/**
 * latch — CLI for the Latch split-control escrow program.
 *
 * Global options:
 *   -u, --url <rpc>        RPC endpoint (default: devnet)
 *   -k, --keypair <path>   Signer keypair file (default: ~/.config/solana/id.json)
 *
 * ⚠ Unaudited, experimental, devnet-only. Do not use with real funds.
 */
import { AnchorProvider, Wallet } from "@anchor-lang/core";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { Command } from "commander";
import { createHash } from "crypto";
import { readFileSync } from "fs";
import { homedir } from "os";
import {
  BN,
  DeadlockRule,
  DeadlockRuleName,
  LatchClient,
  RiskFlags,
  TimerMode,
  TimerModeName,
  dealStateName,
  enumName,
  fetchEventHistory,
} from "@latch-labs/sdk";

const program = new Command();
program
  .name("latch")
  .description("Split-control escrow on Solana (unaudited — devnet only)")
  .option("-u, --url <rpc>", "RPC endpoint", "https://api.devnet.solana.com")
  .option("-k, --keypair <path>", "signer keypair file", `${homedir()}/.config/solana/id.json`);

function client(): LatchClient {
  const opts = program.opts();
  const kp = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(readFileSync(opts.keypair, "utf8")))
  );
  const provider = new AnchorProvider(
    new Connection(opts.url, "confirmed"),
    new Wallet(kp),
    { commitment: "confirmed" }
  );
  return LatchClient.fromProvider(provider);
}

const pk = (s: string) => new PublicKey(s);
const ok = (label: string) => (sig: string) => console.log(`${label}: ${sig}`);

program
  .command("create")
  .description("Create a deal (Draft) with a terms document hashed on-chain")
  .requiredOption("--mint <pubkey>", "escrowed token mint")
  .requiredOption("--party <pubkey...>", "party wallets in order (creator must be included)")
  .requiredOption("--milestone <amount...>", "milestone amounts in raw base units")
  .requiredOption("--terms-file <path>", "agreement document; its SHA-256 goes on-chain")
  .option("--payer-idx <n>", "payer party index", "0")
  .option("--payee-idx <n>", "payee party index", "1")
  .option("--threshold <n>", "approvals required per milestone", "")
  .option("--rule <rule>", "TimeoutRelease|TimeoutRefund|AutoSplit|TieBreaker|LongSunset|TrueDeadlock", "TimeoutRefund")
  .option("--timer <mode>", "FromDeadlock|FromActivation", "FromDeadlock")
  .option("--timeout <secs>", "timeout for time-based rules", "604800")
  .option("--split-bps <bps>", "payee share for AutoSplit", "0")
  .option("--tie-breaker <pubkey>", "tie-breaker for TieBreaker rule")
  .requiredOption("--recovery <pubkey...>", "party-chosen recovery signers (1-3)")
  .option("--recovery-threshold <n>", "recovery signatures required", "1")
  .option("--recovery-delay <secs>", "on-chain notice delay before recovery executes", "259200")
  .option("--accept-risk-flags <bits>", "accepted mint risk flags bitfield", String(RiskFlags.ALL))
  .option("--token-2022", "mint is a Token-2022 mint")
  .option("--deal-id <n>", "deal id (default: unix time)")
  .action(async (o) => {
    const c = client();
    const dealId = o.dealId ? Number(o.dealId) : Math.floor(Date.now() / 1000);
    const termsHash = createHash("sha256").update(readFileSync(o.termsFile)).digest();
    const rule = DeadlockRule[o.rule as DeadlockRuleName];
    const timer = TimerMode[o.timer as TimerModeName];
    if (!rule) throw new Error(`unknown rule ${o.rule}`);
    if (!timer) throw new Error(`unknown timer mode ${o.timer}`);
    const builder = c.createDeal({
      dealId,
      parties: o.party.map(pk),
      payerIdx: Number(o.payerIdx),
      payeeIdx: Number(o.payeeIdx),
      approvalThreshold: o.threshold ? Number(o.threshold) : o.party.length,
      termsHash,
      milestoneAmounts: o.milestone.map((m: string) => new BN(m)),
      deadlockRule: rule,
      timerMode: timer,
      timeoutSecs: Number(o.timeout),
      splitBps: Number(o.splitBps),
      tieBreaker: o.tieBreaker ? pk(o.tieBreaker) : undefined,
      recoverySigners: o.recovery.map(pk),
      recoveryThreshold: Number(o.recoveryThreshold),
      recoveryDelaySecs: Number(o.recoveryDelay),
      acceptedRiskFlags: Number(o.acceptRiskFlags),
      mint: pk(o.mint),
      tokenProgram: o.token2022
        ? new PublicKey("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb")
        : undefined,
    });
    const wallet = (c.program.provider as AnchorProvider).wallet.publicKey;
    console.log("deal:", c.dealPda(wallet, dealId).toBase58());
    console.log("terms sha256:", termsHash.toString("hex"));
    await builder.rpc().then(ok("create_deal"));
  });

program
  .command("sign <deal>")
  .description("Execute the agreement: sign its on-chain terms hash")
  .option("--no-consent", "withhold true-deadlock consent (rejected on TrueDeadlock deals)")
  .action(async (deal, o) => {
    await client().signTerms(pk(deal), o.consent).rpc().then(ok("sign_terms"));
  });

program
  .command("deposit <deal> <amount>")
  .description("Fund the escrow vault (payer only; raw base units)")
  .requiredOption("--from <tokenAccount>", "payer's token account")
  .action(async (deal, amount, o) => {
    const b = await client().deposit(pk(deal), new BN(amount), pk(o.from));
    await b.rpc().then(ok("deposit"));
  });

program
  .command("ready <deal>")
  .description("Confirm ready to perform")
  .action(async (deal) => {
    await client().confirmReady(pk(deal)).rpc().then(ok("confirm_ready"));
  });

program
  .command("approve <deal> <index>")
  .description("Approve a milestone for release")
  .action(async (deal, index) => {
    await client().approveMilestone(pk(deal), Number(index)).rpc().then(ok("approve_milestone"));
  });

program
  .command("release <deal> <index>")
  .description("Release an approved milestone to the payee (permissionless)")
  .requiredOption("--to <tokenAccount>", "payee's token account")
  .action(async (deal, index, o) => {
    const b = await client().releaseMilestone(pk(deal), Number(index), pk(o.to));
    await b.rpc().then(ok("release_milestone"));
  });

program
  .command("dispute <deal>")
  .description("Raise a deadlock (pauses a FromActivation clock)")
  .action(async (deal) => {
    await client().raiseDeadlock(pk(deal)).rpc().then(ok("raise_deadlock"));
  });

program
  .command("withdraw-dispute <deal>")
  .description("Withdraw a deadlock you raised (resumes the clock)")
  .action(async (deal) => {
    await client().withdrawDeadlock(pk(deal)).rpc().then(ok("withdraw_deadlock"));
  });

program
  .command("settle-sign <deal> <amountToPayee>")
  .description("Sign a settlement payout (all parties matching → resolvable)")
  .action(async (deal, amount) => {
    await client().resolutionSign(pk(deal), new BN(amount)).rpc().then(ok("resolution_sign"));
  });

program
  .command("resolve <deal>")
  .description("Execute the deal's resolution once conditions hold (permissionless)")
  .requiredOption("--payer-ta <tokenAccount>")
  .requiredOption("--payee-ta <tokenAccount>")
  .action(async (deal, o) => {
    const b = await client().resolve(pk(deal), pk(o.payerTa), pk(o.payeeTa));
    await b.rpc().then(ok("resolve"));
  });

program
  .command("recovery-sign <deal> <amountToPayee>")
  .description("Recovery signer: sign a recovery payout (starts the notice clock at threshold)")
  .action(async (deal, amount) => {
    await client().recoverySign(pk(deal), new BN(amount)).rpc().then(ok("recovery_sign"));
  });

program
  .command("recovery-exec <deal>")
  .description("Execute a recovery payout after the notice delay (permissionless)")
  .requiredOption("--payer-ta <tokenAccount>")
  .requiredOption("--payee-ta <tokenAccount>")
  .action(async (deal, o) => {
    const b = await client().recoveryExecute(pk(deal), pk(o.payerTa), pk(o.payeeTa));
    await b.rpc().then(ok("recovery_execute"));
  });

program
  .command("cancel-draft <deal>")
  .description("Cancel a Draft deal (any party)")
  .action(async (deal) => {
    await client().cancelDraft(pk(deal)).rpc().then(ok("cancel_draft"));
  });

program
  .command("cancel-sign <deal>")
  .description("Sign a mutual cancellation (Signed/Funded; refunds payer when all sign)")
  .requiredOption("--payer-ta <tokenAccount>", "payer's token account for the refund")
  .action(async (deal, o) => {
    const b = await client().cancelSign(pk(deal), pk(o.payerTa));
    await b.rpc().then(ok("cancel_sign"));
  });

program
  .command("show <deal>")
  .description("Show a deal's on-chain state")
  .action(async (deal) => {
    const c = client();
    const d = await c.fetchDeal(pk(deal));
    const a = d.account;
    console.log(`state:      ${d.state}`);
    console.log(`vault:      ${d.vault.toBase58()}`);
    console.log(`mint:       ${a.mint.toBase58()}`);
    console.log(`terms hash: ${Buffer.from(a.termsHash).toString("hex")}`);
    console.log(`parties (${a.numParties}):`);
    for (let i = 0; i < a.numParties; i++) {
      const roles = [i === a.payerIdx ? "payer" : "", i === a.payeeIdx ? "payee" : ""]
        .filter(Boolean)
        .join(",");
      const signed = (a.signed & (1 << i)) !== 0 ? "signed" : "unsigned";
      console.log(`  [${i}] ${a.parties[i].toBase58()} ${roles} ${signed}`);
    }
    console.log(`deposited:  ${a.deposited.toString()} / ${a.totalAmount.toString()}`);
    console.log(`milestones (${a.numMilestones}, threshold ${a.approvalThreshold}):`);
    for (let i = 0; i < a.numMilestones; i++) {
      const m = a.milestones[i];
      console.log(`  [${i}] ${m.amount.toString()} ${m.released ? "RELEASED" : `approvals=${m.approvals}`}`);
    }
    console.log(`rule:       ${enumName(a.deadlockRule as object)} (${enumName(a.timerMode as object)}, ${a.timeoutSecs.toString()}s)`);
    console.log(`recovery:   ${a.recoveryThreshold}-of-${a.numRecovery}, notice ${a.recoveryDelaySecs.toString()}s`);
  });

program
  .command("events <deal>")
  .description("Decode the full event history of a deal (paced for public RPC)")
  .action(async (deal) => {
    const c = client();
    const history = await fetchEventHistory(c.program, c.connection, pk(deal));
    for (const e of history) {
      const when = e.blockTime ? new Date(e.blockTime * 1000).toISOString() : "?";
      console.log(`${String(e.data.seq ?? "?").padStart(3)}  ${when}  ${e.name}`);
    }
  });

program.parseAsync().catch((e) => {
  console.error(e.message ?? e);
  process.exit(1);
});

/**
 * SDK end-to-end test against devnet: a full two-party, two-milestone deal
 * driven exclusively through @latch-labs/sdk — create, sign (both parties),
 * fund, confirm ready, approve + release both milestones, then verify the
 * final state and decode the complete event history.
 *
 * Run: node dist/e2e/devnet.js   (needs ~0.2 devnet SOL in ~/.config/solana/id.json)
 */
import { AnchorProvider, Wallet } from "@anchor-lang/core";
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  createInitializeMint2Instruction,
  createInitializeAccount3Instruction,
  createMintToInstruction,
  MINT_SIZE,
  ACCOUNT_SIZE,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { createHash } from "crypto";
import { readFileSync } from "fs";
import { homedir } from "os";
import {
  BN,
  DeadlockRule,
  LatchClient,
  RiskFlags,
  TimerMode,
  fetchEventHistory,
} from "../index";

const RPC = process.env.LATCH_RPC ?? "https://api.devnet.solana.com";

function loadFunder(): Keypair {
  const raw = JSON.parse(
    readFileSync(`${homedir()}/.config/solana/id.json`, "utf8")
  );
  return Keypair.fromSecretKey(Uint8Array.from(raw));
}

async function main() {
  const connection = new Connection(RPC, "confirmed");
  const alice = loadFunder(); // payer party + creator
  const bob = Keypair.generate(); // payee

  const aliceClient = LatchClient.fromProvider(
    new AnchorProvider(connection, new Wallet(alice), { commitment: "confirmed" })
  );
  const bobClient = LatchClient.fromProvider(
    new AnchorProvider(connection, new Wallet(bob), { commitment: "confirmed" })
  );
  console.log("program:", aliceClient.programId.toBase58());
  console.log("alice:", alice.publicKey.toBase58());
  console.log("bob:  ", bob.publicKey.toBase58());

  const balance = await connection.getBalance(alice.publicKey);
  if (balance < 0.2 * LAMPORTS_PER_SOL) {
    throw new Error("need ≥0.2 devnet SOL in ~/.config/solana/id.json");
  }

  // ---- test mint + token accounts (plain SPL, 6 decimals) ----
  const send = async (tx: Transaction, signers: Keypair[]) => {
    const { blockhash } = await connection.getLatestBlockhash();
    tx.recentBlockhash = blockhash;
    tx.feePayer = alice.publicKey;
    tx.sign(...signers);
    const sig = await connection.sendRawTransaction(tx.serialize());
    await connection.confirmTransaction(sig, "confirmed");
    return sig;
  };

  const mintKp = Keypair.generate();
  const mintRent = await connection.getMinimumBalanceForRentExemption(MINT_SIZE);
  const taRent = await connection.getMinimumBalanceForRentExemption(ACCOUNT_SIZE);
  const aliceTa = Keypair.generate();
  const bobTa = Keypair.generate();
  await send(
    new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: alice.publicKey,
        toPubkey: bob.publicKey,
        lamports: 0.05 * LAMPORTS_PER_SOL,
      }),
      SystemProgram.createAccount({
        fromPubkey: alice.publicKey,
        newAccountPubkey: mintKp.publicKey,
        lamports: mintRent,
        space: MINT_SIZE,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeMint2Instruction(mintKp.publicKey, 6, alice.publicKey, null)
    ),
    [alice, mintKp]
  );
  for (const ta of [aliceTa, bobTa]) {
    const owner = ta === aliceTa ? alice.publicKey : bob.publicKey;
    await send(
      new Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: alice.publicKey,
          newAccountPubkey: ta.publicKey,
          lamports: taRent,
          space: ACCOUNT_SIZE,
          programId: TOKEN_PROGRAM_ID,
        }),
        createInitializeAccount3Instruction(ta.publicKey, mintKp.publicKey, owner)
      ),
      [alice, ta]
    );
  }
  await send(
    new Transaction().add(
      createMintToInstruction(mintKp.publicKey, aliceTa.publicKey, alice.publicKey, 2_000_000_000)
    ),
    [alice]
  );
  console.log("mint + token accounts ready");

  // ---- the deal, driven entirely through the SDK ----
  const dealId = Math.floor(Date.now() / 1000);
  const termsHash = createHash("sha256")
    .update(`SDK e2e agreement ${dealId}`)
    .digest();

  const builder = aliceClient.createDeal({
    dealId,
    parties: [alice.publicKey, bob.publicKey],
    payerIdx: 0,
    payeeIdx: 1,
    approvalThreshold: 2,
    termsHash,
    milestoneAmounts: [600_000_000, 400_000_000],
    deadlockRule: DeadlockRule.TimeoutRefund,
    timerMode: TimerMode.FromDeadlock,
    timeoutSecs: 3600,
    recoverySigners: [Keypair.generate().publicKey],
    recoveryThreshold: 1,
    recoveryDelaySecs: 86400,
    acceptedRiskFlags: RiskFlags.ALL,
    mint: mintKp.publicKey,
  });
  const deal = aliceClient.dealPda(alice.publicKey, dealId);
  console.log("create_deal:", await builder.rpc());

  console.log("sign (alice):", await aliceClient.signTerms(deal).rpc());
  console.log("sign (bob):  ", await bobClient.signTerms(deal).rpc());

  console.log(
    "deposit:",
    await (await aliceClient.deposit(deal, 1_000_000_000, aliceTa.publicKey)).rpc()
  );

  console.log("ready (alice):", await aliceClient.confirmReady(deal).rpc());
  console.log("ready (bob):  ", await bobClient.confirmReady(deal).rpc());

  for (const index of [0, 1]) {
    await aliceClient.approveMilestone(deal, index).rpc();
    await bobClient.approveMilestone(deal, index).rpc();
    const sig = await (
      await bobClient.releaseMilestone(deal, index, bobTa.publicKey)
    ).rpc();
    console.log(`milestone ${index} approved+released:`, sig);
  }

  // ---- verify final state + decode the full event history ----
  const info = await aliceClient.fetchDeal(deal);
  console.log("final state:", info.state, "| released:", info.account.releasedTotal.toString());
  if (info.state !== "Completed") throw new Error("deal did not complete");

  const history = await fetchEventHistory(aliceClient.program, connection, deal);
  console.log(`event history (${history.length} events):`);
  for (const e of history) {
    console.log(`  seq ${e.data.seq?.toString?.() ?? "?"}  ${e.name}`);
  }
  const names = history.map((e) => e.name);
  for (const expected of ["dealCreated", "termsSigned", "depositReceived", "milestoneReleased"]) {
    if (!names.some((n) => n.toLowerCase() === expected.toLowerCase())) {
      throw new Error(`missing expected event ${expected}`);
    }
  }
  console.log("\nSDK devnet E2E: PASS ✅");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

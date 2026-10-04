/**
 * Checkout-package E2E against a local validator (or devnet with LATCH_RPC):
 * mints a test stablecoin, then runs a full GUEST checkout session through
 * the public package API — create session, advance to Active (deal created,
 * both signatures, funded, both ready) — and watches the deal emit its
 * Active event through watchDeal. Exercises exactly what a merchant
 * integration uses: LatchCheckout, FundingAdapter, LocalMerchantAdapter.
 *
 * Run: node dist/e2e/local.js  (funder: LATCH_FUNDER or ~/.config/solana/id.json)
 */
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountIdempotentInstruction,
  createInitializeMint2Instruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { readFileSync } from "fs";
import { homedir } from "os";
import {
  FundingAdapter,
  LatchCheckout,
  LocalMerchantAdapter,
  watchDeal,
  keypairClient,
} from "../index";

const RPC = process.env.LATCH_RPC ?? "http://127.0.0.1:8899";
const IS_LOCAL = /127\.0\.0\.1|localhost/.test(RPC);
const DECIMALS = 6;

function loadFunder(): Keypair {
  const path = process.env.LATCH_FUNDER ?? `${homedir()}/.config/solana/id.json`;
  return Keypair.fromSecretKey(Uint8Array.from(JSON.parse(readFileSync(path, "utf8"))));
}

async function main() {
  const connection = new Connection(RPC, "confirmed");
  const funder = loadFunder();
  const merchantKp = Keypair.generate();

  if (IS_LOCAL) {
    const sig = await connection.requestAirdrop(funder.publicKey, 10 * LAMPORTS_PER_SOL);
    await connection.confirmTransaction(sig, "confirmed");
  }

  const send = async (tx: Transaction, signers: Keypair[]) => {
    tx.feePayer = funder.publicKey;
    tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
    tx.sign(...signers);
    const sig = await connection.sendRawTransaction(tx.serialize());
    await connection.confirmTransaction(sig, "confirmed");
  };

  // Test stablecoin, mint authority = funder.
  const mintKp = Keypair.generate();
  const mintRent = await connection.getMinimumBalanceForRentExemption(MINT_SIZE);
  await send(
    new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: funder.publicKey,
        toPubkey: merchantKp.publicKey,
        lamports: 0.1 * LAMPORTS_PER_SOL,
      }),
      SystemProgram.createAccount({
        fromPubkey: funder.publicKey,
        newAccountPubkey: mintKp.publicKey,
        lamports: mintRent,
        space: MINT_SIZE,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeMint2Instruction(mintKp.publicKey, DECIMALS, funder.publicKey, null)
    ),
    [funder, mintKp]
  );
  console.log("test mint:", mintKp.publicKey.toBase58());

  // Funding adapter: "simulated card" — tops up SOL for fees and mints the
  // purchase amount to the buyer, the role a licensed onramp fills on mainnet.
  const funding: FundingAdapter = {
    async ensureFunds({ buyer, price, onProgress }) {
      if ((await connection.getBalance(buyer)) < 0.02 * LAMPORTS_PER_SOL) {
        await send(
          new Transaction().add(
            SystemProgram.transfer({
              fromPubkey: funder.publicKey,
              toPubkey: buyer,
              lamports: 0.05 * LAMPORTS_PER_SOL,
            })
          ),
          [funder]
        );
        onProgress("  · fee money delivered");
      }
      const ata = getAssociatedTokenAddressSync(mintKp.publicKey, buyer, true);
      const bal = await connection
        .getTokenAccountBalance(ata)
        .then((r) => r.value.uiAmount ?? 0)
        .catch(() => 0);
      if (bal < price) {
        await send(
          new Transaction().add(
            createAssociatedTokenAccountIdempotentInstruction(
              funder.publicKey,
              ata,
              buyer,
              mintKp.publicKey
            ),
            createMintToInstruction(
              mintKp.publicKey,
              ata,
              funder.publicKey,
              BigInt(price) * BigInt(10 ** DECIMALS)
            )
          ),
          [funder]
        );
        onProgress("  · simulated card charged → test dollars delivered");
      }
    },
  };

  const checkout = new LatchCheckout({
    connection,
    mint: mintKp.publicKey,
    decimals: DECIMALS,
    merchant: new LocalMerchantAdapter(connection, merchantKp),
    funding,
  });

  const guest = Keypair.generate();
  const session = checkout.createSession({
    item: { id: "desk-01", name: "Vintage writing desk" },
    price: 120,
    subject: "Purchase of: Vintage writing desk — from E2E Test Store",
    guest,
  });
  const persisted = JSON.parse(JSON.stringify(session.state)); // resume-safe record
  console.log("session deal:", session.deal.toBase58());

  // Merchant-side watcher, armed before the buyer pays.
  const merchantClient = keypairClient(connection, merchantKp);
  const seen: string[] = [];
  const watcher = watchDeal(
    merchantClient,
    session.deal,
    (e) => {
      seen.push(e.state);
      console.log(`  [merchant webhook] state=${e.state}`);
    },
    { intervalMs: 1000 }
  );

  // Restore from the persisted record (proves serialization) and pay.
  const restored = checkout.restoreSession(persisted);
  await restored.advance(null, { onProgress: (m) => console.log(m) });

  const d = await merchantClient.fetchDeal(session.deal);
  if (d.state !== "Active") throw new Error(`expected Active, got ${d.state}`);
  await new Promise((r) => setTimeout(r, 2500)); // let the watcher observe Active
  watcher.stop();
  if (!seen.includes("Active")) throw new Error("merchant watcher never saw Active");

  console.log("\nCHECKOUT PACKAGE E2E: PASS ✅ (guest checkout → Active, watcher fired)");
}

main().catch((e) => {
  console.error("FAILED:", e);
  process.exit(1);
});

/**
 * Devnet SOL tap — the public faucet's requestAirdrop is heavily rate-limited
 * and fails for most visitors, so the app carries its own small tap: a funded
 * devnet keypair that hands out fee money. The secret is DELIBERATELY public
 * (devnet SOL is free and worthless); the tap pays its own fee so a brand-new
 * wallet needs nothing to receive. Capped per request; refilled as needed.
 */
import { Connection, Keypair, PublicKey, SystemProgram, Transaction } from "@solana/web3.js";

export const SOL_TAP_AMOUNT = 0.05; // enough for dozens of transactions

const TAP_SECRET = new Uint8Array([
  222, 233, 214, 14, 105, 244, 207, 34, 81, 125, 20, 58, 183, 72, 223, 134,
  250, 63, 98, 158, 191, 93, 116, 229, 65, 8, 241, 82, 166, 218, 201, 63, 61,
  155, 19, 187, 86, 57, 227, 30, 16, 68, 150, 108, 65, 238, 154, 141, 128, 66,
  203, 193, 196, 36, 136, 29, 34, 178, 62, 196, 121, 122, 105, 96,
]);

/** Send tap SOL to `recipient`. The tap signs and pays the fee itself. */
export async function tapSol(connection: Connection, recipient: PublicKey): Promise<string> {
  const tap = Keypair.fromSecretKey(TAP_SECRET);
  const tx = new Transaction().add(
    SystemProgram.transfer({
      fromPubkey: tap.publicKey,
      toPubkey: recipient,
      lamports: Math.round(SOL_TAP_AMOUNT * 1e9),
    })
  );
  tx.feePayer = tap.publicKey;
  tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;
  tx.sign(tap);
  const sig = await connection.sendRawTransaction(tx.serialize());
  await connection.confirmTransaction(sig, "confirmed");
  return sig;
}

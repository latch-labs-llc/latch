/**
 * Latch Demo Dollar (LDD) — a devnet-only test token so anyone can try Latch
 * without hunting for faucet tokens.
 *
 * The mint authority's secret key is DELIBERATELY public: LDD is play money
 * on devnet, worth nothing by construction, and anyone being able to mint it
 * is the feature. Never do this with a real token.
 */
import {
  createAssociatedTokenAccountIdempotentInstruction,
  createMintToInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { Connection, Keypair, PublicKey, Transaction } from "@solana/web3.js";

export const LDD_MINT = new PublicKey("2fZy4obHV2oUBqptHiANRmQU4A2vF8fTriT1mTPVLANi");
export const LDD_DECIMALS = 6;

const LDD_AUTHORITY_SECRET = new Uint8Array([
  164, 121, 250, 107, 96, 83, 124, 88, 155, 252, 141, 249, 233, 253, 66, 38,
  150, 154, 8, 202, 106, 233, 63, 104, 121, 184, 169, 196, 239, 108, 98, 255,
  63, 39, 36, 97, 232, 62, 94, 59, 54, 123, 54, 57, 245, 205, 2, 27, 91, 106,
  12, 175, 163, 106, 201, 193, 2, 76, 58, 183, 70, 165, 143, 21,
]);

export function lddAuthority(): Keypair {
  return Keypair.fromSecretKey(LDD_AUTHORITY_SECRET);
}

export function lddAta(owner: PublicKey): PublicKey {
  return getAssociatedTokenAddressSync(LDD_MINT, owner, true);
}

/** Build a transaction that mints `uiAmount` LDD to the owner's ATA (creating it if needed). */
export function buildLddFaucetTx(owner: PublicKey, uiAmount: number): { tx: Transaction; authority: Keypair } {
  const ata = lddAta(owner);
  const authority = lddAuthority();
  const tx = new Transaction().add(
    createAssociatedTokenAccountIdempotentInstruction(owner, ata, owner, LDD_MINT),
    createMintToInstruction(LDD_MINT, ata, authority.publicKey, BigInt(Math.round(uiAmount * 10 ** LDD_DECIMALS)))
  );
  return { tx, authority };
}

export async function tokenUiBalance(connection: Connection, ata: PublicKey): Promise<number> {
  try {
    const r = await connection.getTokenAccountBalance(ata);
    return r.value.uiAmount ?? 0;
  } catch {
    return 0;
  }
}

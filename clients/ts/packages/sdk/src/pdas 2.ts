import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

/** SPL Associated Token Account program (constant across clusters). */
export const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey(
  "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
);
export const TOKEN_PROGRAM_ID = new PublicKey(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
);
export const TOKEN_2022_PROGRAM_ID = new PublicKey(
  "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
);

/** The deal account PDA: seeds ["deal", creator, deal_id (u64 LE)]. */
export function dealPda(
  creator: PublicKey,
  dealId: BN | number | bigint,
  programId: PublicKey
): PublicKey {
  const id = new BN(dealId.toString());
  return PublicKey.findProgramAddressSync(
    [Buffer.from("deal"), creator.toBuffer(), id.toArrayLike(Buffer, "le", 8)],
    programId
  )[0];
}

/** Anchor event-CPI authority PDA: seeds ["__event_authority"]. */
export function eventAuthorityPda(programId: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("__event_authority")],
    programId
  )[0];
}

/** The escrow vault: the deal PDA's associated token account. */
export function vaultAta(
  deal: PublicKey,
  mint: PublicKey,
  tokenProgram: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [deal.toBuffer(), tokenProgram.toBuffer(), mint.toBuffer()],
    ASSOCIATED_TOKEN_PROGRAM_ID
  )[0];
}

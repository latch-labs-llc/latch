import { Program } from "@anchor-lang/core";
import {
  Connection,
  PublicKey,
  VersionedTransactionResponse,
} from "@solana/web3.js";
import bs58 from "bs58";
import { Latch } from "./idl/latch";

/**
 * Anchor `emit_cpi!` events ride in the program's self-CPI inner-instruction
 * data — NOT in logs (logs can be truncated by RPC providers). Layout:
 * 8-byte event-instruction tag, then the borsh event (8-byte discriminator +
 * fields).
 */
const EVENT_IX_TAG_LE = Buffer.from([0xe4, 0x45, 0xa5, 0x2e, 0x51, 0xcb, 0x9a, 0x1d]);

export interface DecodedLatchEvent {
  name: string;
  data: any;
}

/** Decode all Latch CPI events out of a fetched transaction. */
export function decodeEventsFromTransaction(
  program: Program<Latch>,
  tx: VersionedTransactionResponse
): DecodedLatchEvent[] {
  const out: DecodedLatchEvent[] = [];
  const meta = tx.meta;
  if (!meta || meta.err) return out;
  const keys = tx.transaction.message.getAccountKeys({
    accountKeysFromLookups: meta.loadedAddresses ?? undefined,
  });
  const programIdStr = program.programId.toBase58();
  for (const group of meta.innerInstructions ?? []) {
    for (const ix of group.instructions) {
      const pid = keys.get(ix.programIdIndex);
      if (!pid || pid.toBase58() !== programIdStr) continue;
      const data = Buffer.from(bs58.decode(ix.data));
      if (data.length < 16 || !data.subarray(0, 8).equals(EVENT_IX_TAG_LE)) continue;
      const decoded = program.coder.events.decode(
        data.subarray(8).toString("base64")
      );
      if (decoded) out.push(decoded);
    }
  }
  return out;
}

/** Fetch a transaction by signature and decode its Latch events. */
export async function fetchEvents(
  program: Program<Latch>,
  connection: Connection,
  signature: string
): Promise<DecodedLatchEvent[]> {
  const tx = await connection.getTransaction(signature, {
    maxSupportedTransactionVersion: 0,
    commitment: "confirmed",
  });
  if (!tx) return [];
  return decodeEventsFromTransaction(program, tx);
}

/**
 * Walk the full on-chain history of a deal (or the whole program) and return
 * every event, oldest first. This is the primitive behind signature
 * certificates and adoption metrics.
 */
export async function fetchEventHistory(
  program: Program<Latch>,
  connection: Connection,
  address: PublicKey
): Promise<Array<DecodedLatchEvent & { signature: string; blockTime: number | null }>> {
  const sigs = await connection.getSignaturesForAddress(address, {}, "confirmed");
  const out: Array<DecodedLatchEvent & { signature: string; blockTime: number | null }> = [];
  for (const s of sigs.reverse()) {
    if (s.err) continue;
    // Pace requests: public RPC endpoints rate-limit getTransaction hard.
    await new Promise((r) => setTimeout(r, 350));
    const events = await fetchEvents(program, connection, s.signature);
    for (const e of events) {
      out.push({ ...e, signature: s.signature, blockTime: s.blockTime ?? null });
    }
  }
  return out;
}

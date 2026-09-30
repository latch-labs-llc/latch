/**
 * Deterministic agreement template. The text is a pure function of the deal's
 * on-chain fields plus a short free-text subject carried in the share link, so
 * ANY client can regenerate it and verify its SHA-256 against the terms hash
 * stored on-chain — a self-verifying agreement.
 */
import { PublicKey } from "@solana/web3.js";
import { LATCH_PROGRAM_ID } from "@latch-labs/sdk";

export interface AgreementParams {
  deal: PublicKey;
  dealId: string; // decimal string
  buyer: PublicKey;
  seller: PublicKey;
  mint: PublicKey;
  decimals: number;
  milestoneAmounts: bigint[]; // raw units
  ruleName: string;
  timerModeName: string;
  timeoutSecs: string;
  recoverySigners: PublicKey[];
  recoveryThreshold: number;
  recoveryDelaySecs: string;
  subject: string;
}

function ui(raw: bigint, decimals: number): string {
  const s = raw.toString().padStart(decimals + 1, "0");
  return `${s.slice(0, -decimals)}.${s.slice(-decimals)}`;
}

export function buildAgreement(p: AgreementParams): string {
  const total = p.milestoneAmounts.reduce((a, b) => a + b, 0n);
  const milestones = p.milestoneAmounts
    .map((m, i) => `   ${i + 1}. ${ui(m, p.decimals)} tokens`)
    .join("\n");
  return `LATCH ESCROW AGREEMENT (template v1 — devnet demonstration)

This is a demonstration on Solana devnet. It creates no legal obligations
and involves no real funds.

1. SUBJECT. ${p.subject}

2. PARTIES.
   Buyer (payer):  ${p.buyer.toBase58()}
   Seller (payee): ${p.seller.toBase58()}

3. CONSIDERATION AND ESCROW. Buyer shall deposit ${ui(total, p.decimals)} tokens
   (mint ${p.mint.toBase58()}) into the split-control escrow vault of
   on-chain deal ${p.deal.toBase58()} (deal id ${p.dealId}) under program
   ${LATCH_PROGRAM_ID.toBase58()} on Solana devnet. No party and no third
   party, including any operator, can move escrowed funds unilaterally.

4. MILESTONES. Funds release per milestone upon the required approvals:
${milestones}

5. RESOLUTION. Rule fixed at formation: ${p.ruleName} (timer mode
   ${p.timerModeName}, timeout ${p.timeoutSecs} seconds). The parties may
   settle any dispute at any time by jointly signing a payout division.

6. RECOVERY. Recovery signers (${p.recoveryThreshold}-of-${p.recoverySigners.length}):
${p.recoverySigners.map((r) => `   ${r.toBase58()}`).join("\n")}
   Recovery payouts may only be directed to the parties and become
   executable only after a ${p.recoveryDelaySecs}-second on-chain notice period.

7. ELECTRONIC EXECUTION. The parties consent to transact electronically.
   Each party executes this agreement by a Solana transaction, signed by
   their wallet, recording approval of this document's SHA-256 digest on
   the deal account. The on-chain record of digest, signatures, and
   timestamps is the authoritative record of execution.

EXECUTION: by on-chain signature of each party's wallet on deal
${p.deal.toBase58()}.
`;
}

export async function sha256(text: string): Promise<Uint8Array> {
  const data = new TextEncoder().encode(text);
  const digest = await crypto.subtle.digest("SHA-256", data as BufferSource);
  return new Uint8Array(digest);
}

export function hex(bytes: Uint8Array | number[]): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

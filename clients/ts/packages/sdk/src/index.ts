/**
 * @latch-labs/sdk — TypeScript SDK for Latch, the split-control escrow
 * primitive on Solana.
 *
 * Quickstart:
 * ```ts
 * import { LatchClient, DeadlockRule, TimerMode, RiskFlags } from "@latch-labs/sdk";
 * const client = LatchClient.fromProvider(provider); // AnchorProvider
 * const { deal } = await client.createDeal({ ... }).then(b => b.rpc());
 * ```
 *
 * ⚠ Latch is unaudited, experimental, devnet-only software. Do not use with
 * real funds.
 */
import { AnchorProvider, BN, Program } from "@anchor-lang/core";
import { Connection, PublicKey, SystemProgram } from "@solana/web3.js";
import idlJson from "./idl/latch.json";
import { Latch } from "./idl/latch";
import {
  dealPda,
  eventAuthorityPda,
  vaultAta,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
} from "./pdas";
import { DeadlockRule, TimerMode, RiskFlags, dealStateName } from "./enums";

export * from "./pdas";
export * from "./enums";
export * from "./events";
export { BN };
export type { Latch };
export const IDL = idlJson as Latch;
export const LATCH_PROGRAM_ID = new PublicKey((idlJson as any).address);
/** Seconds a funded deal may wait for every party to confirm ready (from the IDL). */
export const ACTIVATION_WINDOW_SECS = Number(
  (idlJson as any).constants.find((c: any) => c.name === "ACTIVATION_WINDOW_SECS").value
);

/** Parameters for {@link LatchClient.createDeal}. Amounts are raw base units. */
export interface CreateDealArgs {
  /** Deal id, unique per creator (e.g. unix seconds). */
  dealId: BN | number;
  /** All party wallets, 2–8, creator included. */
  parties: PublicKey[];
  payerIdx: number;
  payeeIdx: number;
  /** M-of-N approvals required to release each milestone. */
  approvalThreshold: number;
  /** SHA-256 of the parties' off-chain agreement document. */
  termsHash: Uint8Array | number[];
  /** Milestone amounts in raw base units; must sum to the total. */
  milestoneAmounts: (BN | number)[];
  deadlockRule: (typeof DeadlockRule)[keyof typeof DeadlockRule];
  timerMode?: (typeof TimerMode)[keyof typeof TimerMode];
  timeoutSecs?: BN | number;
  splitBps?: number;
  tieBreaker?: PublicKey;
  /** Party-chosen recovery signers (1–3) and threshold. */
  recoverySigners: PublicKey[];
  recoveryThreshold: number;
  /** On-chain notice delay before a recovery payout becomes executable. */
  recoveryDelaySecs?: BN | number;
  /** Bitfield of mint risk flags the parties accept (see {@link RiskFlags}). */
  acceptedRiskFlags: number;
  priceRiskTerm?: number;
  /** The escrowed mint and its token program. */
  mint: PublicKey;
  tokenProgram?: PublicKey;
}

/** A fetched deal account plus derived conveniences. */
export interface DealInfo {
  address: PublicKey;
  vault: PublicKey;
  state: string;
  account: Awaited<ReturnType<Program<Latch>["account"]["deal"]["fetch"]>>;
}

export class LatchClient {
  readonly program: Program<Latch>;
  readonly connection: Connection;

  constructor(program: Program<Latch>) {
    this.program = program;
    this.connection = program.provider.connection;
  }

  /** Build from an AnchorProvider (wallet + connection). */
  static fromProvider(provider: AnchorProvider): LatchClient {
    return new LatchClient(new Program<Latch>(IDL, provider));
  }

  get programId(): PublicKey {
    return this.program.programId;
  }

  // ---------------------------------------------------------------- derive

  dealPda(creator: PublicKey, dealId: BN | number): PublicKey {
    return dealPda(creator, dealId, this.programId);
  }

  // ---------------------------------------------------------------- fetch

  async fetchDeal(address: PublicKey): Promise<DealInfo> {
    const account = await this.program.account.deal.fetch(address);
    return {
      address,
      vault: vaultAta(address, account.mint, account.tokenProgram),
      state: dealStateName(account.state as object),
      account,
    };
  }

  // ------------------------------------------------------------ lifecycle
  // Each method returns Anchor's MethodsBuilder: chain `.rpc()` to send,
  // `.instruction()` to compose, or `.transaction()` to sign elsewhere.

  createDeal(args: CreateDealArgs) {
    const creator = (this.program.provider as AnchorProvider).wallet.publicKey;
    const tokenProgram = args.tokenProgram ?? TOKEN_PROGRAM_ID;
    const dealId = new BN(args.dealId);
    const deal = this.dealPda(creator, dealId);
    const milestoneAmounts = args.milestoneAmounts.map((a) => new BN(a));
    const totalAmount = milestoneAmounts.reduce((s, a) => s.add(a), new BN(0));
    const params = {
      dealId,
      parties: args.parties,
      payerIdx: args.payerIdx,
      payeeIdx: args.payeeIdx,
      approvalThreshold: args.approvalThreshold,
      termsHash: Array.from(args.termsHash),
      priceRiskTerm: args.priceRiskTerm ?? 0,
      totalAmount,
      milestoneAmounts,
      deadlockRule: args.deadlockRule,
      timerMode: args.timerMode ?? TimerMode.FromDeadlock,
      timeoutSecs: new BN(args.timeoutSecs ?? 0),
      splitBps: args.splitBps ?? 0,
      tieBreaker: args.tieBreaker ?? PublicKey.default,
      recoverySigners: args.recoverySigners,
      recoveryThreshold: args.recoveryThreshold,
      recoveryDelaySecs: new BN(args.recoveryDelaySecs ?? 0),
      acceptedRiskFlags: args.acceptedRiskFlags,
    };
    return this.program.methods.createDeal(params as any).accounts({
      creator,
      mint: args.mint,
      tokenProgram,
    } as any);
  }

  signTerms(deal: PublicKey, consentTrueDeadlock = true, party?: PublicKey) {
    return this.program.methods
      .signTerms(consentTrueDeadlock)
      .accounts({ party: this.signer(party), deal } as any);
  }

  async deposit(deal: PublicKey, amount: BN | number, payerTokenAccount: PublicKey) {
    const d = await this.fetchDeal(deal);
    return this.program.methods.deposit(new BN(amount)).accounts({
      payer: this.signer(),
      deal,
      mint: d.account.mint,
      vault: d.vault,
      payerTokenAccount,
      tokenProgram: d.account.tokenProgram,
    } as any);
  }

  confirmReady(deal: PublicKey, party?: PublicKey) {
    return this.program.methods
      .confirmReady()
      .accounts({ party: this.signer(party), deal } as any);
  }

  approveMilestone(deal: PublicKey, index: number, party?: PublicKey) {
    return this.program.methods
      .approveMilestone(index)
      .accounts({ party: this.signer(party), deal } as any);
  }

  /** Permissionless: anyone can crank a release once approvals are in. */
  async releaseMilestone(deal: PublicKey, index: number, payeeTokenAccount: PublicKey) {
    const d = await this.fetchDeal(deal);
    return this.program.methods.releaseMilestone(index).accounts({
      deal,
      mint: d.account.mint,
      vault: d.vault,
      payeeTokenAccount,
      tokenProgram: d.account.tokenProgram,
    } as any);
  }

  raiseDeadlock(deal: PublicKey, party?: PublicKey) {
    return this.program.methods
      .raiseDeadlock()
      .accounts({ party: this.signer(party), deal } as any);
  }

  /** Only the party who raised the deadlock may withdraw it. */
  withdrawDeadlock(deal: PublicKey, party?: PublicKey) {
    return this.program.methods
      .withdrawDeadlock()
      .accounts({ party: this.signer(party), deal } as any);
  }

  /** Sign (or propose) a settlement payout; matching signatures from all
   * parties enable `resolve`. Under TieBreaker, the tie-breaker's signature
   * alone decides. */
  resolutionSign(deal: PublicKey, amountToPayee: BN | number, signer?: PublicKey) {
    return this.program.methods
      .resolutionSign(new BN(amountToPayee))
      .accounts({ signer: this.signer(signer), deal } as any);
  }

  /** Permissionless: applies the deal's resolution once conditions hold. */
  async resolve(deal: PublicKey, payerTokenAccount: PublicKey, payeeTokenAccount: PublicKey) {
    const d = await this.fetchDeal(deal);
    return this.program.methods.resolve().accounts({
      deal,
      mint: d.account.mint,
      vault: d.vault,
      payerTokenAccount,
      payeeTokenAccount,
      tokenProgram: d.account.tokenProgram,
    } as any);
  }

  recoverySign(deal: PublicKey, amountToPayee: BN | number, signer?: PublicKey) {
    return this.program.methods
      .recoverySign(new BN(amountToPayee))
      .accounts({ signer: this.signer(signer), deal } as any);
  }

  /** Permissionless once M-of-N recovery signatures are in and the on-chain
   * notice delay has elapsed. Pays only the parties. */
  async recoveryExecute(deal: PublicKey, payerTokenAccount: PublicKey, payeeTokenAccount: PublicKey) {
    const d = await this.fetchDeal(deal);
    return this.program.methods.recoveryExecute().accounts({
      deal,
      mint: d.account.mint,
      vault: d.vault,
      payerTokenAccount,
      payeeTokenAccount,
      tokenProgram: d.account.tokenProgram,
    } as any);
  }

  cancelDraft(deal: PublicKey, party?: PublicKey) {
    return this.program.methods
      .cancelDraft()
      .accounts({ party: this.signer(party), deal } as any);
  }

  async cancelSign(deal: PublicKey, payerTokenAccount: PublicKey, party?: PublicKey) {
    const d = await this.fetchDeal(deal);
    return this.program.methods.cancelSign().accounts({
      party: this.signer(party),
      deal,
      mint: d.account.mint,
      vault: d.vault,
      payerTokenAccount,
      tokenProgram: d.account.tokenProgram,
    } as any);
  }

  /**
   * Return the whole vault to the payer once a deal holding a deposit has gone
   * `ACTIVATION_WINDOW_SECS` (3 days) without becoming Active — measured from
   * full funding, or from signing for a partially funded deal. Permissionless:
   * any wallet may submit it; the payout can only reach the payer.
   */
  async refundUnactivated(deal: PublicKey, payerTokenAccount: PublicKey, cranker?: PublicKey) {
    const d = await this.fetchDeal(deal);
    return this.program.methods.refundUnactivated().accounts({
      cranker: this.signer(cranker),
      deal,
      mint: d.account.mint,
      vault: d.vault,
      payerTokenAccount,
      tokenProgram: d.account.tokenProgram,
    } as any);
  }

  /**
   * The payee alone returns everything still in the vault to the payer — an
   * honest seller backing out. Payer-only destination; valid with a deposit in
   * Signed, or in Funded / Active / Deadlocked.
   */
  async refundByPayee(deal: PublicKey, payerTokenAccount: PublicKey, payee?: PublicKey) {
    const d = await this.fetchDeal(deal);
    return this.program.methods.refundByPayee().accounts({
      payee: this.signer(payee),
      deal,
      mint: d.account.mint,
      vault: d.vault,
      payerTokenAccount,
      tokenProgram: d.account.tokenProgram,
    } as any);
  }

  // ---------------------------------------------------------------- utils

  private signer(explicit?: PublicKey): PublicKey {
    return explicit ?? (this.program.provider as AnchorProvider).wallet.publicKey;
  }
}

export { SystemProgram, ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID };

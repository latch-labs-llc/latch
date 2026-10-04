/**
 * State-driven checkout sessions. Every step is derived from ON-CHAIN state,
 * so a checkout interrupted at any point (RPC hiccup, closed tab, crashed
 * process) resumes from exactly where it stopped: persist `session.state`
 * before the first transaction and call `advance` again. All RPC calls retry
 * with backoff; deterministic program rejections surface immediately.
 */
import { AnchorProvider, BN } from "@anchor-lang/core";
import {
  DeadlockRule,
  LATCH_PROGRAM_ID,
  LatchClient,
  RiskFlags,
  TOKEN_PROGRAM_ID,
  TimerMode,
  dealPda,
  vaultAta,
} from "@latch-labs/sdk";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { buildAgreement, sha256 } from "./agreement";
import { keypairClient } from "./merchant";
import { withRetry } from "./retry";
import {
  CheckoutConfig,
  CheckoutItem,
  CheckoutSessionState,
} from "./types";

const DEFAULT_PROTECTION_DAYS = 7;
const DEFAULT_RECOVERY_DELAY_SECS = 259200; // 3 days

export interface CreateSessionParams {
  item: CheckoutItem;
  /** Price in UI units of the configured mint. */
  price: number;
  /** Agreement subject line (appears verbatim in the signed agreement). */
  subject: string;
  /** The paying wallet. For guest checkout pass `guest` instead. */
  buyer?: PublicKey;
  /** Guest checkout: an ephemeral wallet created for this buyer. */
  guest?: Keypair;
}

export class LatchCheckout {
  readonly connection: Connection;
  private readonly cfg: Required<Pick<CheckoutConfig, "protectionDays" | "recoveryDelaySecs">> &
    CheckoutConfig;

  constructor(config: CheckoutConfig) {
    this.connection = config.connection;
    this.cfg = {
      protectionDays: DEFAULT_PROTECTION_DAYS,
      recoveryDelaySecs: DEFAULT_RECOVERY_DELAY_SECS,
      ...config,
    };
  }

  /** Start a new checkout session (no transaction is sent yet). */
  createSession(params: CreateSessionParams): CheckoutSession {
    const buyer = params.guest ? params.guest.publicKey : params.buyer;
    if (!buyer) throw new Error("createSession needs a buyer wallet or a guest keypair");
    const dealId = Math.floor(Date.now() / 1000);
    const state: CheckoutSessionState = {
      itemId: params.item.id,
      itemName: params.item.name,
      price: params.price,
      subject: params.subject,
      dealId,
      deal: dealPda(buyer, dealId, LATCH_PROGRAM_ID).toBase58(),
      buyer: buyer.toBase58(),
      guestSecret: params.guest ? Array.from(params.guest.secretKey) : undefined,
    };
    return new CheckoutSession(this.cfg, state);
  }

  /** Restore a persisted session (see `CheckoutSession.state`). */
  restoreSession(state: CheckoutSessionState): CheckoutSession {
    return new CheckoutSession(this.cfg, state);
  }
}

export class CheckoutSession {
  readonly state: CheckoutSessionState;
  readonly deal: PublicKey;
  private readonly cfg: CheckoutConfig &
    Required<Pick<CheckoutConfig, "protectionDays" | "recoveryDelaySecs">>;

  constructor(
    cfg: CheckoutConfig & Required<Pick<CheckoutConfig, "protectionDays" | "recoveryDelaySecs">>,
    state: CheckoutSessionState
  ) {
    this.cfg = cfg;
    this.state = state;
    this.deal = new PublicKey(state.deal);
  }

  /** The exact agreement text both parties sign (hash goes on-chain). */
  agreementText(): string {
    const s = this.state;
    return buildAgreement({
      deal: this.deal,
      dealId: String(s.dealId),
      buyer: new PublicKey(s.buyer),
      seller: this.cfg.merchant.publicKey,
      mint: this.cfg.mint,
      decimals: this.cfg.decimals,
      milestoneAmounts: [this.rawPrice()],
      ruleName: "TimeoutRelease",
      timerModeName: "FromActivation",
      timeoutSecs: String(this.cfg.protectionDays * 86400),
      recoverySigners: [new PublicKey(s.buyer), this.cfg.merchant.publicKey],
      recoveryThreshold: 2,
      recoveryDelaySecs: String(this.cfg.recoveryDelaySecs),
      subject: s.subject,
    });
  }

  /**
   * The buyer's LatchClient. Guest sessions build one from the stored
   * ephemeral key; wallet sessions must pass the connected client, which is
   * checked against the wallet the session was started with.
   */
  buyerClient(connected: LatchClient | null): { client: LatchClient; guestKeypair?: Keypair } {
    if (this.state.guestSecret) {
      const kp = Keypair.fromSecretKey(Uint8Array.from(this.state.guestSecret));
      return { client: keypairClient(this.cfg.connection, kp), guestKeypair: kp };
    }
    const want = this.state.buyer;
    if (!connected || connected.program.provider.publicKey!.toBase58() !== want) {
      throw new Error(`connect the wallet this order was started with (${want.slice(0, 6)}…)`);
    }
    return { client: connected };
  }

  /**
   * Drive the deal to Active from whatever state it is in, calling the
   * merchant adapter when the seller's signature is required and the funding
   * adapter when the buyer's balances fall short. Safe to call repeatedly;
   * returns when the escrow is Active (order placed).
   */
  async advance(
    connected: LatchClient | null,
    opts: { onProgress?: (message: string) => void } = {}
  ): Promise<void> {
    const push = opts.onProgress ?? (() => {});
    const { client, guestKeypair } = this.buyerClient(connected);
    const connection = this.cfg.connection;
    const buyer = new PublicKey(this.state.buyer);
    const raw = this.rawPrice();

    const ensureFunds = async () => {
      if (!this.cfg.funding) return;
      await this.cfg.funding.ensureFunds({
        connection,
        buyer,
        price: this.state.price,
        guestKeypair,
        client,
        onProgress: push,
      });
    };

    for (let guard = 0; guard < 12; guard++) {
      let state = "Missing";
      let account: any = null;
      try {
        const d = await withRetry("deal lookup", () => client.fetchDeal(this.deal));
        state = d.state;
        account = d.account;
      } catch {
        /* not created yet */
      }

      if (state === "Missing") {
        push("Preparing buyer balances…");
        await ensureFunds();
        push("Creating the escrow deal…");
        const termsHash = await sha256(this.agreementText());
        await withRetry("create deal", () =>
          client
            .createDeal({
              dealId: this.state.dealId,
              parties: [buyer, this.cfg.merchant.publicKey],
              payerIdx: 0,
              payeeIdx: 1,
              approvalThreshold: 2,
              termsHash,
              milestoneAmounts: [new BN(raw.toString())],
              deadlockRule: DeadlockRule.TimeoutRelease,
              timerMode: TimerMode.FromActivation,
              timeoutSecs: new BN(this.cfg.protectionDays * 86400),
              recoverySigners: [buyer, this.cfg.merchant.publicKey],
              recoveryThreshold: 2,
              recoveryDelaySecs: new BN(this.cfg.recoveryDelaySecs),
              acceptedRiskFlags: RiskFlags.ALL,
              mint: this.cfg.mint,
            })
            .rpc()
        );
      } else if (state === "Draft") {
        if (!(account.signed & 1)) {
          push("Signing the agreement (your signature)…");
          await withRetry("buyer signature", () => client.signTerms(this.deal, true).rpc());
        } else if (!(account.signed & 2)) {
          push("Merchant countersigning…");
          await this.cfg.merchant.countersign(this.deal);
        }
      } else if (state === "Signed") {
        push(`Placing ${this.state.price} into escrow…`);
        await ensureFunds();
        // ATA derivation is generic: [owner, token program, mint].
        const buyerAta = vaultAta(buyer, this.cfg.mint, TOKEN_PROGRAM_ID);
        await withRetry("deposit", async () =>
          (await client.deposit(this.deal, new BN(raw.toString()), buyerAta)).rpc()
        );
      } else if (state === "Funded") {
        if (!(account.ready & 1)) {
          push("Confirming ready (buyer)…");
          await withRetry("buyer ready", () => client.confirmReady(this.deal).rpc());
        } else {
          push("Confirming ready (merchant)…");
          await this.cfg.merchant.confirmReady(this.deal);
        }
      } else if (state === "Active") {
        return; // order placed
      } else {
        throw new Error(`unexpected deal state during checkout: ${state}`);
      }
    }
    throw new Error("checkout did not converge — call advance again to resume");
  }

  private rawPrice(): bigint {
    return BigInt(this.state.price) * BigInt(10 ** this.cfg.decimals);
  }
}

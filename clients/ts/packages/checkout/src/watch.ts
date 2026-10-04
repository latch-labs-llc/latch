/**
 * Merchant-side order events, webhook-shaped: subscribe to a deal and get a
 * callback on every state transition and milestone release. In production
 * the hosted API turns these into HTTP webhooks; the polling implementation
 * here serves the same contract directly from chain state.
 */
import { LatchClient } from "@latch-labs/sdk";
import { PublicKey } from "@solana/web3.js";

export interface DealEvent {
  deal: PublicKey;
  /** "Draft" | "Signed" | "Funded" | "Active" | "Deadlocked" | "Completed" | "Cancelled" */
  state: string;
  previousState: string | null;
  /** Milestones released so far (raw units, cumulative). */
  releasedTotal: bigint;
  account: any;
}

export interface WatchHandle {
  stop(): void;
}

/**
 * Poll a deal and invoke `onEvent` whenever its state or released total
 * changes (and once immediately with the current state). Returns a handle;
 * the watch stops itself when the deal reaches a terminal state.
 */
export function watchDeal(
  client: LatchClient,
  deal: PublicKey,
  onEvent: (e: DealEvent) => void,
  opts: { intervalMs?: number } = {}
): WatchHandle {
  const intervalMs = opts.intervalMs ?? 10_000;
  let prevState: string | null = null;
  let prevReleased = -1n;
  let stopped = false;

  const tick = async () => {
    if (stopped) return;
    try {
      const d = await client.fetchDeal(deal);
      const released = BigInt(d.account.releasedTotal?.toString?.() ?? 0);
      if (d.state !== prevState || released !== prevReleased) {
        const e: DealEvent = {
          deal,
          state: d.state,
          previousState: prevState,
          releasedTotal: released,
          account: d.account,
        };
        prevState = d.state;
        prevReleased = released;
        onEvent(e);
      }
      if (d.state === "Completed" || d.state === "Cancelled") {
        stopped = true;
        return;
      }
    } catch {
      /* transient — keep polling */
    }
    if (!stopped) setTimeout(tick, intervalMs);
  };
  void tick();

  return { stop: () => void (stopped = true) };
}

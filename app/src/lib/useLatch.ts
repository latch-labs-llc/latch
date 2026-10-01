import { AnchorProvider } from "@anchor-lang/core";
import { LatchClient } from "@latch-labs/sdk";
import { useAnchorWallet, useConnection } from "@solana/wallet-adapter-react";
import { Keypair } from "@solana/web3.js";
import { useMemo } from "react";
import { keypairWallet } from "./merchant";

/** LatchClient bound to the connected wallet (null when no wallet). */
export function useLatch(): LatchClient | null {
  const { connection } = useConnection();
  const wallet = useAnchorWallet();
  return useMemo(() => {
    if (!wallet) return null;
    const provider = new AnchorProvider(connection, wallet, { commitment: "confirmed" });
    return LatchClient.fromProvider(provider);
  }, [connection, wallet]);
}

/** A client regardless of wallet state: connected wallet if present, else an
 * ephemeral read-only identity (viewing deals, certificates, order status). */
export function useLatchOrReadonly(): LatchClient {
  const { connection } = useConnection();
  const wallet = useAnchorWallet();
  return useMemo(() => {
    const w = wallet ?? (keypairWallet(Keypair.generate()) as any);
    return LatchClient.fromProvider(new AnchorProvider(connection, w, { commitment: "confirmed" }));
  }, [connection, wallet]);
}

export function short(addr: string, n = 4): string {
  return `${addr.slice(0, n)}…${addr.slice(-n)}`;
}

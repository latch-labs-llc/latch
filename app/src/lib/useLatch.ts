import { AnchorProvider } from "@anchor-lang/core";
import { LatchClient } from "@latch-labs/sdk";
import { useAnchorWallet, useConnection } from "@solana/wallet-adapter-react";
import { useMemo } from "react";

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

export function short(addr: string, n = 4): string {
  return `${addr.slice(0, n)}…${addr.slice(-n)}`;
}

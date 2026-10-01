import { Buffer } from "buffer";
(globalThis as any).Buffer = Buffer;

import { WalletAdapterNetwork } from "@solana/wallet-adapter-base";
import { ConnectionProvider, WalletProvider } from "@solana/wallet-adapter-react";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
  UnsafeBurnerWalletAdapter,
} from "@solana/wallet-adapter-wallets";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";
import "@solana/wallet-adapter-react-ui/styles.css";

const endpoint =
  (import.meta as any).env?.VITE_RPC_URL || "https://api.devnet.solana.com";
const wallets = [
  new PhantomWalletAdapter({ network: WalletAdapterNetwork.Devnet }),
  new SolflareWalletAdapter({ network: WalletAdapterNetwork.Devnet }),
  // Zero-setup path for trying the app: an in-browser throwaway keypair.
  new UnsafeBurnerWalletAdapter(),
];

ReactDOM.createRoot(document.getElementById("root")!).render(
  <ConnectionProvider endpoint={endpoint}>
    <WalletProvider wallets={wallets} autoConnect>
      <WalletModalProvider>
        <App />
      </WalletModalProvider>
    </WalletProvider>
  </ConnectionProvider>
);

import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { useEffect, useState } from "react";
import Certificate from "./pages/Certificate";
import Deal from "./pages/Deal";
import Home from "./pages/Home";
import Orbit from "./pages/Orbit";
import Store from "./pages/Store";

/** Tiny hash router: #/ , #/deal/<address>?s=<subject> , #/cert/<address>?s=... */
export interface Route {
  page: "home" | "deal" | "cert" | "store" | "orbit";
  address?: string;
  subject: string;
}

function parseHash(): Route {
  const h = window.location.hash.replace(/^#\/?/, "");
  const [path, query] = h.split("?");
  const subject = new URLSearchParams(query ?? "").get("s") ?? "";
  const [page, address] = path.split("/");
  if (page === "deal" && address) return { page: "deal", address, subject };
  if (page === "cert" && address) return { page: "cert", address, subject };
  if (page === "store") return { page: "store", subject: "" };
  if (page === "orbit") return { page: "orbit", subject: "" };
  return { page: "home", subject: "" };
}

export default function App() {
  const [route, setRoute] = useState<Route>(parseHash());
  useEffect(() => {
    const onHash = () => setRoute(parseHash());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  return (
    <div className="shell">
      <header>
        <a className="brand" href="#/">
          <span className="logo">⟟</span> Latch
          <span className="tag">split-control escrow · devnet</span>
        </a>
        <WalletMultiButton />
      </header>
      <div className="warnbar">
        Unaudited, experimental software on Solana <b>devnet</b> — test tokens only, never real funds.
      </div>
      <main>
        {route.page === "home" && <Home />}
        {route.page === "store" && <Store />}
        {route.page === "orbit" && <Orbit />}
        {route.page === "deal" && <Deal address={route.address!} subject={route.subject} />}
        {route.page === "cert" && <Certificate address={route.address!} subject={route.subject} />}
      </main>
      <footer>
        <a href="https://github.com/latch-labs-llc/latch" target="_blank" rel="noreferrer">GitHub</a> ·{" "}
        <a href="https://www.npmjs.com/package/@latch-labs/sdk" target="_blank" rel="noreferrer">SDK</a> ·
        program <code>BT8tr…v5yv</code> · Apache-2.0
      </footer>
    </div>
  );
}

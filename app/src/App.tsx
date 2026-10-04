import { useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { useEffect, useState } from "react";
import Certificate from "./pages/Certificate";
import Cheat from "./pages/Cheat";
import Deal from "./pages/Deal";
import Home from "./pages/Home";
import Landing from "./pages/Landing";
import Orbit from "./pages/Orbit";
import Send from "./pages/Send";
import Store from "./pages/Store";

/** Tiny hash router: #/ , #/deal/<address>?s=<subject> , #/cert/<address>?s=... */
export interface Route {
  page: "home" | "deal" | "cert" | "store" | "orbit" | "about" | "cheat" | "send";
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
  if (page === "about") return { page: "about", subject: "" };
  if (page === "cheat") return { page: "cheat", subject: "" };
  if (page === "send") return { page: "send", subject: "" };
  return { page: "home", subject: "" };
}

export default function App() {
  const [route, setRoute] = useState<Route>(parseHash());
  const { publicKey } = useWallet();
  useEffect(() => {
    const onHash = () => setRoute(parseHash());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  // The marketing landing is a full-bleed standalone page with its own
  // chrome — shown at #/about and to disconnected visitors on #/.
  if (route.page === "about" || (route.page === "home" && !publicKey)) {
    return <Landing />;
  }

  return (
    <div className="shell">
      <header>
        <a className="brand" href="#/">
          <svg className="logo-svg" width="22" height="27" viewBox="0 0 460 560" aria-hidden="true">
            <path d="M130 230 v-60 a100 100 0 0 1 200 0 v60" fill="none" stroke="#1d4fd7" strokeWidth="44" strokeLinecap="round" />
            <rect x="70" y="230" width="320" height="260" rx="56" fill="none" stroke="#1d4fd7" strokeWidth="44" />
            <circle cx="160" cy="360" r="28" fill="#1d4fd7" />
            <circle cx="300" cy="360" r="28" fill="#1d4fd7" />
            <rect x="196" y="346" width="68" height="26" rx="13" fill="#1d4fd7" />
          </svg>
          Latch <span className="tag">protected payments · devnet</span>
        </a>
        <nav className="main">
          <a href="#/send">Send</a>
          <a href="#/store">Store</a>
          <a href="#/orbit">Orbit</a>
          <a href="#/about">About</a>
          <a href="https://latchlabs.org/api/" target="_blank" rel="noreferrer">Docs</a>
        </nav>
        <WalletMultiButton />
      </header>
      <div className="warnbar">
        Unaudited, experimental software on Solana <b>devnet</b> — test tokens only, never real funds.
      </div>
      <main>
        {route.page === "home" && <Home />}
        {route.page === "store" && <Store />}
        {route.page === "cheat" && <Cheat />}
        {route.page === "send" && <Send />}
        {route.page === "orbit" && <Orbit />}
        {route.page === "deal" && <Deal address={route.address!} subject={route.subject} />}
        {route.page === "cert" && <Certificate address={route.address!} subject={route.subject} />}
      </main>
      <footer>
        <a href="#/about">What is Latch?</a> ·{" "}
        <a href="https://github.com/latch-labs-llc/latch" target="_blank" rel="noreferrer">GitHub</a> ·{" "}
        <a href="https://www.npmjs.com/package/@latch-labs/sdk" target="_blank" rel="noreferrer">SDK</a> ·
        program <code>BT8tr…v5yv</code> · Apache-2.0
      </footer>
    </div>
  );
}

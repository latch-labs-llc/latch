/**
 * The marketing landing — shown to disconnected visitors on Home and
 * available to everyone at #/about ("What is Latch?").
 */
export default function Landing() {
  return (
    <>
      <div className="hero-band">
        <svg width="64" height="78" viewBox="0 0 460 560" aria-hidden="true">
          <path d="M130 230 v-60 a100 100 0 0 1 200 0 v60" fill="none" stroke="#fff" strokeWidth="40" strokeLinecap="round" />
          <rect x="70" y="230" width="320" height="260" rx="56" fill="none" stroke="#fff" strokeWidth="40" />
          <circle cx="160" cy="360" r="26" fill="#fff" />
          <circle cx="300" cy="360" r="26" fill="#fff" />
          <rect x="196" y="348" width="68" height="24" rx="12" fill="#fff" />
        </svg>
        <h1>Payments between strangers,<br />protected on both sides.</h1>
        <p>
          Latch holds the buyer's payment in a vault neither side — and no operator — can move alone, and releases
          it under rules both parties chose up front. Final settlement for the seller, no chargebacks; delivery
          protection for the buyer; a court-readable record of every signature.
        </p>
        <div className="ctas">
          <a className="btnlike gold" href="#/store">Try the demo store — no wallet needed</a>
          <a className="btnlike" href="#/orbit">Second merchant, ~15-line integration</a>
          <a className="btnlike" href="https://latchlabs.org/api/" target="_blank" rel="noreferrer">
            SDK API reference
          </a>
        </div>
      </div>

      <div className="proof">
        <span>Open source · Apache-2.0</span>
        <span>45 tests, every failure path</span>
        <span>Verified reproducible build</span>
        <span>3 packages on npm</span>
        <span>No fees in the protocol</span>
        <span>No token</span>
      </div>

      <div className="card">
        <h3>See it</h3>
        <div className="shots2">
          <figure>
            <a href="#/store"><img src="/shots/checkout-sheet.png" alt="Latch Checkout pay sheet" /></a>
            <figcaption>
              The pay sheet: card checkout with no wallet — escrow created, signed by both sides, and funded in one
              click.
            </figcaption>
          </figure>
          <figure>
            <img src="/shots/evidence-packet.png" alt="Evidence packet certificate" />
            <figcaption>
              Every order ends in an evidence packet a court can verify entirely from public chain data.
            </figcaption>
          </figure>
        </div>
      </div>

      <div className="card">
        <h3>How it compares</h3>
        <div className="tablewrap">
          <table className="compare">
            <thead>
              <tr>
                <th></th>
                <th>Card rails</th>
                <th>Zelle / wires</th>
                <th>Escrow.com</th>
                <th className="hl">Latch</th>
              </tr>
            </thead>
            <tbody>
              <tr><td>Protects the buyer</td><td>✓ (chargebacks)</td><td>✗</td><td>✓</td><td className="hl">✓ held until delivery</td></tr>
              <tr><td>Protects the seller</td><td>✗ eats chargebacks</td><td>✓</td><td>✓</td><td className="hl">✓ final settlement</td></tr>
              <tr><td>Who holds the money</td><td>processor</td><td>nobody</td><td>the company</td><td className="hl">a vault no one controls</td></tr>
              <tr><td>Protocol fees</td><td>~2.9% + 30¢</td><td>free</td><td>~3.25%</td><td className="hl">none, ever</td></tr>
              <tr><td>Evidence a court can verify</td><td>✗</td><td>✗</td><td>company records</td><td className="hl">✓ public-chain certificate</td></tr>
              <tr><td>Works if the operator vanishes</td><td>✗</td><td>—</td><td>✗</td><td className="hl">✓ permissionless</td></tr>
            </tbody>
          </table>
        </div>
      </div>

      <div className="card">
        <h3>How it works</h3>
        <div className="steps">
          <div><b>1 · Agree</b><br />Both sides sign the deal's terms — the agreement is hashed on-chain and self-verifying, so neither can later dispute what was signed.</div>
          <div><b>2 · Pay into the vault</b><br />The buyer funds a program-owned vault neither party — and no company — can move alone. Card checkout needs no wallet.</div>
          <div><b>3 · Settle</b><br />Funds release on delivery confirmation, the protection timer, a jointly signed settlement, or the rule the parties fixed up front. Anyone can execute it.</div>
        </div>
      </div>

      <div className="card">
        <h3>What's underneath</h3>
        <ul className="features">
          <li>Six resolution rules fixed at signing — timeouts, auto-split, a named arbiter, or consented true deadlock.</li>
          <li>Party-chosen recovery for lost keys and court orders: payouts only to the parties, after on-chain notice.</li>
          <li>Every order ends in a printable <b>evidence packet</b> — methodology and declaration included — that a court or auditor verifies entirely from public chain data.</li>
          <li>Token-2022 aware: freeze and seize powers are vetted and disclosed before any deal starts.</li>
          <li>Verified reproducible build; permissionless release cranks; Apache-2.0, no token, no fees.</li>
          <li>
            Nothing to host, no one to trust: the program runs on Solana itself, anyone can execute any step, and
            this app is an optional, forkable static page — every deal settles even if Latch Labs vanishes.
          </li>
        </ul>
        <p className="muted">
          Build on it: <code>npm install @latch-labs/checkout</code> — two storefronts on this site run on it;
          the smaller one integrates in ~15 lines. <a href="https://latchlabs.org/api/">API reference</a> ·{" "}
          <a href="https://github.com/latch-labs-llc/latch" target="_blank" rel="noreferrer">GitHub</a>
        </p>
      </div>
      <p className="center muted">
        Building custom deals (milestones, arbiters, disputes)? Connect a wallet above — or pick <b>Burner Wallet</b>{" "}
        for a zero-setup throwaway.
      </p>
    </>
  );
}

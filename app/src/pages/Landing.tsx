/**
 * Standalone marketing landing — full-bleed, its own chrome. Shown at
 * #/about and to disconnected visitors on #/. Connecting a wallet from the
 * nav drops the visitor into the deal tools (App switches layouts).
 */
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";

function Lock({ size = 22, color = "#fff" }: { size?: number; color?: string }) {
  return (
    <svg width={size} height={size * 1.22} viewBox="0 0 460 560" aria-hidden="true" style={{ display: "block" }}>
      <path d="M130 230 v-60 a100 100 0 0 1 200 0 v60" fill="none" stroke={color} strokeWidth="44" strokeLinecap="round" />
      <rect x="70" y="230" width="320" height="260" rx="56" fill="none" stroke={color} strokeWidth="44" />
      <circle cx="160" cy="360" r="28" fill={color} />
      <circle cx="300" cy="360" r="28" fill={color} />
      <rect x="196" y="346" width="68" height="26" rx="13" fill={color} />
    </svg>
  );
}

const CODE = `import { LatchCheckout, watchDeal } from "@latch-labs/checkout";

const checkout = new LatchCheckout({ connection, mint, decimals: 6, merchant, funding });
const session  = checkout.createSession({
  item: { id: "desk-01", name: "Vintage writing desk" },
  price: 120,
  subject: "Purchase of: Vintage writing desk — from Your Store",
  guest: Keypair.generate(),
});
await session.advance(null, { onProgress: console.log }); // → escrow Active
watchDeal(merchantClient, session.deal, (e) => e.state === "Active" && ship());`;

export default function Landing() {
  return (
    <div className="lp">
      <nav className="lp-nav">
        <div className="lp-wrap lp-nav-in">
          <a className="lp-brand" href="#/">
            <Lock size={20} />
            Latch
          </a>
          <div className="lp-links">
            <a href="#/send">Send money</a>
            <a href="#/store">Demo store</a>
            <a href="#/cheat">Try to cheat</a>
            <a href="https://latchlabs.org/api/" target="_blank" rel="noreferrer">Docs</a>
            <a href="https://github.com/latch-labs-llc/latch" target="_blank" rel="noreferrer">GitHub</a>
          </div>
          <WalletMultiButton />
        </div>
      </nav>

      <div className="lp-devnet">Devnet beta — unaudited, test tokens only, never real funds.</div>

      <header className="lp-hero">
        <div className="lp-wrap lp-hero-grid">
          <div>
            <div className="lp-herobadge">⬡ Open-source public-goods infrastructure · built on Solana · Apache-2.0</div>
            <h1>
              Payments between strangers,
              <br />
              <span className="lp-grad">protected on both sides.</span>
            </h1>
            <p className="lp-sub">
              Latch holds the buyer's payment in a vault neither side — and no operator — can move alone, and
              releases it under rules both parties chose up front. Final settlement for the seller. Delivery
              protection for the buyer. A court-readable record of every signature.
            </p>
            <div className="lp-ctas">
              <a className="lp-btn lp-btn-primary" href="#/store">Try the demo store →</a>
              <a className="lp-btn" href="#/send">Send a protected payment</a>
            </div>
            <div className="lp-metrics">
              <span><b>53</b> tests, every failure path</span>
              <span><b>Verified</b> reproducible build</span>
              <span><b>3</b> packages on npm</span>
              <span><b>0</b> protocol fees · no token</span>
            </div>
          </div>
          <div className="lp-shotframe">
            <div className="lp-browser">
              <div className="lp-browser-bar"><i /><i /><i /><span>latchlabs.org/#/store</span></div>
              <a href="#/store"><img src="/shots/storefront.png" alt="Latch demo storefront" /></a>
            </div>
          </div>
        </div>
      </header>

      <section className="lp-section">
        <div className="lp-wrap">
          <div className="lp-eyebrow">How it works</div>
          <h2>Three steps. No one to trust.</h2>
          <div className="lp-steps">
            <div>
              <svg viewBox="0 0 24 24" width="26" height="26" fill="none" stroke="#7ea4ff" strokeWidth="1.8"><path d="M4 20h16M6 16l9.5-9.5a2.1 2.1 0 1 1 3 3L9 19l-4 1z" /></svg>
              <h3>Agree</h3>
              <p>Both sides sign the deal's terms. The agreement is hashed on-chain and self-verifying — neither party can later dispute what was signed.</p>
            </div>
            <div>
              <svg viewBox="0 0 24 24" width="26" height="26" fill="none" stroke="#7ea4ff" strokeWidth="1.8"><rect x="4" y="10" width="16" height="10" rx="2.5" /><path d="M8 10V7a4 4 0 0 1 8 0v3" /><circle cx="12" cy="15" r="1.6" fill="#7ea4ff" /></svg>
              <h3>Pay into the vault</h3>
              <p>The buyer funds a program-owned vault neither party — and no company — can move alone. Card checkout needs no wallet at all.</p>
            </div>
            <div>
              <svg viewBox="0 0 24 24" width="26" height="26" fill="none" stroke="#7ea4ff" strokeWidth="1.8"><path d="M5 12l5 5L20 7" /></svg>
              <h3>Settle</h3>
              <p>Funds release on delivery confirmation, a protection timer, a joint settlement, or the rule fixed up front. Once a payout is due, anyone can submit it — no one can redirect it.</p>
            </div>
          </div>
        </div>
      </section>

      <section className="lp-section">
        <div className="lp-wrap">
          <div className="lp-eyebrow">Compare</div>
          <h2>Every other rail protects one side.</h2>
          <div className="lp-tablecard">
            <table>
              <thead>
                <tr><th></th><th>Card rails</th><th>Zelle / wires</th><th>Escrow.com</th><th className="hl">Latch</th></tr>
              </thead>
              <tbody>
                <tr><td>Protects the buyer</td><td><em className="y">✓</em> chargebacks</td><td><em className="n">✗</em></td><td><em className="y">✓</em></td><td className="hl"><em className="y">✓</em> held until delivery</td></tr>
                <tr><td>Protects the seller</td><td><em className="n">✗</em> eats chargebacks</td><td><em className="y">✓</em></td><td><em className="y">✓</em></td><td className="hl"><em className="y">✓</em> final settlement</td></tr>
                <tr><td>Who holds the money</td><td>processor</td><td>nobody</td><td>the company</td><td className="hl">a vault no one controls</td></tr>
                <tr><td>Protocol fees</td><td>~2.9% + 30¢</td><td>free</td><td>~3.25%</td><td className="hl">none, ever</td></tr>
                <tr><td>Court-verifiable evidence</td><td><em className="n">✗</em></td><td><em className="n">✗</em></td><td>company records</td><td className="hl"><em className="y">✓</em> public-chain certificate</td></tr>
                <tr><td>Survives the operator vanishing</td><td><em className="n">✗</em></td><td>—</td><td><em className="n">✗</em></td><td className="hl"><em className="y">✓</em> permissionless</td></tr>
              </tbody>
            </table>
          </div>
          <p className="lp-dim" style={{ marginTop: 14 }}>
            Don't take the table's word for it — <a href="#/cheat">attack a real funded deal and watch the
            program reject every attempt →</a>
          </p>
        </div>
      </section>

      <section className="lp-section">
        <div className="lp-wrap">
          <div className="lp-eyebrow">One form, one spectrum</div>
          <h2>Protection is the dial.</h2>
          <div className="lp-steps">
            <div>
              <h3>⚡ Instant</h3>
              <p>A plain transfer, final immediately. For people you trust — the baseline every wallet already gives you.</p>
            </div>
            <div>
              <h3>🛡 Protected</h3>
              <p>Held until the buyer confirms — or 7 days after both sides start. A dispute pauses the clock; then the money moves only when both sides agree.</p>
            </div>
            <div>
              <h3>⚙️ Custom</h3>
              <p>Milestones, arbiters, timers, party-chosen recovery signers — the full deal form, for when it's serious.</p>
            </div>
          </div>
          <p className="lp-dim" style={{ marginTop: 14 }}>
            Same rails underneath — the dial just sets the deal's formation parameters.{" "}
            <a href="#/send">Send a protected payment →</a>
          </p>
        </div>
      </section>

      <section className="lp-section">
        <div className="lp-wrap lp-split">
          <div>
            <div className="lp-eyebrow">The enforceability layer</div>
            <h2>Every order ends in evidence a court can verify.</h2>
            <p>
              Certificates are rebuilt entirely from public chain data — who signed which document hash, when,
              every event, every transaction. They print as an evidence packet with a methodology page and a
              signable declaration. No fact on it is asserted by any company, including ours.
            </p>
            <p className="lp-dim">
              Designed backward from one question: what has to be on-chain for a judge — or a stranger — to
              reconstruct who agreed to what?
            </p>
          </div>
          <div className="lp-doc">
            <img src="/shots/evidence-packet.png" alt="Evidence packet" />
          </div>
        </div>
      </section>

      <section className="lp-section">
        <div className="lp-wrap">
          <div className="lp-eyebrow">For developers</div>
          <h2>Escrow checkout in ten lines.</h2>
          <div className="lp-split lp-dev">
            <pre className="lp-code"><code>{CODE}</code></pre>
            <div>
              <p>
                <code className="lp-pill">npm install @latch-labs/checkout</code>
              </p>
              <p>
                Sessions, a merchant adapter, a funding adapter where a licensed onramp slots in on mainnet, and
                webhook-style order events — all deriving every step from on-chain state, so interrupted checkouts
                resume exactly where they stopped.
              </p>
              <p>
                Two storefronts on this site run on the published package; the <a href="#/orbit">smaller one</a>{" "}
                integrates in ~15 lines. Plus <code>@latch-labs/sdk</code> for everything else and a CLI for the
                terminal.
              </p>
              <p className="lp-dim">
                <a href="https://latchlabs.org/api/" target="_blank" rel="noreferrer">API reference</a> ·{" "}
                <a href="https://github.com/latch-labs-llc/latch/blob/main/INTEGRATION.md" target="_blank" rel="noreferrer">Integration guide</a> ·{" "}
                <a href="https://github.com/latch-labs-llc/latch" target="_blank" rel="noreferrer">Source</a>
              </p>
            </div>
          </div>
        </div>
      </section>

      <section className="lp-section">
        <div className="lp-wrap">
          <div className="lp-eyebrow">Roadmap</div>
          <h2>Built in the open, in this order.</h2>
          <div className="lp-road">
            <div>
              <span className="lp-chip lp-chip-done">Shipped</span>
              <h3>The primitive, proven</h3>
              <p>Verified reproducible program on devnet, 53 tests, three npm packages, two storefronts on the published checkout package, evidence packets on every deal.</p>
            </div>
            <div>
              <span className="lp-chip lp-chip-next">Next</span>
              <h3>Independent audit → mainnet</h3>
              <p>External security review, then mainnet with the upgrade key in a Squads multisig behind a timelock. Real money only after real review.</p>
            </div>
            <div>
              <span className="lp-chip lp-chip-done">Shipped</span>
              <h3>Protection is the dial</h3>
              <p>The dial is live on this site: instant with friends, held-until-you-confirm with strangers, the full deal form when it's serious — <a href="#/send">try a protected send →</a>. The standalone consumer app builds on it.</p>
            </div>
            <div>
              <span className="lp-chip lp-chip-next">Next</span>
              <h3>Hosted session API</h3>
              <p>The checkout interface served over HTTP with webhooks — and licensed onramp partners on the fiat edges, so no operator ever sits in the flow of funds.</p>
            </div>
            <div>
              <span className="lp-chip lp-chip-exp">Exploring</span>
              <h3>Documentary settlement</h3>
              <p>Releases triggered by delivery and inspection attestations — parcel tracking, documents against payment — with the arbiter role as examiner. The shape trade workflows need.</p>
            </div>
            <div>
              <span className="lp-chip lp-chip-exp">Exploring</span>
              <h3>Confidential amounts (ZK proofs)</h3>
              <p>Token-2022 confidential transfers hide deal amounts behind zero-knowledge proofs. The test suite already verifies real ZK ElGamal proofs; the account layout reserves the space.</p>
            </div>
          </div>
          <p className="lp-dim lp-road-note">
            Every stage ships open source under Apache-2.0 — the roadmap doubles as a contribution map for Solana
            developers and a reference for integrating Latch into your own product.{" "}
            <a href="https://github.com/latch-labs-llc/latch/blob/main/ROADMAP.md" target="_blank" rel="noreferrer">Full roadmap →</a>
          </p>
        </div>
      </section>

      <section className="lp-band">
        <div className="lp-wrap">
          <Lock size={34} color="#8fb0ff" />
          <h2>Nothing to host. No one to trust.</h2>
          <p>
            The program runs on Solana itself. Every release is a permissionless crank — anyone can submit it,
            no one can redirect it — and this site is an optional, forkable static page. Apache-2.0, no token, no fees in the protocol —{" "}
            <b>every deal settles even if Latch Labs vanishes.</b>
          </p>
          <a className="lp-btn lp-btn-primary" href="#/store">Try it in 60 seconds — no wallet needed</a>
        </div>
      </section>

      <footer className="lp-foot">
        <div className="lp-wrap lp-foot-grid">
          <div>
            <a className="lp-brand" href="#/"><Lock size={18} /> Latch</a>
            <p className="lp-dim">Protected payments on Solana.<br />Built by Latch Labs LLC.</p>
          </div>
          <div>
            <h4>Product</h4>
            <a href="#/send">Send money</a>
            <a href="#/store">Demo store</a>
            <a href="#/cheat">Try to cheat</a>
            <a href="#/about">What is Latch?</a>
          </div>
          <div>
            <h4>Developers</h4>
            <a href="https://latchlabs.org/api/" target="_blank" rel="noreferrer">API reference</a>
            <a href="https://github.com/latch-labs-llc/latch" target="_blank" rel="noreferrer">GitHub</a>
            <a href="https://www.npmjs.com/package/@latch-labs/checkout" target="_blank" rel="noreferrer">npm</a>
          </div>
          <div>
            <h4>Protocol</h4>
            <span className="lp-dim">Program BT8tr…v5yv · devnet</span>
            <span className="lp-dim">Verified reproducible build</span>
            <span className="lp-dim">Apache-2.0 · no token · no fees</span>
          </div>
        </div>
      </footer>
    </div>
  );
}

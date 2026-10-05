# Contributing

Thanks for looking under the hood. Latch is Apache-2.0, and contributions —
code, docs, issues, integration reports — are welcome. Issues tagged
`good first issue` are scoped entry points.

## Ground rules

This program is designed to hold other people's money, so the bar is:

- **Auditability over cleverness.** Plain code, explicit checks, a comment
  only where the code can't say it.
- **A test for every failure path.** New instructions or rule changes need
  LiteSVM coverage for the unhappy paths, not just the demo path.
- **No operator powers, ever.** No instruction may be gated on a maintainer
  key, and nothing may route funds anywhere but the parties' accounts.
  PRs that add fees, tokens, or admin paths will be declined regardless of
  quality.

## Building

Rust (stable), Agave 4.x CLI, Anchor 1.2.x, Node 22+.

```sh
make build     # REQUIRED path: anchor build --arch v0 (SBPF v3 is rejected
               # by devnet deploys and LiteSVM — plain `anchor build` breaks)
make test      # 75 LiteSVM tests
./scripts/e2e-local.sh   # full SDK + checkout lifecycle on a local validator
                         # (build clients/ts first: npm ci && npm run build)
```

TypeScript workspace lives in `clients/ts` (npm workspaces: sdk, checkout,
cli). The app is `app/` (Vite + React; `npm run dev`); its Playwright
judge-simulations are in `app/e2e/`.

Treat `cargo build-sbf` stack-frame warnings as errors — the binary they
produce fails verification at load. Box large accounts instead.

## Pull requests

- `cargo fmt` clean; keep clippy happy where reasonable.
- Reference the issue; explain *why*, briefly, in the PR body.
- Program changes: update `SECURITY_NOTES.md` if the trust surface moves,
  and `DECISIONS.md` if you made a judgment call.

## Security

Suspected vulnerabilities: see [SECURITY.md](SECURITY.md) — please don't
open public issues for them.

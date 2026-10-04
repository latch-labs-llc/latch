#!/usr/bin/env bash
# Full SDK lifecycle E2E against a local validator: boots solana-test-validator
# with the program preloaded at genesis, then drives a two-party, two-milestone
# deal end to end through @latch-labs/sdk (same script as the devnet E2E).
#
# Prereqs: `make build` (target/deploy/latch.so) and a built TS workspace
# (`cd clients/ts && npm ci && npm run build`). Run: ./scripts/e2e-local.sh
set -euo pipefail
cd "$(dirname "$0")/.."

PROGRAM_ID=BT8tr7YXYLQPsLMq3qCipY1fBKyjT3gmGQB5amV2v5yv
SO=target/deploy/latch.so
E2E=clients/ts/packages/sdk/dist/e2e/devnet.js
[ -f "$SO" ] || { echo "missing $SO — run 'make build' first" >&2; exit 1; }
[ -f "$E2E" ] || { echo "missing $E2E — run 'npm run build' in clients/ts first" >&2; exit 1; }

LEDGER=$(mktemp -d)
FUNDER=$(mktemp -d)/funder.json
solana-keygen new --no-bip39-passphrase --silent -o "$FUNDER"

solana-test-validator --reset --quiet --ledger "$LEDGER" \
  --bpf-program "$PROGRAM_ID" "$SO" &
VALIDATOR_PID=$!
cleanup() {
  kill "$VALIDATOR_PID" 2>/dev/null || true
  wait "$VALIDATOR_PID" 2>/dev/null || true
  rm -rf "$LEDGER" "$(dirname "$FUNDER")"
}
trap cleanup EXIT

echo "waiting for local validator…"
for _ in $(seq 1 60); do
  if curl -s http://127.0.0.1:8899 -X POST -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -q '"ok"'; then
    break
  fi
  sleep 1
done
curl -s http://127.0.0.1:8899 -X POST -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' | grep -q '"ok"' \
  || { echo "validator did not become healthy in 60s" >&2; exit 1; }

LATCH_RPC=http://127.0.0.1:8899 LATCH_FUNDER="$FUNDER" node "$E2E"
echo "LOCAL-VALIDATOR SDK E2E: PASS"

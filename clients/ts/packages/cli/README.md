# @latch-labs/cli

Command-line client for [Latch](https://github.com/latch-labs-llc/latch) —
split-control escrow on Solana.

> ⚠️ **Unaudited, experimental, devnet only. Do not use with real funds.**

```sh
npm install -g @latch-labs/cli
latch --help
```

Global options: `-u/--url <rpc>` (default devnet), `-k/--keypair <path>`
(default `~/.config/solana/id.json`).

## Full lifecycle

```sh
# Buyer creates the deal; the agreement file's SHA-256 goes on-chain.
latch create \
  --mint <MINT> \
  --party <BUYER> <SELLER> \
  --milestone 600000000 400000000 \
  --terms-file agreement.txt \
  --rule TimeoutRefund --timer FromDeadlock --timeout 604800 \
  --recovery <RECOVERY_AGENT> --recovery-delay 259200

latch sign <DEAL>                      # each party, with their own -k
latch deposit <DEAL> 1000000000 --from <BUYER_TOKEN_ACCOUNT>
latch ready <DEAL>                     # each party
latch approve <DEAL> 0                 # each approving party
latch release <DEAL> 0 --to <SELLER_TOKEN_ACCOUNT>   # anyone can crank

latch show <DEAL>                      # decoded on-chain state
latch events <DEAL>                    # full decoded event history
```

Disputes and recovery:

```sh
latch dispute <DEAL>                   # raise a deadlock
latch withdraw-dispute <DEAL>          # raiser only; resumes the clock
latch settle-sign <DEAL> <AMOUNT_TO_PAYEE>   # all parties matching → resolvable
latch resolve <DEAL> --payer-ta <TA> --payee-ta <TA>
latch recovery-sign <DEAL> <AMOUNT_TO_PAYEE>  # recovery signers
latch recovery-exec <DEAL> --payer-ta <TA> --payee-ta <TA>  # after notice delay
latch cancel-draft <DEAL> / latch cancel-sign <DEAL> --payer-ta <TA>
```

Apache-2.0. Program ID (devnet): `BT8tr7YXYLQPsLMq3qCipY1fBKyjT3gmGQB5amV2v5yv`.

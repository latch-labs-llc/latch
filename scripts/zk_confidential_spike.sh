#!/bin/zsh
# Devnet spike: Token-2022 confidential transfers end to end.
# Creates a confidential-transfer mint, configures an account, deposits public
# tokens into the encrypted balance, applies the pending balance, and withdraws
# — every step involving real ZK proofs verified by the live ZK ElGamal Proof
# Program on devnet. Prints tx signatures throughout.
#
# Requires: spl-token CLI, devnet SOL in ~/.config/solana/id.json.
set -e
export PATH="$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"

echo "== cluster: $(solana config get | grep RPC)"
echo "== payer: $(solana address) ($(solana balance))"

echo "\n== 1. Create a Token-2022 mint with confidential transfers enabled"
MINT=$(spl-token create-token --program-2022 --enable-confidential-transfers auto --output json | python3 -c "import sys,json; print(json.load(sys.stdin)['commandOutput']['address'])")
echo "mint: $MINT"

echo "\n== 2. Create the token account and configure its confidential extension"
spl-token create-account "$MINT"
spl-token configure-confidential-transfer-account --address-keypair-or-owner "$MINT" 2>/dev/null || spl-token configure-confidential-transfer-account "$MINT"

echo "\n== 3. Mint 1000 public tokens"
spl-token mint "$MINT" 1000

echo "\n== 4. Deposit 600 into the CONFIDENTIAL balance (amount becomes encrypted)"
spl-token deposit-confidential-tokens "$MINT" 600

echo "\n== 5. Apply pending balance (decrypt-and-roll-up on the owner side)"
spl-token apply-pending-balance "$MINT"

echo "\n== 6. Balances now — note the confidential portion is not visible on-chain"
spl-token display "$MINT"
spl-token balance "$MINT"

echo "\n== 7. Withdraw 100 back to the public balance (ZK equality + range proofs)"
spl-token withdraw-confidential-tokens "$MINT" 100

echo "\n== done. Inspect the account on explorer (?cluster=devnet) — the"
echo "   confidential balance shows only ciphertext."

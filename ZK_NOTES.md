# ZK privacy — current state and the path to confidential escrow

2026-09-22

## What works today (implemented and tested)

1. **In-suite proof verification** (`tests/test_zk_proofs.rs`): the native ZK
   ElGamal Proof Program is injected into LiteSVM as a builtin; tests generate
   real proofs client-side and verify them on-(test-)chain:
   - pubkey-validity proof accepted;
   - a bit-flipped proof rejected;
   - zero-ciphertext proof accepted for an encrypted 0 and **rejected for an
     encrypted 42** — the soundness property, demonstrated.
2. **Devnet spike** (`scripts/zk_confidential_spike.sh`): full Token-2022
   confidential-transfer flow against the live devnet proof program — CT mint,
   account configuration (pubkey-validity proof), deposit into encrypted
   balance, apply-pending, withdraw (equality + range proofs). Amounts on-chain
   are ciphertext.
3. **Escrow-side readiness** (already in the program): `CONFIDENTIAL_CAPABLE`
   mint flag recorded per deal; amounts kept raw u64 everywhere; `version` byte
   + 64 reserved bytes in `Deal` for per-party ElGamal keys; events carry
   amounts as u64 fields with room for a future commitment variant.

## Proof-verification costs (from the native program, per instruction)

| Proof | CU |
| --- | --- |
| Pubkey validity | 2,600 |
| Zero ciphertext | 6,000 |
| Ciphertext-commitment equality | 6,400 |
| Grouped-ciphertext validity (2 handles) | 6,400 |
| Batched range proof u64 | 111,000 |
| Batched range proof u128 | 200,000 |

A full confidential transfer needs equality + validity + range proofs — the
range proof alone is ~55% of a default 200k-CU transaction, which is why
transfers split proofs across context-state accounts over multiple
transactions.

## What confidential escrow (phase 2) actually requires

The prototype's accounting assumes public u64 amounts: deposits credited by
vault balance delta, milestone amounts compared to approvals, payouts split by
subtraction. With encrypted balances the program cannot read any of these, so:

1. **Vault becomes a confidential token account** (same ATA — CT lives in the
   account's extension area). Deposits arrive as ciphertext; "fully funded" can
   no longer be checked by delta. Options: (a) parties deposit publicly and the
   *vault* converts to confidential (amounts public at funding, private while
   locked); (b) full ciphertext deposits with an equality proof that the
   deposit matches `total_amount` — leaks the total to the program anyway.
   Option (a) is the pragmatic first step: hide the balance during the deal,
   not the deal size at formation.
2. **Milestone releases in ciphertext** need the program to verify a
   ciphertext-commitment equality proof against the (public) milestone amount,
   or per-milestone encrypted amounts with range proofs that they sum to the
   total — the latter hides milestone structure but is the expensive path.
3. **Splits (auto-split, mutual settlement)** on encrypted balances need
   percentage-with-cap proofs (6,500 CU) or two equality proofs.
4. **Key management:** each party holds an ElGamal keypair + AES key derived
   from their wallet (the CLI derives from a wallet signature). Recovery
   signers cannot decrypt — court-order execution on encrypted balances
   releases ciphertext the recipient can claim; amounts stay hidden from the
   recovery signers themselves.
5. **Auditor/selective disclosure:** CT supports an auditor ElGamal key per
   mint; per-deal auditor keys (courts, regulators) would use the same
   mechanism — aligns with the concept doc's "selective disclosure" promise.

Estimated scope: a redesign of `deposit`/`release`/`resolve` plus a client
proof pipeline — weeks, after the product wedge is validated. Ecosystem usage
of CT is still near zero (re-enabled June 2026), so being early here is a
differentiator but also means thin tooling (Rust-only, no JS).

## Status

Summary: escrow today, privacy-ready by construction — real ZK proofs verify
in the test suite and on devnet. Phase 2 (option (a): confidential-while-locked
vault) is deferred until there is concrete integrator demand.

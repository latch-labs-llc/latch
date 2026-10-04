# Latch agreement format — specification v0.1

Status: **v0.1 (draft)** · Documents: **template v1**, exactly as shipped in
[`@latch-labs/checkout`](clients/ts/packages/checkout/src/agreement.ts) 0.1.0
· Program: `BT8tr7YXYLQPsLMq3qCipY1fBKyjT3gmGQB5amV2v5yv` (Solana devnet)

This document specifies the deterministic, self-verifying agreement format
whose 32-byte SHA-256 digest is stored on-chain as a Latch deal's
`terms_hash`. Any client that implements this spec regenerates the agreement
byte-for-byte from the deal's parameters and verifies it against the chain —
no server, no Latch-operated service, no trust in the client that produced it.

The key words MUST, MUST NOT, SHOULD, and MAY are to be interpreted as
described in RFC 2119.

## 1. Design goals

1. **Self-verifying.** The agreement text is a pure function of the deal's
   on-chain fields plus one short free-text subject. Given those inputs, any
   implementation produces identical bytes and therefore an identical hash.
2. **Opaque to the program.** The on-chain program stores and echoes
   `terms_hash` as 32 opaque bytes. It never parses agreement text. New
   template versions require **no program changes**.
3. **Nothing private on-chain.** Only the digest goes on-chain. The text —
   including the subject line — stays with the parties.
4. **Human-readable execution record.** Section 7 of the template states the
   electronic-execution consent; each party's `sign_terms` transaction is
   their signature over a deal account that carries this digest.

## 2. On-chain binding

| Where | What |
|---|---|
| `create_deal` instruction | Takes `terms_hash: [u8; 32]` as a parameter; stores it on the `Deal` account. |
| `Deal` account | `terms_hash` is immutable after creation. Once all parties have signed, every fund-flow parameter is frozen by the program. |
| `sign_terms` instruction | Each party signs the deal in the Draft state; the emitted `TermsSigned` event echoes `terms_hash`, the party, and the timestamp. |
| Events | `DealCreated` and `TermsSigned` both carry `terms_hash` (CPI events — decode from inner instructions, not logs). |

Verification therefore reduces to: rebuild the text (§4), hash it (§5),
compare with the `terms_hash` on the deal account or in its events.

## 3. Inputs

The template is parameterised by `AgreementParams`:

| Field | Type | Rendered as | Constraints (v1) |
|---|---|---|---|
| `deal` | pubkey | base58 | The deal PDA. |
| `dealId` | u64 | decimal string | As passed to `create_deal`. |
| `buyer` | pubkey | base58 | Payer party. |
| `seller` | pubkey | base58 | Payee party. |
| `mint` | pubkey | base58 | SPL mint of the escrowed token. |
| `decimals` | u8 | — (drives amount formatting) | MUST be ≥ 1 (see §6.2). |
| `milestoneAmounts` | u64[] | one list item each, UI units | ≥ 1 entry; raw base units in, fixed-point out. |
| `ruleName` | string | verbatim | One of the program's deadlock rules: `TimeoutRelease`, `TimeoutRefund`, `AutoSplit`, `TieBreaker`, `LongSunset`, `TrueDeadlock`. |
| `timerModeName` | string | verbatim | `FromDeadlock` or `FromActivation`. |
| `timeoutSecs` | u64 | decimal string | As fixed at formation. |
| `recoverySigners` | pubkey[] | base58, one per line | The party-chosen recovery set. |
| `recoveryThreshold` | u8 | decimal | M of the M-of-N recovery set. |
| `recoveryDelaySecs` | u64 | decimal string | On-chain notice period. |
| `subject` | string | verbatim | Free text. MUST be a single line (no `\n` or `\r`); SHOULD be ≤ 90 characters. |

Producers MUST render enum names and numbers exactly as the deal was created
— the text is evidence of what was formed, so a mismatch is a construction
bug, not a formatting choice.

## 4. Canonical text construction

The canonical form is the exact output of the reference implementation,
[`buildAgreement`](clients/ts/packages/checkout/src/agreement.ts):

- Encoding is **UTF-8**. Implementations MUST NOT apply Unicode
  normalization (no NFC/NFD rewriting of the subject): the bytes the
  producer hashed are the bytes the verifier must hash.
- Line endings are **LF** (`\n`) only. The text ends with a single trailing
  LF.
- Public keys render as base58 (standard Solana encoding), integers as plain
  decimal strings with no separators.
- The first line identifies the template version:
  `LATCH ESCROW AGREEMENT (template v1 — devnet demonstration)`.
  Parsers and verifiers MUST treat the first line as the version marker.
- Milestones render one per line as
  `   1. <amount> tokens` (three leading spaces, 1-based index, period,
  space, fixed-point amount per §6.1).
- Recovery signers render one per line with three leading spaces.

Vector 1 in §7 reproduces a complete canonical text. The reference
implementation's template literal is normative for v1; where prose and
implementation disagree, the implementation's bytes win and this document
has a bug to fix.

## 5. Hashing

`terms_hash = SHA-256(UTF-8 bytes of the canonical text)` — 32 bytes,
exactly as produced by `crypto.subtle.digest("SHA-256", bytes)`. No salt, no
length prefix, no domain separator in v1. The digest is passed to
`create_deal` and is immutable thereafter.

## 6. Formatting rules and v1 limits

### 6.1 Amount formatting

Raw base-unit amounts render as fixed-point decimal with exactly `decimals`
fractional digits and at least one integer digit:

```
ui(raw, decimals) = pad(raw, decimals + 1) split at decimals from the right
```

Examples: `25000000` with 6 decimals → `25.000000`; `1` with 9 decimals →
`0.000000001`. No thousands separators, no trailing-zero trimming. The
"CONSIDERATION" total is the checked sum of all milestone amounts, formatted
the same way.

### 6.2 Known v1 limits

- `decimals = 0` produces malformed output (a leading `.`), so v1 is
  restricted to mints with ≥ 1 decimal. A future template version will lift
  this.
- The token renders as the generic word "tokens"; the mint address, not a
  symbol, is the identity of the asset.
- The banner marks v1 as a devnet demonstration that creates no legal
  obligations. A mainnet template version will carry different language and
  therefore a different first line.
- v1 fixes exactly two parties (buyer/seller) and one resolution rule
  section. Multi-party and arbiter-named templates are future versions.

## 7. Transport of the subject

Every input except `subject` is recoverable from the deal account. The
subject travels out of band — the reference app carries it in the share
link's `s=` query parameter (URL-encoded) and the checkout SDK stores it in
the session. Integrity does not depend on the transport: a tampered subject
produces a different hash and verification fails. Loss of the subject means
the text cannot be regenerated, so clients SHOULD preserve it with the deal
record; parties SHOULD keep their own copy of the full text (it is their
agreement).

## 8. Verification procedure

1. Fetch the deal account; read `terms_hash`, parties, mint, milestone
   amounts, rule, timer mode, timeout, and the recovery set.
2. Obtain the subject (share link, session record, or the counterparty).
3. Rebuild the text with `buildAgreement` or any conforming implementation.
4. SHA-256 the UTF-8 bytes and compare all 32 bytes against `terms_hash`.
5. Optionally confirm execution: decode the `TermsSigned` CPI events and
   check each party's signature transaction echoes the same digest.

The reference app performs steps 1–4 live on the deal page and prints the
digest-match in the certificate's verification walkthrough.

## 9. Test vectors

All vectors use deterministic keys — the base58 encoding of 32 repeated
bytes — so any implementation can reproduce them without key material:

| Byte | Pubkey |
|---|---|
| `0x01` ×32 | `4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi` (buyer) |
| `0x02` ×32 | `8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR` (seller) |
| `0x03` ×32 | `CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8` (mint) |
| `0x04` ×32 | `GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq` (deal) |
| `0x05` ×32 | `LbUiWL3xVV8hTFYBVdbTNrpDo41NKS6o3LHHuDzjfcY` (3rd recovery signer) |

### Vector 1 — single milestone (protected-send preset shape)

```
dealId            1759600000
decimals          6
milestoneAmounts  [25000000]
rule / timer      TimeoutRelease / FromActivation, timeout 604800
recovery          2-of-2 [buyer, seller], delay 259200
subject           Protected payment: as agreed
```

Expected: **1779 bytes**, SHA-256
`64ec55990f1b599718dfc68a7bb3562e5c125d848aef9537be99b402f24f2c98`.

Full canonical text (between the fence lines; every line ends with LF,
including the last):

```
LATCH ESCROW AGREEMENT (template v1 — devnet demonstration)

This is a demonstration on Solana devnet. It creates no legal obligations
and involves no real funds.

1. SUBJECT. Protected payment: as agreed

2. PARTIES.
   Buyer (payer):  4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi
   Seller (payee): 8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR

3. CONSIDERATION AND ESCROW. Buyer shall deposit 25.000000 tokens
   (mint CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8) into the split-control escrow vault of
   on-chain deal GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq (deal id 1759600000) under program
   BT8tr7YXYLQPsLMq3qCipY1fBKyjT3gmGQB5amV2v5yv on Solana devnet. No party and no third
   party, including any operator, can move escrowed funds unilaterally.

4. MILESTONES. Funds release per milestone upon the required approvals:
   1. 25.000000 tokens

5. RESOLUTION. Rule fixed at formation: TimeoutRelease (timer mode
   FromActivation, timeout 604800 seconds). The parties may
   settle any dispute at any time by jointly signing a payout division.

6. RECOVERY. Recovery signers (2-of-2):
   4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi
   8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR
   Recovery payouts may only be directed to the parties and become
   executable only after a 259200-second on-chain notice period.

7. ELECTRONIC EXECUTION. The parties consent to transact electronically.
   Each party executes this agreement by a Solana transaction, signed by
   their wallet, recording approval of this document's SHA-256 digest on
   the deal account. The on-chain record of digest, signatures, and
   timestamps is the authoritative record of execution.

EXECUTION: by on-chain signature of each party's wallet on deal
GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq.
```

### Vector 2 — three milestones, non-ASCII subject

```
dealId            42
decimals          6
milestoneAmounts  [1000000, 2500000, 500000]
rule / timer      TrueDeadlock / FromDeadlock, timeout 0
recovery          2-of-3 [buyer, seller, 0x05×32], delay 604800
subject           Déjà-vu commission — 3 tranches
```

Expected: **1858 bytes**, SHA-256
`6d6b208037065f002b2f3edc9de17207fcc9faa83eefae0f83531c32e9398cbb`.
(The subject's `é`, `à`, and `—` pin the UTF-8, no-normalization rule.)

### Vector 3 — nine-decimal mint, minimum-unit amount

```
dealId            7
decimals          9
milestoneAmounts  [1]
rule / timer      TimeoutRefund / FromActivation, timeout 86400
recovery          2-of-2 [buyer, seller], delay 259200
subject           Minimum-unit formatting check
```

Expected: **1773 bytes**, SHA-256
`17e1b5923df7a2401ed778fb2d9455e951bcef32d15526ad449c05824b1c27c8`.
(The amount renders as `0.000000001`.)

A conforming implementation MUST reproduce all three digests exactly.

## 10. Versioning

- The **first line** of the text names the template version. Verifiers that
  parse (rather than merely re-hash) MUST check it.
- Template versions are append-only: v1 texts stay verifiable forever
  because the program never interprets the hash.
- Changes that alter even one byte of output for the same inputs require a
  new template version. Prose-only changes to this document do not.
- This specification is versioned independently (v0.1) and will track
  template versions as they ship.

## 11. Reference implementation

- Template + hash: [`clients/ts/packages/checkout/src/agreement.ts`](clients/ts/packages/checkout/src/agreement.ts)
  (`buildAgreement`, `sha256`, `hex`), published as `@latch-labs/checkout`.
- The web app re-exports it from a single source
  ([`app/src/lib/agreement.ts`](app/src/lib/agreement.ts)) — integrators
  SHOULD do the same rather than fork the template text.
- On-chain fields: [`programs/latch/src/state.rs`](programs/latch/src/state.rs)
  (`Deal.terms_hash`),
  [`programs/latch/src/instructions/create_deal.rs`](programs/latch/src/instructions/create_deal.rs),
  [`programs/latch/src/instructions/sign_terms.rs`](programs/latch/src/instructions/sign_terms.rs).

Feedback and independent implementations are welcome — open an issue or PR.

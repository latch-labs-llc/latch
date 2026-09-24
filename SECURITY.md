# Security Policy

Latch is **unaudited, experimental software deployed to Solana devnet only**.
Do not use it with real funds. An independent security review is planned; this
file will be updated when it completes.

## Reporting a vulnerability

Please report vulnerabilities privately — do not open a public issue.

- **Preferred:** GitHub's private vulnerability reporting on this repository
  ("Security" tab → "Report a vulnerability").
- If that is unavailable, contact the maintainer through the repository
  profile and ask for a private channel before sharing details.

We aim to acknowledge reports within 5 business days. Critical issues in any
deployed program will be triaged immediately.

## Scope

In scope: the Anchor program in `programs/latch`, its instruction handlers,
account validation, mint vetting, and state machine; the reference demo and
scripts insofar as they demonstrate unsafe patterns.

Out of scope: third-party dependencies (report upstream), the Solana runtime,
and social engineering.

## Known-risk documentation

`SECURITY_NOTES.md` in this repository lists the design assumptions, trust
boundaries, and accepted risks we already know about — please read it before
reporting, and reference it in your report where relevant.

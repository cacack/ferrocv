# Security Policy

## Reporting a vulnerability

Please report security vulnerabilities **privately** via GitHub's
[security advisory form](https://github.com/cacack/ferrocv/security/advisories/new).
Do **not** file public issues for suspected vulnerabilities.

We aim to acknowledge reports within 72 hours. For confirmed issues,
we aim to publish a fix and a coordinated security advisory within
30 days of acknowledgement.

## Supported versions

`ferrocv` is pre-1.0 and moves fast. Only the latest `0.x` release
line receives security fixes; older `0.x` tags are archived and will
not be patched. Once we release `1.0.0`, this policy will expand to
cover the latest minor line.

| Version    | Supported |
| ---------- | --------- |
| latest 0.x | Yes       |
| older 0.x  | No        |

## Scope

`ferrocv` reads `resume.json`, renders via embedded Typst, and writes
files the user specified. It makes no network calls at runtime — see
[`CONSTITUTION.md`](CONSTITUTION.md) §6 for the full trust model.

In scope:

- Maliciously crafted `resume.json` causing crashes, excessive memory
  use, or reaching outside the Typst sandbox.
- Vendored theme templates under `assets/themes/` mishandling user data
  or escaping Typst's sandbox.
- Dependency vulnerabilities flagged by `cargo-audit` or `cargo-deny`.
- Release-pipeline integrity (workflow tampering, unsigned artifacts).

Out of scope:

- Exploits that require an already-compromised host or a modified
  binary.
- Social engineering or credential theft targeting the user's OS
  account.
- Issues in third-party tools that happen to consume `ferrocv`'s
  output (PDF viewers, browsers, ATS systems).

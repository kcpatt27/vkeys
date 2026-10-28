# Security

## Reporting

Open a private security advisory on the repository (or contact the maintainer
listed on the repository profile). Do not open a public issue for a live secret
or a bypass.

## Threat model — Tier 1

- The vault is encrypted at rest: DPAPI (Windows) or XChaCha20-Poly1305 with a
  32-byte master key held in the OS credential store (macOS Keychain / Linux
  Secret Service).
- The OS-user boundary stops other accounts and disk theft. It does **not** stop
  a process running as the same user: such a process can ask the OS to decrypt,
  and a Windows DPAPI vault can be decrypted without going through `vkeys`.
- vkeys makes accidental exposure hard, not impossible: no `get`, profiles with
  fixed `argv`, a clean child environment, output scrubbing, and an audit log.
  It is not a sandbox.
- Values never appear in output, logs, or the audit. The audit records key
  **names**, status, and token/cost metadata only.

## Threat model — Tier 2 broker

- Binds loopback by default; clients authenticate with a separate broker token.
- The provider key is injected only into the upstream request and is never
  returned to a client.
- Audit records method, route, status, and token counts; never prompts,
  completions, headers, or keys.
- Do not expose the broker port publicly. Use a VPN/tailnet plus the broker token.

## Supply chain

- The Rust dependency set is locked in `rust/Cargo.lock`.
- CI builds and tests on Windows, macOS, and Linux (`.github/workflows/ci.yml`).

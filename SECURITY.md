# Security

## Reporting

Open a private security advisory on the repository (or contact the maintainer
listed on the repository profile). Do not open a public issue for a live secret
or a bypass.

## Threat model - Tier 1

- The vault is encrypted at rest: DPAPI (Windows) or XChaCha20-Poly1305 with a
  32-byte master key held in the OS credential store (macOS Keychain / Linux
  Secret Service).
- The OS-user boundary stops other accounts and disk theft. It does **not** stop
  a process running as the same user: such a process can ask the OS to decrypt,
  and a Windows DPAPI vault can be decrypted without going through `vkeys`.
- On headless Linux without a Secret Service, the master key is a `0600` file
  (`~/.config/vkeys/master.key`). Same-user processes can read it; that is the
  documented trade for unattended operation.
- **Strong boundary:** run the Tier 2 broker as a different OS user (Linux
  `vkeys` system user; Windows SYSTEM via a startup task) with the vault/key
  readable only by that user. The agent's user cannot decrypt; it holds a
  use-only token. See `deploy/raven/` and `deploy/windows/`.
- On Windows, `deploy/windows/install.ps1` is designed to install the broker as
  SYSTEM and ACL `C:\ProgramData\vkeys-broker` to SYSTEM + Administrators; the
  agent gets only a use-only token. It sets `VKEYS_SCHEME=xchacha20poly1305` so
  the SYSTEM task can read a key file the agent cannot. This path is verified on
  a live Windows host for install, boot start, health, ACL, and broker use.
- vkeys makes accidental exposure hard, not impossible: no `get`, profiles with
  fixed `argv`, a clean child environment, output scrubbing, and an audit log.
  It is not a sandbox.
- Values never appear in output, logs, or the audit. The audit records key
  **names**, status, and token/cost metadata only.

## Threat model - Tier 2 broker

- Binds loopback by default; clients authenticate with a separate broker token.
- The provider key is injected only into the upstream request and is never
  returned to a client.
- Audit records method, route, status, and token counts; never prompts,
  completions, headers, or keys.
- Do not expose the broker port publicly. Use a VPN/tailnet plus the broker token.

## Supply chain

- The Rust dependency set is locked in `rust/Cargo.lock`.
- CI is configured to build and test on Windows, macOS, and Linux
  (`.github/workflows/ci.yml`). It is manual-only while the repository is
  private; push and pull-request runs are re-enabled after the public flip.

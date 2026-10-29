# T1 — vkeys on Linux/ARM (Raven)

**Status:** planning. Blocks vkeys-as-keyring-of-record on the always-on host.

Raven is a Raspberry Pi 5, aarch64, Debian 13, no Rust toolchain, no vkeys
binary. README claims Windows/macOS/Linux; only Windows has real use behind it.

## Scope

- Build `vkeys` and `vkeys-broker` for `aarch64-unknown-linux-gnu`.
- Install to `~/.local/bin`, add to PATH for user services.
- Confirm the Linux vault backend (README: XChaCha20-Poly1305 with the master
  key held in the OS keyring). If no Secret Service / keyring daemon exists on
  the Pi, decide: headless keyring file with 0600 + explicit threat note, or
  refuse and require a human-presence unlock.
- Prove on Raven: `init`, `set`, `ls`, `env`, `run`, `audit`, `selfcheck`.

## Acceptance

- `vkeys selfcheck` passes on Raven.
- A profile injects a test key into a child process; `env` output contains a
  `<stored: name>` marker, never the value.
- No plaintext key material on disk outside the vault.
- Install and keyring decision documented in `README.md`.

## Why it matters

Paperclip and other daemons need long-lived secrets on this host. Without a
Linux build, Raven keeps secrets in Paperclip's local encrypted file and vkeys
stays a desktop tool.

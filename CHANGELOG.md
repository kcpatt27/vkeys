# Changelog

## 0.1.0 — unreleased

- **Tier 1**: encrypted vault (Windows DPAPI; macOS Keychain / Linux Secret
  Service with XChaCha20-Poly1305), named profiles with fixed argv, no `get`,
  clean child environment, output scrubbing, audit log.
- **Tier 2**: OpenRouter broker (`vkeys-broker`) — broker-token auth, in-memory
  budget, per-request audit, non-streaming.
- Rust-only; Apache-2.0; CI matrix on Windows, macOS, Linux.
- Python oracle archived at tag `python-oracle-final`.

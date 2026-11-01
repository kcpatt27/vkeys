# 010 — Portable vault + key export/import

Status: open · Priority: P2 · Area: storage

## Context
Master key is OS-bound (DPAPI/Keychain/Secret Service/key file). A portable vault
needs a key export/import path (passphrase or age recipient).

## Acceptance
- [ ] Design ADR; export/import with a passphrase; no plaintext key.
- [ ] Move a vault between Windows/Linux/macOS; docs.

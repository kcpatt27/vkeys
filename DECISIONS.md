# Decisions

## ADR-0001 — vkeys: name, scope, and the no-read rule

**Status:** accepted (2026-10-19).

**Context.** Keys leak through env vars, `.env` files, and pasted-in-chat
secrets. The requirement is two-sided: an agent may *use* a key, but must not be
able to *see* it, and there should be no easy command that reveals one.

**Decision.**

1. **Name `vkeys`** ("virtual keys"). `keys` is too generic (collides with npm
   helpers and future platform tooling); `secret`/`vault` are generic too. For a
   local `uv tool` only PATH matters.
2. **Store:** `C:\ProgramData\secrets\vault.bin` on Windows, DPAPI **CurrentUser**
   scope (not LocalMachine). macOS/Linux use the platform data dir and
   XChaCha20-Poly1305 (ADR-0003). Encrypted per value; metadata plaintext.
3. **No `get`.** Values are write-only. Seeing one requires making a new one.
4. **Profiles, not generic injection.** `keys` in a profile map env names to
   store names; `cmd` is fixed; `agent-ok` gates non-interactive use.
5. **Clean env + output scrub + audit.** The child gets only allowlisted base
   vars plus the profile's keys; its stdout/stderr is scrubbed of injected
   values before reaching the caller; every run is logged.

**Consequences.** Tier 1 makes accidental exposure hard. It does **not** stop a
determined same-user process: DPAPI-CurrentUser is decryptable by anything
running as the user. Tier 2 is `vkeys-broker` (OpenRouter only, 2026-10-28): the
provider key stays in the broker and never enters the client. It is still not a
general secret server.

**Rejected.** `secret get` (violates the rule), `export-env` (dumps values into
a shell), LocalMachine DPAPI (any process/SYSTEM can decrypt), hashing instead of
encryption (can't inject the real key).

## ADR-0002 — Rust-only main; oracle archived; portable vault later

**Status:** accepted (2026-10-28).

**Decision.**

1. Rust is the only implementation on `main`. The Python implementation was
   removed and is preserved at tag `python-oracle-final` (commit `80847db`);
   `PORT.md` is the historical record.
2. A **portable vault file** is a planned direction: one encrypted file format
   across OSes, with the master key wrapped by the platform keychain (Windows
   DPAPI / macOS Keychain / Linux Secret Service). Not built yet.
3. **Per-client broker budgets** are wanted later, but require per-client tokens
   or identities first; deferred.
4. macOS and Linux native store backends landed 2026-10-28 (ADR-0003); the broker
   (`tiny_http`/`ureq`) is already portable.

**Consequences.** `main` is Rust-only; contributors do not maintain two
implementations. The archived tag remains the reference for the Python parity
harness.

## ADR-0003 — Cross-platform storage

**Status:** accepted (2026-10-28).

**Decision.** Windows keeps per-value DPAPI (`scheme = "dpapi"`) so existing
vaults keep working. macOS and Linux use XChaCha20-Poly1305 (`scheme =
"xchacha20poly1305"`) with a 32-byte master key held in the OS credential store
(Keychain / Secret Service) through the `keyring` crate. Entries carry a
`scheme`; a missing scheme means `dpapi`. A DPAPI entry opened on macOS/Linux
errors with "open it on Windows to migrate".

**Consequences.** The same binary builds on all three platforms; the vault file
is portable once the master key is present on the target machine. Master-key
export/import (the last step for a truly portable vault) is still future.
DPAPI entries are not readable off Windows without a migration step.

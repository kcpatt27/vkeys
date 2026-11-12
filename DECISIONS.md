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

## ADR-0004 - VeraCrypt recovery material stays out of vkeys

**Status:** accepted (2026-11-01).

**Context.** A 5 TB offsite backup drive is encrypted with VeraCrypt as a
whole-volume standard volume. Recovery depends on a password a human can read, a
volume header backup stored off the volume, and remembering the cipher, hash,
PIM and keyfile choices. The question was whether vkeys should hold this
material, possibly behind a new feature.

**Decision.** No. vkeys remains a write-only key store for injecting values into
fixed-argv processes.

1. **Retrieval is required.** Recovery is a human typing a password into a mount
   dialog. `get` is deliberately absent (ADR-0001 item 3); adding a reveal path
   would remove the property the tool exists to provide.
2. **The vault is machine-bound.** Windows entries are DPAPI CurrentUser
   (ADR-0003), so the vault dies with this account and this machine. Recovery
   material must survive exactly that loss.
3. **The consumer is wrong.** Values are injected into child processes with
   fixed argv. A human mounting a volume is not a child process.
4. **The payload is wrong.** A volume header backup is a small binary artifact;
   the store holds short hidden-paste strings.

**Consequences.**
- The password lives in the password manager.
- The header backup lives on a second drive plus one off-machine copy.
- A manifest records volume label and size, cipher, hash, PIM, keyfile use,
  creation date, VeraCrypt version, the header backup SHA-256, and a pointer to
  the password entry - never the password itself.
- Durability and findability are the requirements, not secrecy: the header
  backup is already ciphertext.

**Rejected.**
- A `reveal` / `unseal` subcommand - breaks ADR-0001.
- Storing the header as a vault value - binary payload, and still unretrievable.
- Recording only the password - loses the PIM / keyfile / cipher state that can
  make a volume permanently unmountable.

## ADR-0005 - Transport is dropped for now; target identity is the later direction

**Status:** accepted (2026-11-11).

**Context.** The unmerged `feat/transport-age-bundle` branch implements an age +
scrypt vault export/import in human-vault scope. The 2026-11-10 audit found that
it does not enforce the service user, the TTY gate is bypassable by a same-user
PTY or ConPTY, and it lacks target binding, replay protection, replace
confirmation, and actor/host/hash audit. The privileged transport ticket is
therefore not satisfied. The transport analysis asked for a decision before any
redo.

**Decision.**

1. **0.2 ships without transport.** Choose option B: bootstrap uses the Raven and
   Windows installers; migration uses a controlled file copy or the portable
   vault work in 010.
2. **Do not merge `feat/transport-age-bundle` as 001.** Its reusable parts
   (atomic write, size cap, path checks, negative tests) can seed a later design.
3. **Target identity (option D) is the later direction.** Revisit it only if
   target binding and replay protection become real requirements. D means an age
   recipient public key, a signed manifest, and a one-time import token.
4. **Do not claim transport exists, that 001 is satisfied, or that the TTY gate
   is an agent boundary.** 001 is deferred; 018 is closed by this ADR.
5. **Preserve the old work.** `feat/transport-age-bundle` and the dirty
   `wip/transport-push` worktree stay unmerged. Do not delete either without a
   separate decision.

**Consequences.** 0.2 has less surface area. Offline transfer remains manual
until D. The installers already cover the bootstrap case. The ticket queue shows
001 as deferred and 018 as done.

**Rejected.** A as-is (does not meet 001 and the boundary claim would be false);
C now (no named-key offline need established); D now (too much for 0.2).

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
2. **Store:** `C:\ProgramData\secrets\vault.bin`, DPAPI **CurrentUser** scope
   (not LocalMachine). Encrypted per value; metadata plaintext.
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

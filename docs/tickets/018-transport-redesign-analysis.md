# 018 - transport-to: structured analysis before a redo

Status: **done - decision B (2026-11-11)**. Priority: P1 (security), not a 0.1
blocker. ADR-0005: no transport in 0.1 or 0.2; target identity (D) may be
revisited later.

## Why

`feat/transport-age-bundle` implements an age-encrypted vault export/import:
`vkeys transport export` and `import`, hidden passphrase, atomic writes, 48
`#[test]` functions (47 pass on Windows). It is in human-vault scope: it does not
enforce the service user and is not an agent boundary. It is **not in 0.1** - the
release prompt says "do not merge any `feat/transport-*` branch; 0.1 ships
without transport."

## Audit 2026-11-10
The branch has good negative tests and atomic writes, but the TTY gate is
bypassable by a same-user PTY or ConPTY, so it is not an agent boundary. There is
no ADR, no public-key recipient, no target binding, no replay or stale-bundle
protection, and no actor/host/hash audit. The old agent-callable push exists only
as uncommitted lines in the dirty `wip/transport-push` worktree.

Decision 2026-11-11: **B**. ADR-0005 records it. Drop transport for 0.2; the
installers cover bootstrap and 010 covers portable vault/migration. D is the
later direction if target binding and replay protection become real
requirements. Do not merge the age branch as 001.

The concern is not whether the code works. It is whether the shape is right.
"Move a vault" and "seed one provider key into a new service user" are different
jobs. If the main need is the latter, the installer prompt is simpler and safer,
and transport should not be on the critical path. This ticket decides that before
any rewrite.

The 48 test functions are not all compiled on every platform: 3 are gated
(`#[cfg(unix)]` on `key_file_is_0600`, `#[cfg(windows)]` on
`dry_run_preview_writes_nothing` and `vault_export_import_roundtrip`). A Windows
run therefore executes 47 and a Linux or macOS run executes 46.

## Questions to settle

1. **Primary use case.** Bootstrap a new host, migrate an existing vault, move a
   single named key, disaster recovery, or phone/USB handoff? Pick one primary;
   the others become explicit non-goals or later cases.
2. **Unit of transfer.** Whole vault, named keys, or a profile. A named-key
   export is smaller and easier to reason about; a whole-vault move preserves
   metadata but carries everything.
3. **Who may run it.** Service user only (Linux `vkeys` / Windows SYSTEM), never
   the agent user. How is that enforced on each platform, and how is it tested?
4. **Crypto and custody.** age passphrase vs recipient public key; where the
   passphrase lives; what happens when it is lost; whether the bundle can be
   replayed; whether the target key is pinned.
5. **Channel.** Local file, USB, SSH/SCP, phone. No network listener, no stdout
   values, no plaintext temp files. What does a failed transfer leave behind?
6. **Import semantics.** Merge vs replace; name conflicts; partial writes;
   atomicity; idempotency; whether import can overwrite an existing key without
   an explicit flag.
7. **Evidence.** Export/import audit events: names only, counts, hashes, actor,
   timestamp, target host. Never values. Where is it recorded?
8. **Platform matrix.** Linux service user, Windows SYSTEM, macOS. Which paths
   are real, which are aspirational? What is the CI matrix?
9. **Failure and rollback.** Wrong passphrase, truncated bundle, wrong target,
   stale bundle, disk full, interrupted import. What is the recovery procedure?
10. **Relationship to other work.** The broker bootstrap (003), portable vault
    (010), vault locking (012), and the 0.1/0.2 split. Does transport replace any
    of them, or sit beside them?

## Options to compare (at least)

- **A. Keep the current age bundle.** Whole-vault or named-key export, service
  user only, no agent path. The 2026-11-10 audit says the branch does not meet
  this; treat A as a rewrite, not a merge.
- **B. Drop transport; keep installer/import.** For bootstrap, the provider-key
  prompt or `VKEYS_PROVIDER_KEY_FILE` is enough. For migration, use the service
  user plus a controlled file copy.
- **C. Named-key transport only.** Export/import one key at a time, no whole
  vault; simpler conflicts, clearer audit.
- **D. Redesign around a target identity.** Recipient public key (age) + signed
  manifest + one-time import token; strongest, most work.

## Deliverable

A decision document (ADR) that states:

- the chosen use case and why the others are out of scope;
- the data model (whole vault vs named keys) and import semantics;
- the threat model and who can run it;
- the CLI shape and the exact user-visible prompts;
- a test matrix (success, wrong passphrase, truncated, conflict, replay,
  interrupted, wrong target);
- a recommendation: keep, narrow, or redo.

No implementation until that document is accepted.

## Done when

One use case, one data model, one threat model, one CLI shape, and a test matrix
are agreed, with a recorded decision to keep, narrow, or redo. The 0.1 release is
not blocked by this ticket.

## Links

- `001-privileged-transport-bundle.md` - the original acceptance.
- `002-remove-transport-push.md` - why the old agent-callable push was removed.
- `003-raven-broker-golive.md` - the bootstrap case that may make transport
  unnecessary.
- `010-portable-vault.md` - overlapping export/import work.
- `feat/transport-age-bundle` @ `1c88c8e` - current implementation, 48 test
  functions, not merged.

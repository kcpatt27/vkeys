# T4 — vault transport between machines

**Status:** planning. The gap that bites on every OS migration.

## Problem

The vault is encrypted under an OS backend (DPAPI on Windows, Keychain on macOS,
keyring or a master-key file on Linux). That is correct for a local store and
wrong for moving a life's worth of keys to a new machine. Today the only path is
to re-enter every value by hand. At ten keys that is annoying; at a hundred it
is a reason not to adopt the tool.

The design constraint is the one that makes vkeys good: **there is no `get`.**
A transport feature must not become a `get` with extra steps.

## Proposal

Two commands, one encrypted artefact, no plaintext on stdout:

```
vkeys export --recipient <age-public-key> --out keys.age
vkeys import keys.age
```

- `export` decrypts each value in memory, re-encrypts the bundle to an
  **age/X25519 recipient** (or a passphrase with argon2id), writes one file,
  and prints only: count, names, recipient fingerprint, expiry.
- `import` decrypts the bundle, merges into the local vault, refuses name
  collisions unless `--replace`, and prints only names and counts.
- The bundle is single-use by convention: `export` records a transfer ID and
  `import` records the same ID; an expired or reused bundle is refused.
- Human-only: `agent-ok` does not apply; export/import require a TTY and an
  explicit `--yes`.
- Audit both sides: name, recipient fingerprint, count, transfer ID, time.

## Why age and not "just copy vault.bin"

`vault.bin` is bound to the source OS identity. Copying it to another machine
fails to decrypt, or worse, tempts a future change that stores the backend key
next to it. A transport bundle is portable *by construction* and expires.

## Acceptance

1. Export 5 test keys on Windows, import on Linux, all 5 inject correctly.
2. The bundle cannot be read without the recipient key; a wrong key fails
   cleanly.
3. No value is printed at any point; `vkeys ls` shows names only.
4. A replayed bundle is refused; an expired bundle is refused.
5. A restore rehearsal on a clean profile matches T3.

## Open questions

- Recipient key custody: where does the age private key live, and is it the same
  key as the backup key?
- Should the bundle include profile definitions, or keys only?
- Is a passphrase-only mode acceptable for a machine with no SSH key yet?

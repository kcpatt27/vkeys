# T3 — vault backup, restore and rotation

**Status:** planning. If vkeys becomes the keyring of record, its loss is the
catastrophic one.

## Scope

- **Backup:** what is sufficient to restore on a clean machine? `vault.bin`
  alone is useless without the backend master key (DPAPI on Windows, keyring on
  Linux). Document the pair, and where each copy lives.
- **Restore rehearsal:** restore onto a fresh profile/machine, unlock, list, and
  prove a stored key still injects. Record the checklist and the exact failure
  modes (wrong OS user, missing keyring entry, vault version mismatch).
- **Rotation:** rotate a stored key without downtime; what to do when a key is
  suspected exposed; how audit proves the rotation.
- **Loss:** written procedure for "master key lost" — what is recoverable, what
  is not, and which downstream secrets must be reissued.

## Acceptance

- A restore rehearsal from backup onto a fresh profile, with transcript.
- A rotation rehearsal with no service downtime beyond a restart.
- A one-page runbook in `docs/`, linked from `README.md`.
- Raven's nightly backup timer includes the vault and its unlock material in the
  same set as the Paperclip master key it protects.

## Why it matters

An unrecoverable keyring is worse than no keyring: it centralises failure. The
backup is part of the product, not an ops afterthought.

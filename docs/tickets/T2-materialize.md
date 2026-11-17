# T2 — `vkeys materialize` for daemons

**Status:** planning. This is the primitive that lets vkeys serve services without
becoming a `get`.

Paperclip reads its secrets master key as a **file path**
(`secrets.localEncrypted.keyFilePath`). vkeys deliberately has no `get`, so
today it cannot hand a daemon a key at all.

## Proposal

```
vkeys materialize NAME --out FILE [--mode 600] [--force]
```

- Writes the value once to `FILE`, with the given mode, owner unchanged.
- Refuses if `FILE` exists unless `--force`.
- Records an audit event: name, target path, mode, time, exit.
- Zeroes buffers; never prints the value.
- `--ttl` optional: remove the file after N seconds? (decide: probably no; a
  daemon needs it at start, and a timer deleting it later is a new failure mode.)

## Acceptance

- A profile-free command places a key at a path a daemon can read.
- Audit shows the materialize event with the target path.
- Overwrite refused without `--force`; refusal is audited.
- `vkeys ls` unchanged (no values, no fingerprints).

## Why it matters

This is what lets vkeys hold the Paperclip `master.key` and what makes restore
possible from a vault-only backup. It is also the difference between "keyring
for humans" and "keyring for a human and their AI".

# T4 — vault transport between machines

**Status:** design revised 2026-11-01 after the board refinement. Pairing is
removed from v1. The whole feature is **one-shot**: `vkeys export` writes a
bundle, and `vkeys sync <bundle>` imports it. The same sync command covers the
first migration and every later top-up. Revocation stays, as a local registry
of recipient public keys.

This revision supersedes the earlier pairing/epoch design in this ticket. The
superseded parts are named explicitly below so the change is auditable.

## Problem

The vault is encrypted under an OS backend (DPAPI on Windows, Keychain on macOS,
keyring or a master-key file on Linux). That is correct for a local store and
wrong for moving a life's worth of keys to a new machine. Today the only path is
to re-enter every value by hand. At ten keys that is annoying; at a hundred it
is a reason not to adopt the tool.

The design constraint is the one that makes vkeys good: **there is no `get`.**
A transport feature must not become a `get` with extra steps.

## Design intent (board, 2026-11-01, refined)

1. The primary user is an **agent**, not a human reading values. Transport must
   end in a profile the agent can run, never a value on stdout.
2. Real cases are **Windows → Pi** and **desktop ↔ laptop**. Both are one-shot.
   There is no separate pairing protocol in v1. One command does first-time and
   incremental import: `vkeys sync <bundle>`.
3. Revocation is the one multi-device concept kept. It is a device registry of
   recipient public keys; bundles are encrypted only to active devices.
4. The "AI saw the key, rotate it" failure is a design target: agents get
   profiles, never generic env. A profile runs one fixed child, output is
   scrubbed, and audit records profile, time, and exit.

Consequences:

- The success test is not "the file arrived"; it is "an agent profile on the
  target runs and the child process receives the key" — with no value shown.
- Export may stay whole-vault in v1. Delta export is an optimisation, not a
  requirement.
- `sync` is idempotent and never overwrites. A conflicting value requires an
  explicit `--replace <name>`.
- Revocation is not retroactive. Old bundles stay readable by a revoked device;
  the remedy for a lost device is rotating the affected keys.

## What changed from the first draft

- **Removed:** `pair invite` / `pair accept`, pairing SAS, signed device cards,
  epoch keys `K_e`, signed manifests, `sync-agent-ok`, vector-clock `rev`, and
  the hard "replayed transfer ID is refused" rule.
- **Added:** `vkeys device init|add|ls|revoke`, `vkeys sync <bundle>`, per-name
  conflict semantics (`--replace <name>`), and an explicit profile-injection
  acceptance test.
- **Kept:** age/X25519 encryption, expiry, transfer IDs, collision safety,
  audit on both sides, human-only TTY gate, and the no-values-on-stdout rule.

The earlier draft assumed a live peer protocol. The board's refinement makes
desktop ↔ laptop one-shot too, so the trust machinery that only pairing needed
is gone. Revocation survives because it only needs a list of recipient public
keys at export time.

## Who runs what

| Actor | May do | May never do |
| --- | --- | --- |
| Human at a TTY | `device init/add/revoke`, `export`, `sync`, authorize `--replace <name>`, edit profiles | — |
| Agent | run an `agent-ok` profile, read metadata (`ls`, `device ls --json`, `transport status --json`, `sync --dry-run --json`) | see a value, hold a device private key, add/revoke a device, export, sync, edit a profile |

"Primary user is an agent" means the **end artefact is agent-consumable**, not
that the agent is the authority for moving key material. Moving keys and
changing trust stay human-authorized: `export`, `sync`, `device add`, and
`device revoke` require a TTY and an explicit `--yes`. Only the profile is
agent-runnable. This preserves the existing "human-only, TTY required" gate
without making a human read values.

An agent is also not given a generic `env` or `run -- <argv>` path. It gets a
named profile whose command is fixed by a human.

## Device registry and revocation

A device is an age X25519 keypair plus a label. There is no handshake: the
public recipient is copied out of band and registered by a human.

```
vkeys device init --name <label> [--json]
vkeys device recipient [--json]
vkeys device add <age1…> --name <label> --yes
vkeys device ls [--json]
vkeys device revoke <fingerprint|label> --yes
```

- `device init` creates the keypair and stores the private key in the OS
  credential store (DPAPI / Keychain / Secret Service / T3's documented file
  fallback). It never leaves the machine and is never written into a bundle,
  a repo file, or a log.
- `device recipient` prints only the public `age1…` recipient and its
  fingerprint. That is metadata and safe for an agent to read.
- `device add` registers a **public recipient** copied from another machine.
  This is the manual replacement for pairing. Human-only, `--yes`, audited.
- `device ls` shows label, fingerprint, added time, and `active`/`revoked`.
  It never shows key material.
- `device revoke` marks the registry entry inactive. Export reads the registry
  at export time, so a revoked recipient is not in the next bundle's age
  header. Human-only, `--yes`, audited.

Revocation is **local to the exporting vault**. With no pairing/sync protocol
there is no automatic global revocation: a device must be revoked on every
machine that exports bundles. That is an honest limit of one-shot, and the
runbook says so. The remedy for a lost device is to revoke it on every source
and rotate the keys it could read.

### What revocation does and does not do

- A bundle exported **after** revocation is encrypted only to the remaining
  active recipients. The revoked device's private key cannot open it; decryption
  fails cleanly and the refusal is audited on that device.
- A bundle exported **before** revocation, or already copied to the device,
  remains readable. Revocation is not retroactive and cannot erase a file.
- Rotating the affected values (existing `set`/`rotate`) is the real remedy.
  Revocation only stops future transport to that device.

## Export

```
vkeys export --out keys.age [--recipient <fingerprint|label>]... \
             [--name NAME]... [--expires 24h] [--yes] [--json]
```

- By default the bundle is encrypted to **every active device in the registry**.
  `--recipient` restricts it to one or more registered active devices, by
  fingerprint or label. A raw `age1…` value is not accepted here; it must be
  registered with `device add` first. This is what makes "bundles encrypted only
  to active devices" true.
- Export refuses if the registry has no active recipients. There is no
  plaintext or passphrase-only export in v1; a passphrase cannot be revoked per
  device.
- Export decrypts each value in memory, re-encrypts the bundle to the selected
  age recipients, writes one file atomically, zeroes buffers, and prints only
  metadata: count, names, recipient fingerprints, expiry, transfer ID.
- Whole-vault is the default. `--name` selects a subset. Delta export is not
  required in v1.
- `--out -` is refused. There is no stdout path for key material.
- `--json` emits **metadata only** (names, counts, fingerprints, IDs, expiry).
  Values are never fields in that JSON.
- Expiry defaults to 24 hours. `created_at` / `expires_at` are UTC. `sync`
  refuses after `expires_at` before any merge.

## Sync / import

```
vkeys sync <bundle> [--replace <name>]... [--dry-run] [--yes] [--json]
vkeys import <bundle> ...     # alias of sync
```

`sync` is the only import path and is safe to run repeatedly. Per-name
semantics:

- **Missing locally:** add the value.
- **Present with the identical value:** no-op.
- **Present with a different value:** report `conflict` and leave the local
  value untouched. The command exits non-zero (`2`) because a human decision is
  still needed.
- **`--replace <name>`** (repeatable) authorizes overwriting exactly that name.
  It never authorizes a wildcard or a silent last-writer-wins. Every
  replacement is audited by name.
- Additions and authorized replacements are applied in **one atomic vault
  write**. Unresolved conflicts block only their own name, not the safe
  additions. There is no partial vault on I/O failure.
- Re-running the same bundle after a clean first run reports
  `added: 0, replaced: 0, identical: N` and changes nothing.
- `--dry-run` performs decryption and comparison but writes nothing; it reports
  names, counts, and conflicts only.
- Expired bundle, wrong recipient key, or a tampered bundle is refused before
  any write. `--json` is metadata only.
- `sync` requires a TTY and `--yes`, like `export` and `device` trust changes.

The board's "never overwrite" rule is stronger than an atomic-abort rule: a
conflict can never destroy a local value, and safe missing keys still arrive.
That is why v1 does not need a separate conflict-resolution command.

### Transfer ledger

- `transfers.jsonl` sits beside `vault.bin` (`0600`). Export appends
  `{transfer_id, direction: out, recipients, count, content_sha256, time}`;
  sync appends `{transfer_id, direction: in, source_fp, count, content_sha256,
  time, added, identical, conflicts, replaced}`.
- A transfer ID seen before with the **same content hash** is an idempotent
  repeat: report the previous outcome and apply the normal per-name rules.
  Running `sync` twice adds nothing the second time.
- A transfer ID reused with a **different content hash** is refused: that is
  tampering or a broken exporter, not a repeat.
- The ledger is part of the T3 backup set. Restoring a vault without its ledger
  weakens duplicate detection; T3's runbook must say so.
- The earlier draft refused every repeated transfer ID. The board's idempotent
  `sync` requirement replaces that: repeat application cannot overwrite
  anything, so a clean no-op is correct, while hash mismatch still fails hard.

## Bundle format

Outer envelope: age v1, with one X25519 stanza per active recipient. Inner
payload: JSON manifest.

```
format: "vkeys-sync-v1"
transfer_id: <random 128-bit, UUID/ULID>
created_at, expires_at
source: { device_id, label, vkeys_version }
recipients: [{ fingerprint, label }]
entries: [{ name, value, set_at }]
```

The values are plaintext **inside the age envelope**; that is the same trust
boundary as the vault at rest, not a second store. `set_at` is informational
for conflict reports and audit; it is not used for ordering and there is no
vector clock. Adding `rev` back would require a peer protocol this design no
longer has.

`recipients` is metadata for audit and diagnostics. Decryption authority comes
from the age header stanzas, not from this list.

## Output scrubbing and audit

- Both sides append to `audit.jsonl`: event, transfer ID, recipient/source
  fingerprint, count, names, time, result (`ok`, `noop`, `conflict`,
  `replaced`, `refused-expired`, `refused-wrong-key`, `refused-tampered-id`).
  Names only; never values.
- Profile runs append profile name, time, and child exit code. They never append
  the environment or child output.
- Before relaying a child's stdout/stderr, vkeys redacts exact occurrences of
  the values it injected. Scrubbing is best-effort: it cannot catch a value the
  child transforms or encodes. The real control is that the agent never gets a
  generic env or `get` in the first place.
- Tests scan every success and failure path for fake marker values in stdout,
  stderr, `--json`, audit, and error text.

## Profile injection

Transport ends in a profile, not a value:

- Profiles remain local and are not carried in a bundle by default. Commands and
  paths are machine-specific; importing them would be a second, surprising
  migration.
- A profile has a fixed `cmd` and a mapping from imported key names to child
  environment variables. A human creates or edits it; an agent may run it only
  if it is marked `agent-ok`.
- `vkeys run <profile>` injects the values into that one child. There is no
  generic `run -- <argv>` and no `export-env`.
- The acceptance check is that the child proves receipt (for example by
  comparing a hash or writing a marker) without printing the value, and the
  transcript contains only names, counts, and exit status.
- `sync --json` may report which imported names already have a matching profile
  so an agent can tell whether it is ready to run. It does not create or edit a
  profile.

## Why age and not "just copy vault.bin"

`vault.bin` is bound to the source OS identity. Copying it to another machine
fails to decrypt, or worse, tempts a future change that stores the backend key
next to it. A transport bundle is portable by construction and expires.

## Implementation slices

1. `rust/Cargo.toml`: add `age` (X25519, multi-recipient). Reuse `serde_json`
   for the inner manifest and the existing error/audit style.
2. `rust/src/vault.rs`: expose an in-memory `entries()` iterator for export and
   an atomic `merge()` for sync. Never write a partial vault on failure.
3. `rust/src/device.rs`: age keypair generation, OS-keystore custody, public
   recipient output, registry add/list/revoke.
4. `rust/src/transport.rs`: multi-recipient bundle encode/decode, age recipient
   fingerprints, transfer ledger (`transfers.jsonl`), expiry, per-name merge,
   conflict and `--replace` handling.
5. `rust/src/main.rs`: `export`, `sync` (with `import` alias), `device`,
   `transport status`; TTY + `--yes` gate; `--json` metadata only; refuse
   `--out -`.
6. Profile path: fixed child, `agent-ok` check, exact-value redaction, profile
   audit. Do not add a generic env or argv path.
7. Tests: round-trip, wrong identity, expiry, identical no-op, conflict with and
   without `--replace`, second sync adds nothing, transfer-ID hash mismatch,
   revoked recipient cannot decrypt a post-revocation bundle, and a
   no-value-in-output scan over success and failure paths. Use fake values.
8. Acceptance on hardware: Windows export → Pi sync → `vkeys run` an `agent-ok`
   profile; the transcript shows counts/names/exit only.

## Acceptance

v1 must pass all of these:

1. Export 5 test keys on Windows, sync on Linux (Pi), all 5 inject correctly.
2. After sync, an `agent-ok` profile injects the keys into the fixed child
   non-interactively; the human never sees a value, and the child sees exactly
   the imported values (fake strings; child proves receipt without printing).
3. Wrong recipient key fails cleanly: non-zero exit, no entries imported, no
   partial vault write, audit records the refusal.
4. Expired bundle is refused before any write and changes nothing.
5. No value appears in stdout, stderr, `--json`, the audit log, or error text
   on success or failure paths.
6. `sync` run twice with the same bundle adds nothing the second time.
7. A name whose local and bundle values differ reports `conflict`, leaves the
   local value unchanged, and exits non-zero without `--replace <name>`.
   With `--replace <name>` only that name changes, and the change is audited.
8. A revoked device cannot decrypt a bundle exported after revocation; an
   active device still succeeds. The refusal is audited on the revoked device.
   A bundle exported before revocation may still be readable by it; that is
   documented, not a failure.
9. Reusing a transfer ID with a different content hash is refused.
10. An agent has no generic env/argv path; only a fixed `agent-ok` profile runs,
    and audit records profile, time, and exit.

`import` is an alias and must pass the same tests.

## Non-goals and honest limits

- Transport does not defend against same-user code. A process running as the
  human can use the OS keystore and `vkeys`; that is the documented Tier 1
  boundary. Transport keeps the **CLI** free of a value-printing path.
- Expiry is a policy check, not cryptographic erasure. Someone who copied a
  bundle before expiry and holds a recipient key can still read it.
- Revocation is local to each exporting vault and is not retroactive. There is
  no global revocation without a pairing/sync protocol. Revoke on every source
  and rotate affected keys after losing a device.
- Device private keys are the real secret. Losing one means generating a new
  identity and updating every source's registry; leaking one means revoke and
  rotate. The T3 backup key is separate and offline; it is not a transport
  device identity.
- Output scrubbing is best-effort against transformed values. The primary
  control is never handing an agent the value.
- Profiles are not in the bundle by default. A future opt-in could carry profile
  definitions without values.
- Delta export and live sync are future work. The bundle format leaves room for
  them, but v1 is one-shot.

## Open questions

- Recipient key custody: the device key lives in the OS credential store; is a
  hardware-backed/non-exportable key worth the platform work in v1?
- Should `sync` scaffold a profile when the bundle's names exactly match an
  existing profile, or require the profile to exist already? (Current answer:
  require it; no silent config edits.)
- Global revocation: with one-shot transport, revocation has to be repeated on
  every source. Is a signed, manually carried revocation list worth it before a
  real pairing/sync mode exists?
- Should `sync` stay human-only forever, or may a human mark a specific bundle
  path as agent-syncable? (Current answer: human-only; the board kept moving key
  material human-authorized.)

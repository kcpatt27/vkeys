# 001 - Privileged transport bundle

Status: deferred · Priority: P1 · Area: security / transport

## Context
The uncommitted `transport push` is agent-callable: the source vault is DPAPI
under the agent user's account, so an agent can decrypt and exfiltrate it. SSH encrypts the trip
but does not fix who can start it. Transport must be an admin/service operation.

## Audit 2026-11-10
The `feat/transport-age-bundle` branch does not meet this ticket. It rewrote the
scope to human-vault export/import and does not enforce the service user. The TTY
gate is not an agent boundary: a same-user PTY or ConPTY can choose the passphrase
and read the bundle. Do not merge it as this ticket.

Missing: service-user enforcement, public-key recipient, target binding,
replay/stale-bundle protection, actor/host/hash audit, `--replace` confirmation
or backup, a successful CLI test, and a non-Windows vault test.

## Decision 2026-11-11
ADR-0005 defers this ticket. 0.2 ships without transport; target identity (D) is
the later direction. The age branch does not satisfy this ticket and must not be
merged as-is.

## Acceptance
- [ ] `vkeys transport export` runs only as the service user (Linux `vkeys` /
  Windows SYSTEM), never the agent user.
- [ ] Bundle encrypted with age + passphrase (or a target public key); no values
  on stdout; hidden passphrase prompt.
- [ ] `import` on the target writes into its service vault; names-only output.
- [ ] Works for laptop/phone/USB without SSH.
- [ ] Docs state the OS-user requirement; tests cover encrypt/decrypt.
- [ ] Reconcile with the 018 decision before any implementation.

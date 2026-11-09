# 001 — Privileged transport bundle

Status: open · Priority: P1 · Area: security / transport

## Context
The uncommitted `transport push` is agent-callable: the source vault is DPAPI
under the agent user's account, so an agent can decrypt and exfiltrate it. SSH encrypts the trip
but does not fix who can start it. Transport must be an admin/service operation.

## Acceptance
- [ ] `vkeys transport export` runs only as the service user (Linux `vkeys` /
  Windows SYSTEM), never the agent user.
- [ ] Bundle encrypted with age + passphrase (or a target public key); no values
  on stdout; hidden passphrase prompt.
- [ ] `import` on the target writes into its service vault; names-only output.
- [ ] Works for laptop/phone/USB without SSH.
- [ ] Docs state the OS-user requirement; tests cover encrypt/decrypt.

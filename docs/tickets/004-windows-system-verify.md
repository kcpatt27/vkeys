# 004 — Windows SYSTEM broker verification

Status: open · Priority: P0 · Area: Windows / release

## Context
`deploy/windows/install.ps1` runs the broker as SYSTEM with an ACLed
`C:\ProgramData\vkeys-broker` and xchacha. Syntax-checked; not run elevated yet.

## Acceptance
- [ ] Elevated install succeeds; task runs at boot.
- [ ] `/healthz` returns 200; token at `%APPDATA%\vkeys\broker.token`.
- [ ] `icacls` shows SYSTEM + Administrators only; `Get-Content master.key` as
  `kcpat` fails.
- [ ] Update the README claim to "verified on Windows".

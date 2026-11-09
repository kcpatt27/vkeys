# 004 — Windows SYSTEM broker verification

Status: open · Priority: P0 · Area: Windows / release

## Context
`deploy/windows/install.ps1` runs the broker as SYSTEM with an ACLed
`C:\ProgramData\vkeys-broker` and xchacha. Syntax-checked; not run elevated yet.

## Check 2026-11-09
Not installed on the audit host: no `vkeys-broker` scheduled task, no
`C:\ProgramData\vkeys-broker`, no `%APPDATA%\vkeys\broker.token`. A non-elevated
run refuses before making changes. The install still needs an elevated run on a
host with port 8787 free.

## Acceptance
- [ ] Elevated install succeeds; task runs at boot.
- [ ] `/healthz` returns 200; token at `%APPDATA%\vkeys\broker.token`.
- [ ] `icacls` shows SYSTEM + Administrators only; `Get-Content master.key` as
  the agent user fails.
- [ ] Update the README claim to "verified on Windows".

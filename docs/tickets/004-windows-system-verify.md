# 004 - Windows SYSTEM broker verification

Status: done · Priority: P0 · Area: Windows / release

## Context
`deploy/windows/install.ps1` runs the broker as SYSTEM with an ACLed
`C:\ProgramData\vkeys-broker` and xchacha. Syntax-checked; not run elevated yet.

## Check 2026-11-09
Not installed on the audit host: no `vkeys-broker` scheduled task, no
`C:\ProgramData\vkeys-broker`, no `%APPDATA%\vkeys\broker.token`. A non-elevated
run refuses before making changes. The install still needs an elevated run on a
host with port 8787 free.

## Verified 2026-11-09
- Elevated install exits 0; the `vkeys-broker` task is Running.
- `/healthz` returns `{"budget_usd":5.0,"ok":true,...}`.
- The agent user cannot open `vault.bin` or `master.key`; it can read only
  `%APPDATA%\vkeys\broker.token`.
- A 1-token completion through the broker returned HTTP 200.
- Installer fixes from this run: inheritable directory ACLs, clean DACL
  replacement, checked `vkeys set` exits, broker stop before binary copy, and
  budget prices.

## Boot verified 2026-11-10
After a reboot at 07:30:12, the task chain started at 07:30:26 (`cmd.exe`), then
`vkeys.exe` and `vkeys-broker.exe` at 07:30:33. `/healthz` returned 200 and the
agent boundary still held. No open items.

## Acceptance
- [x] Elevated install succeeds.
- [x] Task runs at boot. Boot evidence: 14 to 21 seconds after boot.
- [x] `/healthz` returns 200; token at `%APPDATA%\vkeys\broker.token`.
- [x] `icacls` shows SYSTEM + Administrators only; the agent user cannot read
      the master key or vault.
- [x] A broker completion returns 200.
- [x] README claim updated to verified running, boot pending.

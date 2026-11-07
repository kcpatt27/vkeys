# Windows - hardened broker

Status: implemented and syntax-checked; not yet run elevated on a live Windows
host (ticket 004).

Installs the Tier 2 broker as **SYSTEM** via a startup scheduled task. The vault,
master key, and profiles live in `C:\ProgramData\vkeys-broker`, ACLed to
**SYSTEM + Administrators only**. The agent user (`kcpat`) cannot read them; it
gets only `%APPDATA%\vkeys\broker.token` (use-only).

## Install

Run in an **elevated** PowerShell (Run as administrator):

```powershell
pwsh -File .\deploy\windows\install.ps1
```

It asks once for the OpenRouter provider key (hidden), generates
`broker/token`, writes the agent token, registers the `vkeys-broker` scheduled
task as SYSTEM, starts it, and verifies `/healthz`.

For automation, put the provider key in a root-only file:

```powershell
pwsh -File .\deploy\windows\install.ps1 -ProviderKeyFile C:\secure\provider.key
```

or set `$env:VKEYS_PROVIDER_KEY` and pass `-NonInteractive`.

The installer uses the `xchacha20poly1305` vault scheme (not DPAPI), because the
key file must be readable by the SYSTEM task while remaining unreadable by the
agent user.

## Verify

```powershell
Invoke-RestMethod http://127.0.0.1:8787/healthz
icacls C:\ProgramData\vkeys-broker\master.key
# as kcpat (not elevated), this must fail:
Get-Content C:\ProgramData\vkeys-broker\master.key
```

The agent uses:

```text
base URL: http://127.0.0.1:8787/v1
token:    %APPDATA%\vkeys\broker.token
```

## Uninstall

```powershell
pwsh -File .\deploy\windows\install.ps1 -Uninstall
# also remove the vault and binaries:
pwsh -File .\deploy\windows\install.ps1 -Uninstall -RemoveData
```

## Why SYSTEM

Creating a dedicated service account on Windows requires a password and
`LogonAsService`; SYSTEM is simpler and already a separate identity from the
agent. The agent cannot read the ACLed secrets directory. Root/SYSTEM can read
anything, so this is still an OS-user boundary, not a hardware one.

## Notes

- The scheduled task runs at startup and the wrapper restarts the broker on exit.
- Keep the broker on loopback; if the agent is remote, bind the tailnet and keep
  the broker token secret.
- The task and data are per-machine. `broker/token` is use-only; rotate it by
  deleting the token file and re-running the installer.

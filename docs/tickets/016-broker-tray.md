# 016 - Broker lifecycle tray

Status: design - Priority: P2 - Area: UX / broker lifecycle

## Goal

A small cross-platform tray/menu-bar app that shows whether `vkeys-broker` is
running and lets a human start, stop, and open its logs without remembering
service commands.

It is a **lifecycle surface**, not a vault UI. It never reads or prints a key,
a vault value, or a broker token.

## Non-goals

- No key list, reveal, copy-value, or edit UI. The store stays write-only.
- No generic command runner. Every action is a fixed, allowlisted operation.
- No vault decryption in the tray process.
- No second config format; read the broker's existing config/health endpoint.
- No auto-start on logon in v1; the existing installer owns service setup.

## UX

Tray states:

- **green** - `/healthz` returns 200.
- **amber** - process/task exists but health is failing.
- **grey** - not installed or stopped.
- **red** - health error or token/auth failure.

Menu:

- `vkeys-broker: running` (status line, disabled)
- `Open health` - opens `http://127.0.0.1:8787/healthz`
- `Open logs` - opens the configured log file
- `Start broker` - runs the platform's fixed service command
- `Stop broker` - runs the fixed stop command
- `Copy start command` - for hosts where the user lacks service rights
- `Quit`

No free-text input. No shell.

## Security

- The tray runs as the **human user**, not SYSTEM/root.
- It may query `healthz` and the service manager. It may not read the vault,
  `master.key`, or `broker.token`.
- Start/stop uses a fixed argv per platform; no user-supplied arguments.
- If the service manager requires elevation, the tray shows "copy command" and
  does not prompt for credentials itself. Elevation stays with the installer.
- No token display. No "copy token" action.

## Platform plan

| Platform | Service manager | Tray implementation |
|---|---|---|
| Windows | Scheduled task (SYSTEM) | Rust `tray-icon` + fixed `schtasks` argv; status from health |
| macOS | launchd | Rust `tray-icon`; `launchctl` fixed argv |
| Linux | systemd (system or user) | Rust `tray-icon` + `systemctl` fixed argv; AppIndicator fallback |

Stack: a small `vkeys-tray` Rust binary using `tray-icon` (same repo, new
`[[bin]]`), not a web UI and not Tauri. It shares the broker health JSON parser.

## Phases

1. **Status-only tray** - health poll, green/amber/grey, open health/logs.
2. **Fixed start/stop** - platform command allowlist, no elevation prompt.
3. **Packaging** - ship with release artifacts; signed later (013).

## Acceptance

- [ ] Tray shows health within one poll interval and recovers after broker restart.
- [ ] No key value, vault path content, or broker token appears in UI/logs.
- [ ] Start/stop commands are fixed and unit-tested; no arbitrary argv.
- [ ] Works on Windows first; macOS/Linux documented as untested until run.
- [ ] `cargo test --release` covers health parsing and the command allowlist.
- [ ] Release artifacts include `vkeys-tray` only when phase 1 is green.

## Open questions

- Does Windows scheduled-task start require elevation from a standard user? If
  yes, phase 2 is "copy command" only and the tray stays status-only.
- Ship a separate tray binary or a `vkeys tray` subcommand? Separate binary keeps
  the CLI headless and the dependency surface small.
- Signing/notarization is deferred with the rest of the macOS artifacts (013).

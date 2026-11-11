# vkeys - virtual keys

[![ci](https://github.com/kcpatt27/vkeys/actions/workflows/ci.yml/badge.svg)](https://github.com/kcpatt27/vkeys/actions/workflows/ci.yml)

![vkeys demo](https://raw.githubusercontent.com/kcpatt27/vkeys/main/docs/assets/demo.gif)

Personal, machine-wide key store for Windows, macOS, and Linux. Keys are stored
once, encrypted at rest, and **injected into named profiles** - an agent can
*use* a key but has no command that prints it.

Name: **vkeys** = "virtual keys". Deliberately not `keys` / `secret` / `vault`
(see `DECISIONS.md`).

## Model

```
vkeys set openrouter/meridian        # hidden paste (or --clipboard); value never echoed
vkeys ls                             # names + metadata; never values
vkeys env meridian                   # OPENROUTER_API_KEY=<stored: openrouter/meridian>
vkeys run meridian                   # fixed argv + fixed env; caller supplies no command
vkeys rotate openrouter/meridian     # new value required; old is unrecoverable
vkeys audit                          # profile, time, scrub hits, exit
vkeys import-env path/.env --names OPENROUTER_API_KEY
vkeys init                           # create the vault + ACL (once)
vkeys selfcheck                      # runnable check
```

**There is no `vkeys get`.** If you need to see a key, make a new one. That is
the point - it is a feature, not a missing command.

Tier 2 is available as `vkeys-broker`: a local OpenRouter proxy that holds the
provider key and authenticates clients with a separate broker token. See
`docs/BROKER.md`.

## Profiles

`%APPDATA%\vkeys\profiles.toml` (see `profiles.example.toml`):

```toml
[profile.meridian]
cmd = ["C:\\...\\meridian-whisper\\.venv\\Scripts\\python.exe", "ui_hotkey.py"]
cwd = "C:\\...\\meridian-whisper"
agent-ok = true

[profile.meridian.keys]
OPENROUTER_API_KEY = "openrouter/meridian"
```

- `agent-ok = true` → agents may run it non-interactively.
- `agent-ok = false` (default) → human-only; refuses when stdin is not a TTY.
- A profile's command is **fixed**. There is no generic `run -- <argv>`, so
  `vkeys run -- python -c "print(env)"` cannot be expressed.

## What Tier 1 guarantees (and doesn't)

Tier 1: profiles + no `get` + clean env + output scrubbing + audit + provider
spend caps. This makes accidental/drive-by exposure very hard.

Honest boundary: `vault.bin` is DPAPI **CurrentUser**. It stops other Windows
accounts and disk theft, but any process running as you - including an agent -
can decrypt it with a small script. Tier 2 is `vkeys-broker`: the provider key
stays in the broker process and never enters the client. See `docs/BROKER.md`.

**The strong boundary is a separate OS user.** Run the broker as a dedicated user
(Linux `vkeys` system user; Windows SYSTEM via a startup task) with the vault and
key readable only by that user. The agent's user then cannot decrypt the vault at
all; it gets a use-only broker token. See [`deploy/raven/`](deploy/raven/) and
[`deploy/windows/`](deploy/windows/).

The Linux path is verified on Raven (ticket 003). The Windows SYSTEM path is
verified on a live Windows host for install, boot start, health, ACL, and broker
use (ticket 004).

## Store

- **Vault:** Windows `C:\ProgramData\secrets\vault.bin`; macOS
  `~/Library/Application Support/vkeys/vault.bin`; Linux
  `$XDG_DATA_HOME/vkeys/vault.bin` (or `~/.local/share/vkeys/vault.bin`).
  Override with `--vault`, `VKEYS_VAULT`, or `VKEYS_DATA_DIR`.
- **Profiles:** Windows `%APPDATA%\vkeys\profiles.toml`; macOS
  `~/Library/Application Support/vkeys/profiles.toml`; Linux
  `$XDG_CONFIG_HOME/vkeys/profiles.toml` (or `~/.config/vkeys/profiles.toml`).
- **Audit:** `audit.jsonl` next to the vault.
- **Crypto:** Windows per-value DPAPI (compat) or XChaCha20-Poly1305;
  macOS/Linux XChaCha20-Poly1305 with the 32-byte master key in Keychain /
  Secret Service. Metadata is plaintext. On Windows,
  `VKEYS_SCHEME=xchacha20poly1305` selects the portable key-file scheme; the
  Windows service install uses it.
- **Headless Linux / Pi:** no Secret Service? The master key falls back to
  `~/.config/vkeys/master.key` (0600). Set `VKEYS_MASTER_KEY_FILE` to pin it.
  See [`docs/PI.md`](docs/PI.md).
- **Service vaults:** set `VKEYS_MASTER_KEY_FILE` to the key that encrypted the
  vault, every time. If the vault exists and that file is missing, vkeys refuses
  to create a new key. The vault stores a key fingerprint, and a wrong key fails
  before any write. See [`docs/RECOVERY.md`](docs/RECOVERY.md).

## Tier 2

`vkeys-broker` is a local OpenRouter proxy (Rust, default build): it holds the
provider key, clients authenticate with a separate broker token, and only
non-streaming `POST /v1/chat/completions` plus `GET /healthz` are exposed. See
`docs/BROKER.md`.

## Install (Rust is the default)

```powershell
cd rust
cargo build --release
$bin = "$env:USERPROFILE\.local\bin"
Copy-Item target\release\vkeys.exe        "$bin\vkeys.exe" -Force
Copy-Item target\release\vkeys-broker.exe "$bin\vkeys-broker.exe" -Force
```

```bash
# macOS / Linux
cargo build --release
install -m 755 target/release/vkeys target/release/vkeys-broker ~/.local/bin/
```

The Python oracle is archived at tag `python-oracle-final`; `PORT.md` is its record.

## Tests

```powershell
cd rust
cargo test --release        # 3 CLI + 19 broker + 3 unit tests
```

CI is manual-only while the repo is private (ticket 015).

See `PORT.md` (historical) for the Python parity record.

## Roadmap

- Portable vault file: master-key export/import so a vault moves between machines.
- Broker: per-client tokens + per-client budgets; optional SSE pass-through.
- Broker lifecycle: logon scheduled task + tray (status/start/stop).

Rust-only addition: `set`/`rotate` take `--stdin` for scripted provisioning
(`printf '%s' "$KEY" | vkeys set --stdin name`). Without a TTY and without
`--stdin`, the Rust build refuses instead of reading a pipe silently. There is
still no `get`.

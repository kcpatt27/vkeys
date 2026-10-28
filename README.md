# vkeys — virtual keys

Personal, machine-wide key store for this PC. Keys are stored once, encrypted at
rest, and **injected into named profiles** — an agent can *use* a key but has no
command that prints it.

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
the point — it is a feature, not a missing command.

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
accounts and disk theft, but any process running as you — including an agent —
can decrypt it with a small script. Tier 2 (a local broker that makes the call
server-side, so the key never enters an agent-controlled process) is the only
true "cannot see it". Not built yet; see `DECISIONS.md`.

## Store

- Vault: `C:\ProgramData\secrets\vault.bin` (overridable with `--vault` or
  `VKEYS_VAULT`).
- Audit: `audit.jsonl` next to the vault.
- Crypto: Windows DPAPI, current user. Encrypted per value; metadata is plaintext.

## Dev

```powershell
uv sync
python -m unittest discover -s tests -v
uv tool install .            # puts vkeys.exe on PATH (.local\bin)
```

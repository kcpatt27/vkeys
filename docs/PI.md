# vkeys on a Raspberry Pi / headless Linux (aarch64)

Raven-class target: Debian 13 (trixie), aarch64, no Secret Service provider.
vkeys is happy there, but the master key must come from a **0600 key file**
instead of the OS keychain.

**Verified 2026-10-31 on Raven:** Debian 13, aarch64, Rust 1.99.1; native
build 2m43s; `set`/`ls`/`env`/`run`/`audit`; key file `0600`; `[REDACTED]` and
`scrub_hits: 1` confirmed.

## Build natively (recommended for the Pi)

```bash
# once
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"

# from a checkout
cd rust
cargo build --release
install -m 755 target/release/vkeys        "$HOME/.local/bin/vkeys"
install -m 755 target/release/vkeys-broker "$HOME/.local/bin/vkeys-broker"
```

Native build is the support test: it proves the Pi's own glibc/toolchain. For a
release artifact, prefer a static `aarch64-unknown-linux-musl` build (no glibc
version coupling); test it on the same Pi before shipping.

## Key file (headless)

```bash
export VKEYS_MASTER_KEY_FILE="$HOME/.config/vkeys/master.key"
vkeys init
printf '%s\n' 'your-secret' | vkeys set demo/key --stdin
vkeys ls
```

Without `VKEYS_MASTER_KEY_FILE`, vkeys tries the Secret Service and falls back to
the same path when no provider is present. The file is created atomically with
`0600`.

## systemd (broker, set-and-forget)

`~/.config/systemd/user/vkeys-broker.service`:

```ini
[Unit]
Description=vkeys OpenRouter broker

[Service]
Environment=VKEYS_MASTER_KEY_FILE=%h/.config/vkeys/master.key
ExecStart=%h/.local/bin/vkeys run openrouter-broker
Restart=always
RestartSec=3

[Install]
WantedBy=default.target
```

```bash
systemctl --user daemon-reload
systemctl --user enable --now vkeys-broker
systemctl --user status vkeys-broker
```

The `openrouter-broker` profile and its two keys must exist in the Pi's vault
first (`openrouter/broker`, `broker/token`). Keep the broker on loopback or the
tailnet; never expose it publicly.

## Security boundary

The key file is readable by the same user (and root), like DPAPI/Keychain
entries. The vault is still encrypted at rest; the key file is the root of
trust. For a true "agent cannot see it" boundary, use the Tier 2 broker so the
provider key never enters the agent's process.

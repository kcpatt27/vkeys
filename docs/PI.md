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

**Both CI artifacts tested on Raven (2026-10-31):**
- `aarch64-unknown-linux-gnu` — dynamically linked, runs.
- `aarch64-unknown-linux-musl` — **statically linked**, runs.
Use the musl build for the release asset.

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

## Moving a vault from Windows

Copying `vault.bin` moves metadata only. Windows entries are DPAPI and cannot be
decrypted on Linux. Re-enter the values on the Pi (`vkeys set ...`) or use the PC
broker over the tailnet. The Pi's own vault starts empty.

## systemd (broker, hardened)

Use `deploy/raven/install.sh`: it creates a dedicated **`vkeys` system user**,
installs the binaries, and enables `deploy/raven/vkeys-broker.service`. The vault
(`/var/lib/vkeys`) and key (`/etc/vkeys/master.key`) are readable only by
`vkeys`; the agent user (`kcpat`) cannot decrypt them and only gets
`broker/token`.

```bash
sudo ./deploy/raven/install.sh "$PWD/rust/target/release"
# seed openrouter/broker + broker/token as the vkeys user, then:
sudo systemctl enable --now vkeys-broker
curl -s http://127.0.0.1:8787/healthz
```

A `systemd --user` unit as `kcpat` is fine for a quick test but is **not** a
boundary against an agent running as `kcpat`. See [`deploy/raven/`](../deploy/raven/).

## Security boundary

The key file is readable by the same user (and root), like DPAPI/Keychain
entries. The vault is still encrypted at rest; the key file is the root of
trust. For a true "agent cannot see it" boundary, use the Tier 2 broker so the
provider key never enters the agent's process.

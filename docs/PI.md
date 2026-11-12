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
- `aarch64-unknown-linux-gnu` - dynamically linked, runs.
- `aarch64-unknown-linux-musl` - **statically linked**, runs.
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

Use `deploy/raven/install.sh` - one command. It creates a dedicated **`vkeys`
system user**, installs both binaries, prompts once for the OpenRouter key,
generates `broker/token`, writes it to `~<agent-user>/.config/vkeys/broker.token`,
and enables the service. The vault (`/var/lib/vkeys`) and key
(`/etc/vkeys/master.key`) are readable only by `vkeys`; the agent user cannot
decrypt them and only gets the use-only token.

```bash
sudo ./deploy/raven/install.sh "$PWD/rust/target/release"
curl -s http://127.0.0.1:8787/healthz
```

The only interactions are `sudo` and the hidden provider-key prompt. A
`systemd --user` unit as the agent user is fine for a quick test but is **not** a
boundary against an agent running as that user. See
[`deploy/raven/`](../deploy/raven/).

## Broker go-live (verified 2026-11-07)

The hardened install is live on Raven:

- `vkeys-broker.service` is active and enabled; the broker binds
  `127.0.0.1:8787`.
- `/healthz` returned `{"budget_usd":5.0,"ok":true,...}` after a real
  completion.
- The agent user cannot read `/etc/vkeys/master.key`;
  `~/.config/vkeys/broker.token` is `0600`.
- The `openrouter-broker` profile runs with `--budget-usd 5.00 --price-in 0.30
  --price-out 1.20 --audit /var/lib/vkeys/audit.jsonl`.

The earlier Raven start failed because the profile set a `$5` budget without
`--price-in`/`--price-out`; the broker rejects a positive budget with both
prices zero. The prices above are the live fix; the deployment docs record the
acceptance check.

## Security boundary

The key file is readable by the same user (and root), like DPAPI/Keychain
entries. The vault is still encrypted at rest; the key file is the root of
trust. For a true "agent cannot see it" boundary, use the Tier 2 broker so the
provider key never enters the agent's process.

# Raven - hardened broker, one command

Runs the Tier 2 broker as a dedicated **`vkeys` system user**, so the agent user
cannot read `/etc/vkeys/master.key` or `/var/lib/vkeys/vault.bin`. The agent
holds only the broker token and can *use* the provider key through
`http://127.0.0.1:8787`.

## Install

```bash
# on Raven, with a release build in rust/target/release
sudo ./deploy/raven/install.sh "$PWD/rust/target/release"
```

It will:

1. create the `vkeys` system user;
2. install `vkeys` + `vkeys-broker` to `/usr/local/bin`;
3. publish the `openrouter-broker` profile;
4. **prompt once** for the OpenRouter provider key (hidden input);
5. generate `broker/token`, store it, and write it to
   `~<agent-user>/.config/vkeys/broker.token` (0600, owned by the agent);
6. enable and start `vkeys-broker`;
7. verify `GET /healthz`.

The only interactions are `sudo` and the hidden provider-key prompt. For a
non-interactive run, put the provider key in a root-only file and set
`VKEYS_PROVIDER_KEY_FILE`; the broker token is still generated. Re-running is
idempotent; an existing provider key and token are left alone.

## Manual commands

Manual `set`, `rotate`, or `import-env` must use the service environment. Do not
run them as the `vkeys` user without `VKEYS_MASTER_KEY_FILE`: that is how the
2026-11-11 mixed-key vault was created.

```bash
sudo -u vkeys env \
  VKEYS_VAULT=/var/lib/vkeys/vault.bin \
  VKEYS_MASTER_KEY_FILE=/etc/vkeys/master.key \
  VKEYS_PROFILES=/etc/vkeys/profiles.toml \
  VKEYS_SCHEME=xchacha20poly1305 \
  /usr/local/bin/vkeys rotate broker/token --stdin
```

The CLI now refuses to create a key when the vault exists and names the expected
path. See [`../../docs/RECOVERY.md`](../../docs/RECOVERY.md).

## Verify the boundary

```bash
sudo systemctl status vkeys-broker --no-pager
curl -s http://127.0.0.1:8787/healthz
# as the agent user (replace <agent-user>):
cat ~/.config/vkeys/broker.token
sudo -u <agent-user> test -r /etc/vkeys/master.key && echo "BAD: agent can read key" || echo "ok: key unreadable"
```

## Reset the broker keys

The vault is deliberately write-only. To start over:

```bash
sudo systemctl stop vkeys-broker
sudo rm -f /var/lib/vkeys/vault.bin /etc/vkeys/master.key ~<agent-user>/.config/vkeys/broker.token
sudo ./deploy/raven/install.sh "$PWD/rust/target/release"
```

## Why this is the strong boundary

Tier 1 as the agent user can still be decrypted by that user. The service user is
a different OS identity; the agent cannot read the vault or the key, and the
broker only exposes use-only endpoints. That is the honest form of "the agent
never sees the key."

## Notes

- `broker/token` is a use-only bearer token; give it to the agent, not the
  provider key.
- Keep the broker on loopback or the tailnet; never expose `:8787` publicly.
- The service restarts on failure; the audit log is `/var/lib/vkeys/audit.jsonl`.
- Windows equivalent (`deploy/windows/`) is implemented but has not been
  verified on a live host yet; see ticket 004.

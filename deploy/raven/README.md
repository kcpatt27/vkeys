# Raven — hardened broker

Runs the Tier 2 broker as a **dedicated `vkeys` system user**, so the agent user
(`kcpat`) cannot read `/etc/vkeys/master.key` or `/var/lib/vkeys/vault.bin`.
The agent holds only the broker token and can *use* the provider key through
`http://127.0.0.1:8787`.

## Install

```bash
# from the repo on Raven, with a release build in rust/target/release
sudo ./deploy/raven/install.sh "$PWD/rust/target/release"
```

Then seed the two keys as `vkeys` (hidden prompts), and start:

```bash
sudo -u vkeys env VKEYS_VAULT=/var/lib/vkeys/vault.bin \
  VKEYS_MASTER_KEY_FILE=/etc/vkeys/master.key \
  /usr/local/bin/vkeys set openrouter/broker

sudo -u vkeys env VKEYS_VAULT=/var/lib/vkeys/vault.bin \
  VKEYS_MASTER_KEY_FILE=/etc/vkeys/master.key \
  /usr/local/bin/vkeys set broker/token

sudo systemctl enable --now vkeys-broker
curl -s http://127.0.0.1:8787/healthz
```

## Why this is the strong boundary

`vkeys` run as the agent user (Tier 1) can still be decrypted by that user. The
service user is a different OS identity; the agent cannot read the vault or the
key, and the broker only exposes use-only endpoints. That is the honest form of
"the agent never sees the key."

## Notes

- `broker/token` is a use-only bearer token; give it to the agent, not the
  provider key.
- Keep the broker on loopback or the tailnet; never expose `:8787` publicly.
- The service restarts on failure; the audit log is `/var/lib/vkeys/audit.jsonl`.

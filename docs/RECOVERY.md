# Recovery

## Service vaults

A service vault must be opened with the same master key that encrypted it. Set
`VKEYS_MASTER_KEY_FILE` for every command, including manual `set`, `rotate`, and
`import-env`. The systemd unit already does this:

```ini
[Service]
Environment=VKEYS_VAULT=/var/lib/vkeys/vault.bin
Environment=VKEYS_MASTER_KEY_FILE=/etc/vkeys/master.key
Environment=VKEYS_PROFILES=/etc/vkeys/profiles.toml
```

Manual command against the service vault:

```bash
sudo -u vkeys env \
  VKEYS_VAULT=/var/lib/vkeys/vault.bin \
  VKEYS_MASTER_KEY_FILE=/etc/vkeys/master.key \
  VKEYS_PROFILES=/etc/vkeys/profiles.toml \
  VKEYS_SCHEME=xchacha20poly1305 \
  /usr/local/bin/vkeys rotate broker/token --stdin
```

If the vault exists and the resolved key file is missing, vkeys fails instead of
creating a new key. A wrong key is detected by the key fingerprint stored in the
vault and fails before any mutation. The error names `VKEYS_MASTER_KEY_FILE` and
the expected path. It never prints a key or a value.

## Incident 2026-11-11: mixed-key vault on Raven

**Symptom.** The broker failed with `vkeys: decryption failed` after a manual
token rotation.

**Cause.** `vkeys rotate broker/token --stdin` was run as the `vkeys` user
without `VKEYS_MASTER_KEY_FILE`. The CLI fell back to the service user's
`~/.config/vkeys/master.key`, created a new key there, and wrote the rotated
entry under it. The other entry stayed under `/etc/vkeys/master.key`, so the
vault was mixed-key.

**Recovery.** No secret values were printed.

1. Stop the broker:
   ```bash
   sudo systemctl stop vkeys-broker
   ```
2. Restore the original `/etc/vkeys/master.key` from backup.
3. Re-run the set with the service key:
   ```bash
   sudo -u vkeys env \
     VKEYS_VAULT=/var/lib/vkeys/vault.bin \
     VKEYS_MASTER_KEY_FILE=/etc/vkeys/master.key \
     VKEYS_PROFILES=/etc/vkeys/profiles.toml \
     VKEYS_SCHEME=xchacha20poly1305 \
     /usr/local/bin/vkeys set broker/token --stdin
   ```
4. Start the broker and check health:
   ```bash
   sudo systemctl start vkeys-broker
   curl -fsS http://127.0.0.1:8787/healthz
   ```

**Prevention.** The CLI now refuses to create a master key when a vault exists.
It stores a non-secret key id in the vault and fails on a mismatch before any
write. The service docs show the required environment.

## If the key is lost

The vault is write-only and has no recovery command. If the key that encrypted
the vault is lost, the values are unrecoverable. Recreate the vault and re-enter
the keys. Do not delete a vault until you have confirmed the key is available.

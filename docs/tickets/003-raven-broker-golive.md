# 003 - Raven broker go-live + boundary check

Status: done · Priority: P0 · Area: Raven / release

## Context
`deploy/raven/install.sh` is installed and live on Raven as of 2026-11-07. The
broker runs as the `vkeys` system user; the vault is `/var/lib/vkeys/vault.bin`
and the provider key is stored as `openrouter/broker`.

## Acceptance
- [x] Create a fresh OpenRouter key for the Pi (or supply one) and run the installer.
- [x] `curl -s http://127.0.0.1:8787/healthz` returns 200.
- [x] `sudo -u <agent-user> test -r /etc/vkeys/master.key` fails.
- [x] `~<agent-user>/.config/vkeys/broker.token` is 0600 and use-only.
- [x] Record the result in `docs/PI.md`.

## Result (2026-11-07)

- `vkeys-broker.service` is active and enabled; it binds `127.0.0.1:8787`.
- `/healthz` returned `{"budget_usd":5.0,"ok":true,...}` after a real completion.
- `sudo -u <agent-user> test -r /etc/vkeys/master.key` fails.
- `~/.config/vkeys/broker.token` is `0600`.
- The `openrouter-broker` profile runs with `--budget-usd 5.00 --price-in 0.30
  --price-out 1.20 --audit /var/lib/vkeys/audit.jsonl`. The earlier profile used
  the `$5` budget with zero prices, which the broker rejects.
- Recorded in `docs/PI.md`.

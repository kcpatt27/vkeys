# 003 — Raven broker go-live + boundary check

Status: open · Priority: P0 · Area: Raven / release

## Context
`deploy/raven/install.sh` is staged on Raven. The broker vault is empty; the
installer prompts for a provider key and generates `broker/token`.

## Acceptance
- [ ] Create a fresh OpenRouter key for the Pi (or supply one) and run the installer.
- [ ] `curl -s http://127.0.0.1:8787/healthz` returns 200.
- [ ] `sudo -u kcpat test -r /etc/vkeys/master.key` fails.
- [ ] `~kcpat/.config/vkeys/broker.token` is 0600 and use-only.
- [ ] Record the result in `docs/PI.md`.

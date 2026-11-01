# 009 — Broker hardening

Status: open · Priority: P1 · Area: security

## Context
Loopback/token/fixed-upstream is the baseline. No rate limiting; shared token;
in-memory budget can overshoot under concurrency; tailnet binding is manual.

## Acceptance
- [ ] Rate limit per token/IP; 429.
- [ ] Token rotation procedure; optional per-client tokens.
- [ ] Document loopback/tailnet binding + firewall/ACL.
- [ ] Budget accounting is atomic.

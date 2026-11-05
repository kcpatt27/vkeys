# 009 — Broker hardening

Status: done · Priority: P1 · Area: security

## Context
Loopback/token/fixed-upstream is the baseline. No rate limiting; shared token;
in-memory budget can overshoot under concurrency; tailnet binding is manual.

## Acceptance
- [x] Rate limit per token/IP; 429.
- [x] Token rotation procedure; per-client tokens deferred to 011.
- [x] Document loopback/tailnet binding + firewall/ACL.
- [x] Budget accounting is atomic.

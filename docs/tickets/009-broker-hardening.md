# 009 - Broker hardening

Status: done · Priority: P1 · Area: security

## Context
Loopback/token/fixed-upstream is the baseline. No rate limiting; shared token;
in-memory budget can overshoot under concurrency; tailnet binding is manual.

## Acceptance
- [x] Rate limit per token/IP; 429.
- [x] Token rotation procedure; per-client tokens deferred to 011.
- [x] Document loopback/tailnet binding + firewall/ACL.
- [x] Budget accounting is atomic.

## Verification findings

An independent verifier reviewed the hardening and found two HIGH issues and
several MEDIUM issues. Fixed on `main` in `f384656`, `d0a51fa`, and `e73b16f`:

- **HIGH - budget overshoot:** `settle` added actual cost with no cap, so a
  large `max_tokens` could push `spent_usd` past `budget_usd`. The broker now
  parses the request, rejects `max_tokens` above the ceiling with 400, rejects
  `max_tokens` of zero with 400, leaves a missing `max_tokens` unchanged, and
  caps the ledger at the reservation while auditing `overage_usd`.
- **HIGH - interleaved audit lines:** concurrent requests could interleave
  `writeln!` output. Audit appends are serialized and written as one complete
  line per `write_all`.
- **MEDIUM - config validation:** negative/non-finite budget and prices are
  rejected; a positive budget with both prices zero is rejected; `budget_usd ==
  0` is the only disabled value.
- **MEDIUM - unaudited 400s:** invalid body, invalid JSON, and streaming
  refusals now carry a reason in the audit log.
- **MEDIUM - `env_or`:** a present-but-unparseable env value now fails startup
  instead of silently using the default.
- **MEDIUM - shared-IP starvation:** documented in `docs/BROKER.md`; ordering
  unchanged so the unauthenticated flood cap stays intact.

Deferred LOW items are listed in `017-broker-verification-findings.md`.

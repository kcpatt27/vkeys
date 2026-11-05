# 017 — Broker verification findings

Status: done · Priority: P1 · Area: security

## Context

An independent verifier reviewed the broker hardening from ticket 009. This
ticket records the findings and what was fixed versus deferred. The fixes
landed on `fix/broker-budget-audit`.

## HIGH — fixed

### Budget could overshoot

`reserve` used a fixed ceiling, but `settle` added actual cost with no cap. A
client could send a huge `max_tokens` and force `spent_usd > budget_usd`.

Fix:

- parse the request JSON before forwarding
- reject `max_tokens > max_request_tokens` with HTTP 400
  `{"error":"max_tokens exceeds broker ceiling"}` and audit the refusal
- reject `max_tokens == 0` with HTTP 400 `{"error":"max_tokens must be at least 1"}`
- inject `max_tokens = max_request_tokens` when the field is absent
- cap the ledger at the reservation on settle and audit `overage_usd`
  (actual - reserved); the ledger never exceeds `budget_usd`
- document that the provider may still bill overage if the upstream ignores
  `max_tokens`; only the broker ledger is capped

Tests: `oversized_max_tokens_is_rejected`, `absent_max_tokens_is_injected`,
`overage_is_capped_and_audited`.

### Audit lines corrupted under concurrency

Each request opened the audit file and `writeln!` could interleave.

Fix: a process-wide `Mutex<()>` serializes appends and each event is written
with one `write_all` of a complete line.

Test: `concurrent_audit_lines_are_well_formed` (100 concurrent successful
requests, every line parses as JSON).

## MEDIUM — fixed

- **Config validation:** reject negative or non-finite `budget_usd`,
  `price_in`, `price_out`; reject `budget_usd > 0` when both prices are zero;
  `budget_usd == 0` is the only disabled value. Test:
  `config_rejects_invalid_numbers`.
- **Audit 400 refusals:** invalid body, invalid JSON, and streaming now carry a
  reason, matching `docs/BROKER.md`. Test: `bad_requests_are_audited`.
- **`env_or`:** a present-but-unparseable env value returns an error instead of
  silently using the default. Test: `env_or_rejects_unparseable_value`.
- **Shared-IP rate-limit starvation:** documented in `docs/BROKER.md`. The
  check order is unchanged so the unauthenticated flood cap is not weakened.

## LOW — deferred

Not fixed in this change; none are release blockers.

- Constant-time token comparison (current `==` leaks timing).
- Hasher choice for rate-limit keys (`DefaultHasher` is not a security hash).
- `retain` over the whole rate map on every request is O(n).
- Thread-per-request instead of a bounded worker pool.
- No cap on the upstream response body size.
- Audit write failures are ignored (no fallback or alert).

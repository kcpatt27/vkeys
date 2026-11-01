# 002 — Remove agent-callable transport push

Status: open · Priority: P0 · Area: cleanup

## Context
149 uncommitted lines in `rust/src/main.rs` implement `transport push/receive`.
Decision: do not ship a value-export path that the agent user can invoke.
Superseded by 001.

## Acceptance
- [ ] Working tree clean (no uncommitted transport code).
- [ ] `vkeys --help` has no agent-callable `transport push` (privileged-only per 001).
- [ ] No value-bearing command is reachable by the agent user.

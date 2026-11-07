# 002 - Remove agent-callable transport push

Status: done - Priority: P0 - Area: cleanup

## Context

149 uncommitted lines in `rust/src/main.rs` implemented `transport push/receive`.
Decision: do not ship a value-export path that the agent user can invoke.
Superseded by 001.

## Acceptance

- [x] Working tree clean (no uncommitted transport code on `main`).
- [x] `vkeys --help` has no agent-callable `transport push` (privileged-only per 001).
- [x] No value-bearing command is reachable by the agent user.

Verified 2026-11-07 on `main` at `e73b16f`: `rust/src/main.rs` has no `transport`
subcommand. The privileged age bundle exists only on `feat/transport-age-bundle`
and is not merged for 0.1. See ticket 018 for the redesign analysis.

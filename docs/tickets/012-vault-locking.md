# 012 — Vault locking

Status: open · Priority: P3 · Area: storage

## Context
No locking; concurrent writers can lose an update. Documented known edge.

## Acceptance
- [ ] Lock or atomic compare-and-swap; test with two writers.

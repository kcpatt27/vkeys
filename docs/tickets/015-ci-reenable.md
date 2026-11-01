# 015 — CI re-enable + release triggers

Status: open · Priority: P0 · Area: CI

## Context
`ci.yml` is `workflow_dispatch` only; `release-artifacts.yml` is manual/release.
Nothing runs while private.

## Acceptance
- [ ] After public: re-add `push`/`pull_request` to `ci.yml`; run once green.
- [ ] Confirm `release-artifacts` runs on a published release.
- [ ] Keep no push trigger while private.

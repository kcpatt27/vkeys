# 005 - Release v0.1

Status: open · Priority: P0 · Area: release

## Context
Private repo, crate dry-run clean, `vkeys` name free, CI manual.

## Prepared 2026-11-11
- Tag `v0.1.0` at `1dc8afb`, pushed to `origin` and `github`.
- `release-artifacts` run `43104126257` at `1dc8afb`: all six jobs green.
- Six tag-named archives downloaded and checked: both binaries, LICENSE, README,
  and matching SHA-256.
- Draft GitHub release created with the 12 assets; the assets were replaced after
  the final tag build.
- Release notes: `docs/RELEASE-0.1.0.md`. The tag was moved from `47eff7e` to
  `1dc8afb` before publication to include the corrected transport decision. The
  repo was private and the draft had no consumers.

Remaining on 2026-11-12: `vkeys run publish-vkeys`, publish the release, verify
crates.io returns 200, flip public, then 015.

## Acceptance
- [ ] `vkeys run publish-vkeys` publishes 0.1.0; crates.io returns 200.
- [x] Tag `v0.1.0`; draft GitHub release; artifacts attached.
- [ ] Repo flipped public; CI re-enabled and green.
- [x] Release notes use the scoped claim (Tier 1 human-only; Tier 2 strong).

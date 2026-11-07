# 006 - x64 release artifacts

Status: open · Priority: P0 · Area: release

## Context
`release-artifacts.yml` builds Linux ARM64 + macOS universal + Windows ARM64,
plus the x64 jobs added in commit `e6a3e5b`:
`linux-x64` (`x86_64-unknown-linux-musl`, lines 130-158) and `windows-x64`
(`x86_64-pc-windows-msvc`, lines 160-189).

The x64 jobs are not missing and they have run. The last `release-artifacts`
workflow_dispatch run (2026-11-05, run `42022173611`, head `0819084`) was
green for all six jobs, and the x64 archives were downloaded and checked:
both contain `vkeys`, `vkeys-broker`, `LICENSE`, and `README`, and each has a
matching SHA-256 sidecar.

What is still missing: that run predates the three broker fixes now on `main`
(`f384656`, `d0a51fa`, `e73b16f`), so the published x64 binaries do not
contain the current broker code, and no GitHub release exists to attach
artifacts to.

## Acceptance
- [x] Add `x86_64-unknown-linux-musl` and `x86_64-pc-windows-msvc` jobs.
- [x] Include both binaries + LICENSE + README; emit SHA-256.
- [ ] Attach to the release.
- [ ] Rebuild the x64 jobs at the release commit so the artifacts carry the
      current broker code.

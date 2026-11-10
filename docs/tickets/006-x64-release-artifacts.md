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

That run predated the broker fixes now on `main`; the 2026-11-10 run below
supersedes it. The only open item is attaching the tag-built artifacts to the
GitHub release.

## Verified 2026-11-10
`release-artifacts` run `42818528496` at `b145d63` is green for all six jobs.
Every archive was downloaded and checked: both binaries, LICENSE, README, and a
matching SHA-256. This is the release-candidate build; the tag-triggered run
will produce the `vkeys-v0.1.0-*` release set.

## Acceptance
- [x] Add `x86_64-unknown-linux-musl` and `x86_64-pc-windows-msvc` jobs.
- [x] Include both binaries + LICENSE + README; emit SHA-256.
- [x] Give every archive the same layout: Linux ARM64 and Windows ARM64 now
      include LICENSE + README and a SHA-256 sidecar; macOS includes LICENSE and
      README alongside its existing checksum.
- [ ] Attach to the release.
- [x] Rebuild at the release-candidate commit so the artifacts carry the current
      broker code. Run `42818528496` at `b145d63`, all six archives verified.

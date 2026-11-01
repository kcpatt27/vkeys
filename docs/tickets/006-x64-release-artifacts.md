# 006 — x64 release artifacts

Status: open · Priority: P0 · Area: release

## Context
`release-artifacts.yml` builds Linux ARM64 + macOS universal + Windows ARM64.
Missing x64 Linux and x64 Windows artifacts.

## Acceptance
- [ ] Add `x86_64-unknown-linux-musl` and `x86_64-pc-windows-msvc` jobs.
- [ ] Include both binaries + LICENSE + README; emit SHA-256.
- [ ] Attach to the release.

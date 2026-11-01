# 007 — Verify ARM/macOS artifacts

Status: open · Priority: P1 · Area: release

## Context
The ARM/macOS jobs are scaffolded but never run. The macOS universal needs a
real Mac check; Windows ARM needs a Snapdragon check (x64 emulation works today).

## Acceptance
- [ ] Run `release-artifacts.yml` manually after the public flip.
- [ ] macOS `lipo -archs` shows both slices; quarantine/xattr flow documented.
- [ ] Windows ARM PE is `0xAA64`; smoke test on a Snapdragon PC.
- [ ] Record results.

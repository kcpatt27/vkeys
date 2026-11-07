# 007 - Verify ARM/macOS artifacts

Status: open · Priority: P1 · Area: release

## Context
The ARM/macOS jobs ran green in CI on 2026-11-05 (run `42022173611`): the
macOS universal passed its two-slice lipo check on `macos-15`, and Windows ARM64
passed its `0xAA64` PE assertion on `windows-11-arm`. The remaining gap is the
human half: the macOS quarantine/xattr flow on a browser download, and a smoke
test on a Snapdragon PC (the CI runner is not one). The artifacts have also not
been rebuilt at the release commit.

## Acceptance
- [ ] Run `release-artifacts.yml` manually after the public flip.
- [ ] macOS `lipo -archs` shows both slices; quarantine/xattr flow documented.
- [ ] Windows ARM PE is `0xAA64`; smoke test on a Snapdragon PC.
- [ ] Record results.

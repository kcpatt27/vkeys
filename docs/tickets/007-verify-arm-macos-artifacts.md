# 007 - Verify ARM/macOS artifacts

Status: open · Priority: P1 · Area: release

## Context
The ARM/macOS jobs ran green in CI on 2026-11-05 (run `42022173611`): the
macOS universal passed its two-slice lipo check on `macos-15`, and Windows ARM64
passed its `0xAA64` PE assertion on `windows-11-arm`. The remaining gap is the
human half: the macOS quarantine/xattr flow on a browser download, and a smoke
test on a Snapdragon PC (the CI runner is not one). The artifacts have also not
been rebuilt at the release commit.

## Static verification 2026-11-10
- Run `42697091248` at `3a493cc`: all six jobs green.
- macOS binaries are fat Mach-O with `nfat_arch=2` (x86_64 + arm64); the tarball
  keeps `rwxr-xr-x`.
- Windows ARM64 executables parse to PE machine `0xAA64`.
- All six archives contain both binaries, `LICENSE`, `README`, and a matching
  SHA-256.

Remaining human checks: macOS quarantine/`xattr` on a browser download, a
Snapdragon smoke test, and a run against the `v0.1.0` tag after the public flip.

## Acceptance
- [ ] Run `release-artifacts.yml` manually after the public flip.
- [ ] macOS `lipo -archs` shows both slices; quarantine/xattr flow documented.
- [x] Windows ARM PE is `0xAA64` (static check).
- [ ] Smoke test on a Snapdragon PC.
- [x] Record the static results (run `42697091248`).

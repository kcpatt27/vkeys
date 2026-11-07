# Releasing

vkeys is kept **private** until release day. Order: registry first, host
visibility last.

## Pre-flight

```powershell
cd rust
cargo test --release          # Windows; CI covers macOS/Linux after the flip
cargo publish --dry-run
cd ..
vkeys selfcheck
```

**CI is manual while the repo is private.** `ci.yml` has only
`workflow_dispatch`; `release-artifacts.yml` runs manually or on a published
release. After the public flip, re-add `push`/`pull_request` to `ci.yml` and run
it once.

- Confirm no secrets in history: use the `publish-ready` scan.
- Confirm the working tree is clean and `main` is pushed to `origin` (local
  relay) and `github`.

## Tag

```powershell
git tag -a v0.1.0 -m "vkeys 0.1.0"
git push github main
git push github v0.1.0
```

## Publish to crates.io

The token lives in vkeys as `crates/vkeys`. The profile is human-only:

```powershell
cd rust
vkeys run publish-vkeys
```

`vkeys run` injects `CARGO_REGISTRY_TOKEN` into `cargo publish`; an agent cannot
run it without a human typing the profile name. The crate is published from
`rust/`.

## Release artifacts

`release-artifacts.yml` builds, but does not publish:

- Linux ARM64: `aarch64-unknown-linux-gnu` + `aarch64-unknown-linux-musl`
  (Raven tested both; ship musl).
- macOS: universal (`x86_64` + `arm64`) via `lipo`, with a slice assertion.
- Windows ARM64: native `windows-11-arm`, PE machine asserted as `0xAA64`.
- Linux x64: `x86_64-unknown-linux-musl` via `cross`, on `ubuntu-latest`.
- Windows x64: `x86_64-pc-windows-msvc` on `windows-latest`, runs
  `cargo test --release` before building.

The two x64 jobs package both binaries plus `LICENSE` and `README`, and each
emits a SHA-256 sidecar (`<archive>.sha256`). The ARM64 Linux and Windows ARM64
jobs package the binaries only and emit no checksum; the macOS job packages the
binaries only but does emit a `.sha256` sidecar. Add the licence, readme, and a
checksum to any archive before shipping it.

Run it manually, or let a published GitHub release trigger it. Download the
artifacts and attach them to the release.

**macOS Gatekeeper:** unsigned downloads fetched by a browser get
`com.apple.quarantine` and are refused. `cargo install` and `curl | tar` do not
set it. Document the quarantine removal and publish a SHA-256:

```bash
xattr -dr com.apple.quarantine vkeys vkeys-broker   # only for browser downloads
```

Developer ID signing + notarization (Apple Developer Program, $99/yr) is a later
trust upgrade, not a build requirement.

## Flip the host (only after the crate is live)

```powershell
gh repo edit kcpatt27/vkeys --visibility public --accept-visibility-change-consequences
```

Then create a GitHub release from the tag.

## Rollback

A published version cannot be deleted, only yanked:

```powershell
# CARGO_REGISTRY_TOKEN must be present; add a yank profile if this becomes routine
cargo yank --version 0.1.0 vkeys
```

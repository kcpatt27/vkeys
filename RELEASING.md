# Releasing

vkeys is kept **private** until release day. Order: registry first, host
visibility last.

## Pre-flight

```powershell
cd rust
cargo test --release          # Windows; CI covers macOS/Linux
cargo publish --dry-run
cd ..
vkeys selfcheck
```

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

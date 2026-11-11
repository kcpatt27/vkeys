# vkeys 0.1.0

Release date: Thursday, 2026-11-12.

License: Apache-2.0.

vkeys is a personal, machine-wide key store for Windows, macOS, and Linux. Store
a key once; profiles inject it into a fixed command. The agent can use the key,
but there is no `get`, `print`, or `export-env`.

## What ships

- `vkeys`: `set`, `ls`, `env`, `run`, `rotate`, `audit`, `import-env`, `init`,
  and `selfcheck`.
- `vkeys-broker`: a local OpenRouter proxy with broker-token auth, an in-memory
  budget, per-request audit, and non-streaming `POST /v1/chat/completions` only.
- Vault backends: Windows DPAPI for Tier 1, or XChaCha20-Poly1305 with the master
  key in Keychain, Secret Service, or a `0600` file on headless Linux.
- Fixed-profile `run`; no generic `run -- <argv>`; no command that prints a
  value.

## Verified

- Windows: elevated SYSTEM install, boot start, `/healthz` 200, ACL boundary,
  and a real broker completion. The agent cannot read the vault or master key;
  it can read only `%APPDATA%\vkeys\broker.token`.
- Linux ARM64 on Raven: dedicated `vkeys` system user, service active, boundary
  holds, broker completion returned 200.
- Release archives: Linux ARM64 gnu and musl, macOS universal, Windows ARM64,
  Linux x64 musl, and Windows x64 msvc. Each archive contains both binaries,
  `LICENSE`, and `README`; each has a matching `.sha256` sidecar.
- `cargo test --release`: 3 unit, 19 broker, and 3 CLI tests pass.
- `cargo publish --dry-run` is clean; the `vkeys` crate name is available.

## Boundaries

- Tier 1, same user: the vault is encrypted at rest, but a process running as
  you can ask the OS to decrypt it. It makes accidental exposure hard, not
  impossible.
- Tier 2, separate OS user: run `vkeys-broker` as a dedicated user. The agent
  cannot read the vault or master key; it holds a use-only broker token. This is
  the strong boundary.
- The audit log is local and append-only, not tamper-proof.
- macOS builds are unsigned. Browser downloads need the quarantine removal in
  `RELEASING.md`; Developer ID signing is post-0.1.
- There is no streaming, no per-client budget, no portable vault, and no
  transport in 0.1.

## Not in 0.1

- Transport push/receive. It is deferred to 0.2; ticket 018 is analysis-first,
  and ticket 001 is the privileged design.
- Broker lifecycle tray, per-client budgets, portable vault export/import, vault
  locking, and macOS notarization.

## Install

```powershell
cd rust
cargo build --release
Copy-Item target\release\vkeys.exe,target\release\vkeys-broker.exe "$env:USERPROFILE\.local\bin\" -Force
```

```bash
cargo build --release
install -m 755 target/release/vkeys target/release/vkeys-broker ~/.local/bin/
```

`cargo install vkeys` also installs both binaries. Prebuilt archives are attached
to the GitHub release.

## Verify

```powershell
vkeys selfcheck
```

Expected: `selfcheck ok: crypto roundtrip, no plaintext, scrub, profile parse`.

Check the sidecar before running a downloaded archive:

```powershell
Get-FileHash .\vkeys-v0.1.0-x86_64-pc-windows-msvc.zip -Algorithm SHA256
Get-Content .\vkeys-v0.1.0-x86_64-pc-windows-msvc.zip.sha256
```

The digest must match.

## Documentation

- `README.md`: model, profiles, install, store paths.
- `SECURITY.md`: threat model and boundaries.
- `docs/BROKER.md`: Tier 2 broker setup and API.
- `docs/PI.md` and `deploy/`: Raven and Windows deployment.
- `RELEASING.md`: tag, publish, artifacts, flip, rollback.

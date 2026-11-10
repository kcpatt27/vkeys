# 008 - Claims + vault note update

Status: done · Priority: P0 · Area: docs

## Context
README/SECURITY claim Tier 2 + separate OS user. The vault note still needs the
Windows SYSTEM installer, the transport decision, and the release state.

## Done 2026-11-10
- README and SECURITY scope Tier 1 to human use and Tier 2 to a separate OS user.
- The external vault note is updated with the Windows SYSTEM boot/running
  evidence, the transport decision, and the release state.
- Release notes: `docs/RELEASE-0.1.0.md`.

## Acceptance
- [x] README: Tier 1 = human use; Tier 2 separate user = agent no-see.
- [x] Update `<vault>/projects/secrets-store/secrets-store.md` with Windows
  SYSTEM, transport decision (no agent push), release status.
- [x] No doc claims an unimplemented guarantee.

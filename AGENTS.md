# AGENTS.md — vkeys conventions

## What this is

Personal, machine-wide key store (`vkeys` = virtual keys). See `README.md` and
`DECISIONS.md`. Windows-only (DPAPI).

## Non-negotiables

- **Never add a `get`/`print`/`export-env` command.** The store is write-only for
  values. `env` prints references (`<stored: name>`), never values.
- **Never add a generic `run -- <argv>`.** Profiles carry a fixed `cmd`. A caller
  must not be able to substitute an interpreter expression.
- **Values never appear in output, logs, tests, or commits.** Tests use fake
  strings; the vault is encrypted at rest.
- **Audit every run**, including refusals (human-only profile, missing key).

## Git

- Never commit on `main`; branch per change (`feat/…`, `fix/…`), merge
  `--ff-only`, delete the branch. Worktree for parallel work.
- No secrets in the repo, ever. `profiles.toml` is local and git-ignored.

## Gate

```powershell
cd rust; cargo test --release
cd ..; vkeys selfcheck
```

The Python oracle is archived at tag `python-oracle-final`.

A merge that lands red is not merged.

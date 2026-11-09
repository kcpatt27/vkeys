# History notes

Decisions about what remains in the git history. Newest first.

## Personal identifiers before 2026-11-09

Credential scan: no token, provider key, password, or private key value was
found in any commit. The history does contain the operator's local OS account
name and absolute local paths in docs, install scripts, and the 2026-11-07
readiness report. Those strings were genericized in the tree on 2026-11-09.

Decision: **accepted for publication; history was not rewritten.** The strings
are a local account name and file paths, not credentials or device identifiers,
and the repository owner already publishes the same GitHub handle. A rewrite
would invalidate existing clones for no security gain.

If the account name must be removed later, rewrite history before the public
flip, not after.

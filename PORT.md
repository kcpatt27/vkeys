# vkeys — Python → Rust port

**Flipped 2026-10-28.** Rust is the installed default (`vkeys.exe` +
`vkeys-broker.exe` in `%USERPROFILE%\.local\bin`). Python is retained in `src/`
as the parity oracle.

## Port card

```text
Source:       src/vkeys/*.py @ 0066b83 (cli.py, vault.py, runtime.py, dpapi.py,
              __init__.py; 619 LOC)
Target:       Rust 1.99.1 (MSVC), crate rust/, package vkeys, binary vkeys.exe
Entry points: --version; --vault PATH; --profiles PATH; init;
              set NAME [--clipboard|--stdin] [--yes]; ls; env PROFILE;
              run PROFILE; rotate NAME [--clipboard|--stdin];
              import-env PATH [--names a,b]; audit [--tail N]; selfcheck
Oracle:       parity/run.ps1 (29 cases, byte comparison of stdout/stderr/exit +
              vault/audit state) and `cargo test` (3 cross-impl interop +
              3 CLI-contract tests)
Slices:       1 dpapi (OS edge) -> 2 vault reader/writer -> 3 profiles/scrub/
              audit -> 4 CLI/argv/exit -> 5 child process/env/scrub -> 6 none ->
              7 Windows edges (DPAPI, merged pipe, ppid, icacls, local time)
Deviations:   D2-D8 below, each with a reason and a local ticket id
Swap:         done 2026-10-28; real workload passed 2026-10-27 (see "Swap
              evidence"). Rust binaries installed; Python is the oracle.
```

## Build

```powershell
cd rust
cargo build --release        # -> rust/target/release/vkeys.exe
```

## Parity

```powershell
pwsh parity/run.ps1          # prints PARITY n/N; nonzero on any diff
```

The harness runs both implementations on **temp vaults and temp profiles only**.
It never reads/writes `C:\ProgramData\secrets` or the real `%APPDATA%\vkeys`.
Fixtures are seeded with the old implementation, never with the new one.

The Python side runs through `parity/py_shim.py` rather than `python -m vkeys`:
the package has no `__main__.py`, and the shim is also what makes getpass read
a piped stdin (D3). The shim only replaces `sys.stdin` and calls the untouched
`vkeys.cli.main`.

Compared per step: raw stdout bytes, raw stderr bytes, exit code. Compared after
the last step: vault JSON (names, `uses`, `last_used` presence, `set_at` shape),
audit JSONL (key-order-insensitive), and the file tree.

Normalised nondeterminism, and nothing else: timestamps (`<TS>`), parent pid
(`<PPID>`), DPAPI ciphertext (`<DPAPI>`), and each side's temp sandbox root
(`<TMP>`). DPAPI includes a random salt, so vault files can never be
byte-identical; the real oracle is the interop matrix below.

## Interop matrix

```powershell
cd rust
cargo test --release         # 3 interop + 3 CLI-contract tests
```

- `python_vault_read_by_rust` — Python writes a vault (plain, unicode, empty
  value); Rust `ls`, `env` (key order preserved), and `run` read it. `ls`
  output is byte-identical to Python's on the same file. `env` mutates the same
  `uses`/`last_used` as Python.
- `rust_vault_read_by_python` — Rust `set`/`import-env` write DPAPI blobs
  (including an empty plaintext); Python `Vault.get`, `names()`, `meta()` read
  them, and Python's `ls` is byte-identical to Rust's.
- `both_readers_see_same_vault_shape` — `names()`/`meta()` shape from Python is
  identical for Python-written and Rust-written vaults.

## Swap evidence

2026-10-27, real vault, real profiles, real secret:

```text
vkeys.exe set openrouter/meridian      -> stored
vkeys.exe env meridian                 -> OPENROUTER_API_KEY=<stored: openrouter/meridian>
vkeys.exe run meridian                 -> Meridian-Whisper started, Whisper large-v3-turbo
                                          on CUDA, hotkey ready
vkeys.exe set openrouter/eyewright     -> stored
vkeys.exe set pypi/eyewright           -> stored
vkeys.exe ls                           -> 3 entries; meridian uses=2
```

`run` exercised the clean-env allowlist, DPAPI decrypt, child spawn, and audit
write in one go. The three stores were then readable by the Python build
(interop matrix), so the swap is no longer blocked on behavior.

## Tier 2 broker (Rust)

`src/broker.rs` + `src/broker_main.rs` port `src/vkeys/broker.py` 1:1:
OpenRouter chat-completions only, broker-token auth, in-memory budget, one audit
line per request, no streaming, no prompt/completion text in logs. `cargo test
--release` runs 7 broker tests against a local fake upstream (mirroring
`tests/test_broker.py`); no live provider call.

## Deviations

| id | deviation | reason |
|---|---|---|
| D2 | `--help`/usage text is clap's, not argparse's. argparse's `--vau` prefix abbreviations and `-V` do not exist. `--version` output is byte-identical (`vkeys 0.1.0`), and usage errors exit 2 in both. | clap is the allowed CLI dep; help text is not part of the compared contract. |
| D3 | **Piped input is explicit.** `set`/`rotate` accept `--stdin`; without it and without a TTY the binary refuses with exit 2. Python's `getpass` reads the console directly and hangs on a pipe, so the harness runs Python through `parity/py_shim.py` and drops Python's `value (hidden): ` prompt (Rust's `--stdin` path is silent). `--stdin` and `--clipboard` are mutually exclusive. | A silent pipe read lets `echo $KEY \| vkeys set k` put a secret in shell history by accident. Opting in is safer and makes the agent path scriptable. Python has no equivalent flag, so the invocation differs while the stored outcome matches. |
| D4 | Unhandled failures (bad TOML/JSON, missing import file, spawn failure) print `vkeys: <error>` and exit 1; Python prints a traceback and exits 1. | Exit codes match; stderr shape is not compared by any fixture. |
| D5 | Vault JSON: both now write **insertion order** (Rust uses `IndexMap`), CRLF, 2-space indent, no trailing newline, same field order. The remaining difference is non-ASCII names: Rust writes raw UTF-8, Python `\uXXXX`. | Order is now identical; escaping is not observable through the CLI, and DPAPI blobs already make byte equality impossible. |
| D6 | Rust writes stdout/stderr as UTF-8; Python uses the console/locale code page when redirected. | The harness sets `PYTHONIOENCODING=utf-8`; ASCII output is byte-identical. |
| D7 | Rust prints paths as supplied; Python's `pathlib` normalises `/` to `\` and collapses `.`/repeated separators. | The harness supplies canonical absolute Windows paths; file I/O is unaffected. |
| D8 | TTY-only flows (overwrite `[y/N]`, human-only typed confirmation) are implemented but not machine-compared. | No pty harness; both code paths mirror the Python prompts and exit codes. Recorded as a test gap, not a claimed pass. |

D1 (clipboard) is closed: `arboard` reads the Windows clipboard, text only
(`default-features = false`, so the `image` stack stays out). On failure it
prints Python's line and falls back to the hidden prompt.

Local ticket ids are `port-d2`…`port-d8`; this repo has no issue tracker.

## Do not add `get`

The store is write-only for values. There is no `get`, `print`, `export-env`, or
generic `run -- <argv>` in either implementation, and the port must not add one.
`env` prints `<ENV>=<stored: name>` references only. `Vault::get` is internal,
used by `env`/`run`/`selfcheck`, exactly like Python's `Vault.get`.

## Dependencies

| dep | version | why |
|---|---|---|
| clap | 4.6.7 | derive-based CLI matching the entry points; exit 2 on usage errors |
| serde + serde_json | 1.0.229 / 1.0.154 | vault JSON read/write |
| indexmap | 2.14.2 | vault entries keep insertion order on disk, like Python's dict |
| toml | 0.8.23 (`preserve_order`) | profile parsing; document order is user-visible in `env`. Pulls `indexmap` transitively. |
| base64 | 0.22.1 | vault `ct` encoding |
| windows | 0.62.2 | DPAPI (`Win32_Security_Cryptography`), Toolhelp (ppid), pipes (merged child stdout/stderr), `GetLocalTime` |
| tiny_http | 0.12 | broker HTTP server (Tier 2) |
| ureq | 2.12.1 | broker upstream HTTPS client (Tier 2) |
| rpassword | 7.5.4 | hidden input on a real console |
| arboard | 3.7.0, `default-features = false` | text-only clipboard read for `--clipboard`; the default feature would pull the `image` stack |

No async runtime. `parity/` uses only the Python standard library. The Rust
binary is statically linked (MSVC runtime is the system one), so it adds no
runtime dependency.

## Known edges / out of scope

- No locking in either implementation; concurrent writers can lose an update.
- `icacls` is invoked on `init` only when the parent directory was created, and
  its output/exit code is ignored, matching Python. The ACL bits themselves are
  not compared.
- Malformed vault/profile files exit 1 in both, with different stderr text (D4).
- TTY-only confirmations are not exercised (D8).
- Tier 2 is one provider (OpenRouter) and non-streaming; no per-client budgets.
- Performance and CI are out of scope.

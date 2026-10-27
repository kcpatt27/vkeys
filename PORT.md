# vkeys — Python → Rust port

Branch: `feat/rust-port`. **Python is still the default implementation.** Nothing
here is merged to `main`, and the Python source is untouched.

## Port card

```text
Source:       src/vkeys/*.py @ 0066b83 (cli.py, vault.py, runtime.py, dpapi.py,
              __init__.py; 619 LOC)
Target:       Rust 1.99.1 (MSVC), crate rust/, package vkeys, binary vkeys.exe
Entry points: --version; --vault PATH; --profiles PATH; init;
              set NAME [--clipboard] [--yes]; ls; env PROFILE; run PROFILE;
              rotate NAME [--clipboard]; import-env PATH [--names a,b];
              audit [--tail N]; selfcheck
Oracle:       parity/run.ps1 (29 cases, byte comparison of stdout/stderr/exit +
              vault/audit state) and rust/tests/interop.rs (3 cross-impl tests)
Slices:       1 dpapi (OS edge) -> 2 vault reader/writer -> 3 profiles/scrub/
              audit -> 4 CLI/argv/exit -> 5 child process/env/scrub -> 6 none ->
              7 Windows edges (DPAPI, merged pipe, ppid, icacls, local time)
Deviations:   D1-D8 below, each with a reason and a local ticket id
Swap:         not started. Python stays default until a real workload has run
              through the Rust binary. Rust ships only at
              rust/target/release/vkeys.exe on this branch.
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
cargo test --release         # 3 tests
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

## Deviations

| id | deviation | reason |
|---|---|---|
| D1 | `--clipboard` always prints `clipboard unavailable; falling back to hidden prompt` and prompts; it never reads the Windows clipboard. | Python uses tkinter; no allowed clipboard crate, and the card permits the fallback. Python's own Tk failure path prints the same line. |
| D2 | `--help`/usage text is clap's, not argparse's. argparse's `--vau` prefix abbreviations and `-V` do not exist. `--version` output is byte-identical (`vkeys 0.1.0`), and usage errors exit 2 in both. | clap is the allowed CLI dep; help text is not part of the compared contract. |
| D3 | When stdin is not a TTY, Rust `set`/`rotate` read the pipe. Python's `getpass` reads the console directly on Windows and hangs on a pipe, so the harness runs Python through `parity/py_shim.py` (replaces `sys.stdin` so getpass uses its documented fallback) and drops Python's `Warning: Password input may be echoed.` notice. Both then print the same `value (hidden): ` prompt. | The card requires a stdin-piped `set` case; CPython's console-only read is an interpreter artifact, not a product contract. |
| D4 | Unhandled failures (bad TOML/JSON, missing import file, spawn failure) print `vkeys: <error>` and exit 1; Python prints a traceback and exits 1. | Exit codes match; stderr shape is not compared by any fixture. |
| D5 | Vault JSON: Rust writes entries sorted by name (BTreeMap) and non-ASCII names as raw UTF-8; Python writes insertion order and `\uXXXX`. Both keep CRLF line endings, 2-space indent, no trailing newline, and the same field order/shape. | Order/escaping are not observable through the CLI (`names()` sorts; JSON readers accept both); DPAPI blobs already make byte equality impossible. |
| D6 | Rust writes stdout/stderr as UTF-8; Python uses the console/locale code page when redirected. | The harness sets `PYTHONIOENCODING=utf-8`; ASCII output is byte-identical. |
| D7 | Rust prints paths as supplied; Python's `pathlib` normalises `/` to `\` and collapses `.`/repeated separators. | The harness supplies canonical absolute Windows paths; file I/O is unaffected. |
| D8 | TTY-only flows (overwrite `[y/N]`, human-only typed confirmation) are implemented but not machine-compared. | No pty harness; both code paths mirror the Python prompts and exit codes. Recorded as a test gap, not a claimed pass. |

Local ticket ids are `port-d1`…`port-d8`; this repo has no issue tracker.

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
| toml | 0.8.23 (`preserve_order`) | profile parsing; document order is user-visible in `env`. Pulls `indexmap` transitively. |
| base64 | 0.22.1 | vault `ct` encoding |
| windows | 0.62.2 | DPAPI (`Win32_Security_Cryptography`), Toolhelp (ppid), pipes (merged child stdout/stderr), `GetLocalTime` |
| rpassword | 7.5.4 | hidden input on a real console |

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
- Performance, CI, new commands, and a swap/rename release are out of scope.

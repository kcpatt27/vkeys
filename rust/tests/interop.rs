//! Cross-implementation interop matrix. DPAPI blobs are nondeterministic, so
//! the oracle is "the other implementation reads it", not byte equality:
//!   python writes -> rust ls/env/run reads
//!   rust writes   -> python Vault.get/names/meta reads
//!   both print the same ls/env bytes for the same vault
#![cfg(windows)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const BIN: &str = env!("CARGO_BIN_EXE_vkeys");

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_dir(tag: &str) -> TempDir {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!(
        "vkeys-interop-{}-{tag}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    TempDir(path)
}

fn py_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("src")
}

fn python() -> String {
    std::env::var("VKEYS_PYTHON").unwrap_or_else(|_| "python".to_string())
}

fn run_py(code: &str, args: &[&str]) -> Output {
    Command::new(python())
        .arg("-c")
        .arg(code)
        .args(args)
        .env("PYTHONPATH", py_src())
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONWARNINGS", "ignore")
        .output()
        .expect("python")
}

fn run_py_main(args: &[&str]) -> Output {
    run_py(
        "import sys; from vkeys.cli import main; sys.exit(main())",
        args,
    )
}

fn run_vkeys(args: &[&str]) -> Output {
    Command::new(BIN).args(args).output().expect("vkeys")
}

fn run_vkeys_stdin(args: &[&str], stdin: &[u8]) -> Output {
    let mut child = Command::new(BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn vkeys");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin)
        .expect("write stdin");
    child.wait_with_output().expect("wait vkeys")
}

fn s(path: &Path) -> &str {
    path.to_str().unwrap()
}

fn seed_python_vault(vault: &Path, entries: &[(&str, &str)]) {
    for (name, value) in entries {
        let out = run_py(
            "import sys; from vkeys.vault import Vault; Vault(sys.argv[1]).set(sys.argv[2], sys.argv[3])",
            &[s(vault), name, value],
        );
        assert!(
            out.status.success(),
            "python seed failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

const INSPECT: &str = "\
import json, re, sys
from vkeys.vault import Vault
v = Vault(sys.argv[1])
out = {}
for n in v.names():
    m = v.meta(n)
    out[n] = {
        'uses': m['uses'],
        'last_used_set': m['last_used'] is not None,
        'set_at_ok': bool(re.fullmatch(r'\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}', m['set_at'] or '')),
        'ct_ok': bool(m['ct']),
    }
print(json.dumps({'names': v.names(), 'entries': out}, ensure_ascii=False, sort_keys=True))
";

#[test]
fn python_vault_read_by_rust() {
    let td = temp_dir("py-to-rs");
    let vault = td.0.join("vault.bin");
    let profiles = td.0.join("profiles.toml");
    seed_python_vault(
        &vault,
        &[
            ("k", "s3cret-value-123"),
            ("clé", "valeur-secrète-9"),
            ("empty", ""),
        ],
    );

    // ls: both implementations, same vault file -> identical bytes.
    let py_ls = run_py_main(&["--vault", s(&vault), "--profiles", s(&profiles), "ls"]);
    let rs_ls = run_vkeys(&["--vault", s(&vault), "--profiles", s(&profiles), "ls"]);
    assert!(py_ls.status.success() && rs_ls.status.success());
    assert_eq!(py_ls.stdout, rs_ls.stdout, "ls output differs");

    // env: rust reads python's DPAPI blobs; key order preserved (B before A).
    fs::write(
        &profiles,
        "[profile.t]\ncmd = [\"cmd\", \"/c\", \"echo hello\"]\nagent-ok = true\n\
         [profile.t.keys]\nB_KEY = \"clé\"\nA_KEY = \"k\"\nE_KEY = \"empty\"\n",
    )
    .unwrap();
    let rs_env_vault = td.0.join("rs-env.bin");
    fs::copy(&vault, &rs_env_vault).unwrap();
    let rs_env = run_vkeys(&[
        "--vault",
        s(&rs_env_vault),
        "--profiles",
        s(&profiles),
        "env",
        "t",
    ]);
    assert_eq!(
        std::str::from_utf8(&rs_env.stdout).unwrap(),
        "B_KEY=<stored: clé>\r\nA_KEY=<stored: k>\r\nE_KEY=<stored: empty>\r\n"
    );

    // python env on another copy -> same stdout; both mutate uses/last_used.
    let py_env_vault = td.0.join("py-env.bin");
    fs::copy(&vault, &py_env_vault).unwrap();
    let py_env = run_py_main(&[
        "--vault",
        s(&py_env_vault),
        "--profiles",
        s(&profiles),
        "env",
        "t",
    ]);
    assert_eq!(py_env.stdout, rs_env.stdout, "env output differs");

    for copy in [&rs_env_vault, &py_env_vault] {
        let inspected = run_py(INSPECT, &[s(copy)]);
        assert!(inspected.status.success());
        let json = String::from_utf8_lossy(&inspected.stdout);
        assert!(json.contains("\"uses\": 1"), "{json}");
        assert!(json.contains("\"last_used_set\": true"), "{json}");
        assert!(json.contains("\"set_at_ok\": true"), "{json}");
    }

    // run: rust executes a profile whose value came from python's vault.
    let rs_run_vault = td.0.join("rs-run.bin");
    fs::copy(&vault, &rs_run_vault).unwrap();
    let run = run_vkeys(&[
        "--vault",
        s(&rs_run_vault),
        "--profiles",
        s(&profiles),
        "run",
        "t",
    ]);
    assert!(run.status.success());
    assert_eq!(std::str::from_utf8(&run.stdout).unwrap(), "hello\r\n");
}

#[test]
fn rust_vault_read_by_python() {
    let td = temp_dir("rs-to-py");
    let vault = td.0.join("vault.bin");
    let profiles = td.0.join("profiles.toml");

    let set = run_vkeys_stdin(
        &[
            "--vault",
            s(&vault),
            "--profiles",
            s(&profiles),
            "set",
            "--stdin",
            "k",
        ],
        b"rust-secret-456\n",
    );
    assert!(
        set.status.success(),
        "{}",
        String::from_utf8_lossy(&set.stderr)
    );
    assert_eq!(std::str::from_utf8(&set.stdout).unwrap(), "stored k\r\n");

    let set_unicode = run_vkeys_stdin(
        &[
            "--vault",
            s(&vault),
            "--profiles",
            s(&profiles),
            "set",
            "--stdin",
            "clé",
        ],
        "valeur-secrète-9\n".as_bytes(),
    );
    assert!(set_unicode.status.success());

    // Python decrypts rust's DPAPI blobs and sees the documented meta shape.
    for (name, expected) in [("k", "rust-secret-456"), ("clé", "valeur-secrète-9")] {
        let got = run_py(
            "import sys; from vkeys.vault import Vault; print(Vault(sys.argv[1]).get(sys.argv[2]), end='')",
            &[s(&vault), name],
        );
        assert!(
            got.status.success(),
            "{}",
            String::from_utf8_lossy(&got.stderr)
        );
        assert_eq!(std::str::from_utf8(&got.stdout).unwrap(), expected);
    }

    // Empty plaintext is a valid DPAPI payload and must round-trip too.
    let envfile = td.0.join("empty.env");
    fs::write(&envfile, "EMPTY=\n").unwrap();
    let imported = run_vkeys(&[
        "--vault",
        s(&vault),
        "--profiles",
        s(&profiles),
        "import-env",
        s(&envfile),
    ]);
    assert!(imported.status.success());
    assert_eq!(
        std::str::from_utf8(&imported.stdout).unwrap(),
        "imported EMPTY\r\n"
    );
    let empty = run_py(
        "import sys; from vkeys.vault import Vault; print(Vault(sys.argv[1]).get(sys.argv[2]), end='')",
        &[s(&vault), "EMPTY"],
    );
    assert!(empty.status.success());
    assert_eq!(empty.stdout, b"");

    let inspected = run_py(INSPECT, &[s(&vault)]);
    assert!(
        inspected.status.success(),
        "{}",
        String::from_utf8_lossy(&inspected.stderr)
    );
    let json = String::from_utf8_lossy(&inspected.stdout);
    assert!(
        json.contains("\"names\": [\"EMPTY\", \"clé\", \"k\"]"),
        "{json}"
    );
    assert!(json.contains("\"uses\": 0"), "{json}");
    assert!(json.contains("\"last_used_set\": false"), "{json}");
    assert!(json.contains("\"set_at_ok\": true"), "{json}");
    assert!(json.contains("\"ct_ok\": true"), "{json}");

    // Python ls and rust ls agree on the rust-written vault.
    let py_ls = run_py_main(&["--vault", s(&vault), "--profiles", s(&profiles), "ls"]);
    let rs_ls = run_vkeys(&["--vault", s(&vault), "--profiles", s(&profiles), "ls"]);
    assert!(py_ls.status.success() && rs_ls.status.success());
    assert_eq!(py_ls.stdout, rs_ls.stdout, "ls output differs");
}

#[test]
fn both_readers_see_same_vault_shape() {
    let td = temp_dir("shape");
    let py_vault = td.0.join("py.bin");
    let rs_vault = td.0.join("rs.bin");
    seed_python_vault(&py_vault, &[("a", "value-12345678")]);

    let out = run_vkeys_stdin(
        &[
            "--vault",
            s(&rs_vault),
            "--profiles",
            s(&td.0.join("p.toml")),
            "set",
            "--stdin",
            "a",
        ],
        b"value-12345678\n",
    );
    assert!(out.status.success());

    // names()/meta() shape is identical from Python's perspective.
    for vault in [&py_vault, &rs_vault] {
        let inspected = run_py(INSPECT, &[s(vault)]);
        assert!(inspected.status.success());
        let json = String::from_utf8_lossy(&inspected.stdout);
        assert!(json.contains("\"names\": [\"a\"]"), "{json}");
        assert!(json.contains("\"uses\": 0"), "{json}");
        assert!(json.contains("\"ct_ok\": true"), "{json}");
    }
}

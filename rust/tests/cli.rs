//! CLI contract tests for behavior that has no Python equivalent.
//!
//! Deviation D3: the Python build's `getpass` ignores a pipe (and hangs on
//! one), so the Rust build makes stdin explicit with `--stdin`. These checks
//! pin that contract; parity with Python is covered by parity/parity.py.
//!
//! Windows-only: these exercise the DPAPI-backed CLI end to end. The
//! cross-platform unit tests (crypto, broker) run everywhere.

#![cfg(windows)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn tmpdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vkeys-cli-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(dir: &Path, args: &[&str], stdin: Option<&str>) -> (i32, String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vkeys"));
    cmd.arg("--vault")
        .arg(dir.join("vault.bin"))
        .arg("--profiles")
        .arg(dir.join("profiles.toml"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });

    let mut child = cmd.spawn().expect("spawn vkeys");
    if let Some(text) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin pipe")
            .write_all(text.as_bytes())
            .expect("write stdin");
    }
    let out = child.wait_with_output().expect("wait vkeys");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn piped_stdin_without_flag_refuses() {
    let dir = tmpdir("refuse");
    let (code, _, err) = run(&dir, &["set", "k"], Some("piped-secret-123\n"));
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("no TTY"), "stderr: {err}");
    assert!(err.contains("--stdin"), "stderr: {err}");
    assert!(
        !dir.join("vault.bin").exists(),
        "a refused set must not create a vault"
    );
}

#[test]
fn stdin_flag_stores_the_value() {
    let dir = tmpdir("store");
    let (code, out, err) = run(&dir, &["set", "--stdin", "k"], Some("piped-secret-123\n"));
    assert_eq!(code, 0, "stderr: {err}");
    // Python's text-mode stdout writes os.linesep on Windows; the port matches
    // it byte for byte, so stdout is CRLF even through a pipe.
    assert_eq!(out, "stored k\r\n");

    let (code, out, err) = run(&dir, &["ls"], None);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(out.contains("k"), "ls output: {out}");
    assert!(!out.contains("piped-secret-123"), "ls must never print values");
}

#[test]
fn stdin_conflicts_with_clipboard() {
    let dir = tmpdir("conflict");
    let (code, _, err) = run(&dir, &["set", "--stdin", "--clipboard", "k"], Some("x\n"));
    assert_eq!(code, 2, "stderr: {err}");
    assert!(err.contains("cannot be used with"), "stderr: {err}");
}

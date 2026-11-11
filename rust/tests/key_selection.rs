//! Master-key selection and key/vault mismatch regressions.
//!
//! These tests drive the real binary so they exercise the same env/HOME
//! resolution as a service user. They use a temp key file and the portable
//! XChaCha20-Poly1305 scheme, so they run on Windows, macOS, and Linux.

use std::fs;
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn tmpdir(tag: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "vkeys-key-select-{tag}-{}-{stamp}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

struct Out {
    code: i32,
    out: String,
    err: String,
}

fn run(dir: &Path, args: &[&str], stdin: Option<&str>, envs: &[(&str, Option<&str>)]) -> Out {
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
    cmd.env("VKEYS_SCHEME", "xchacha20poly1305");
    for (key, value) in envs {
        match value {
            Some(value) => {
                cmd.env(key, value);
            }
            None => {
                cmd.env_remove(key);
            }
        }
    }

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
    Out {
        code: out.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&out.stdout).into_owned(),
        err: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn key_env(key: &Path) -> (&str, Option<&str>) {
    ("VKEYS_MASTER_KEY_FILE", key.to_str())
}

fn fixture(dir: &Path) -> PathBuf {
    let key = dir.join("key-a");
    let first = run(
        dir,
        &["set", "--stdin", "one"],
        Some("value-a\n"),
        &[key_env(&key)],
    );
    assert_eq!(first.code, 0, "fixture set failed: {}", first.err);
    key
}

#[test]
fn existing_vault_missing_key_refuses_without_mutation() {
    let dir = tmpdir("missing");
    let key = fixture(&dir);
    let vault = dir.join("vault.bin");
    let before = fs::read(&vault).unwrap();

    let empty = dir.join("empty-home");
    fs::create_dir_all(&empty).unwrap();
    let empty_text = empty.to_str().unwrap();

    // No VKEYS_MASTER_KEY_FILE, empty HOME/XDG/APPDATA, and no usable keyring.
    let out = run(
        &dir,
        &["rotate", "one", "--stdin"],
        Some("value-b\n"),
        &[
            ("VKEYS_MASTER_KEY_FILE", None),
            ("HOME", Some(empty_text)),
            ("XDG_CONFIG_HOME", Some(empty_text)),
            ("APPDATA", Some(empty_text)),
            ("USERPROFILE", Some(empty_text)),
            ("DBUS_SESSION_BUS_ADDRESS", Some("unix:path=/nonexistent")),
            ("XDG_RUNTIME_DIR", Some(empty_text)),
        ],
    );
    assert_ne!(out.code, 0, "missing key must fail");
    assert!(
        out.err.contains("VKEYS_MASTER_KEY_FILE"),
        "error must name the env var: {}",
        out.err
    );
    assert!(!empty.join("vkeys/master.key").exists());
    assert!(!empty.join(".config/vkeys/master.key").exists());
    assert!(!empty
        .join("Library/Application Support/vkeys/master.key")
        .exists());
    assert_eq!(fs::read(&vault).unwrap(), before, "vault must not change");

    // An explicit but missing path must name the expected path.
    let missing = dir.join("missing.key");
    let out = run(
        &dir,
        &["rotate", "one", "--stdin"],
        Some("value-b\n"),
        &[("VKEYS_MASTER_KEY_FILE", Some(missing.to_str().unwrap()))],
    );
    assert_ne!(out.code, 0);
    assert!(
        out.err.contains("VKEYS_MASTER_KEY_FILE"),
        "stderr: {}",
        out.err
    );
    assert!(
        out.err.contains(missing.to_str().unwrap()),
        "error must name the expected path: {}",
        out.err
    );
    assert!(!missing.exists(), "missing key must not be created");
    assert_eq!(fs::read(&vault).unwrap(), before, "vault must not change");

    let _ = key;
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn correct_key_rotation_succeeds() {
    let dir = tmpdir("correct");
    let key = fixture(&dir);
    let vault = dir.join("vault.bin");
    let before = fs::read(&vault).unwrap();

    let out = run(
        &dir,
        &["rotate", "one", "--stdin"],
        Some("value-b\n"),
        &[key_env(&key)],
    );
    assert_eq!(out.code, 0, "stderr: {}", out.err);
    assert_ne!(fs::read(&vault).unwrap(), before, "vault should change");

    let ls = run(&dir, &["ls"], None, &[key_env(&key)]);
    assert_eq!(ls.code, 0, "stderr: {}", ls.err);
    assert!(ls.out.contains("one"), "ls output: {}", ls.out);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn wrong_key_refuses_without_mutation() {
    let dir = tmpdir("wrong");
    let key = fixture(&dir);
    let vault = dir.join("vault.bin");
    let before = fs::read(&vault).unwrap();

    let other = tmpdir("wrong-other");
    let key_b = dir.join("key-b");
    let made = run(
        &other,
        &["set", "--stdin", "x"],
        Some("x\n"),
        &[key_env(&key_b)],
    );
    assert_eq!(made.code, 0, "key B setup failed: {}", made.err);

    let out = run(
        &dir,
        &["rotate", "one", "--stdin"],
        Some("value-b\n"),
        &[key_env(&key_b)],
    );
    assert_ne!(out.code, 0, "wrong key must fail");
    assert!(
        out.err.contains("key mismatch") || out.err.contains("does not decrypt"),
        "stderr: {}",
        out.err
    );
    assert_eq!(fs::read(&vault).unwrap(), before, "vault must not change");

    let _ = key;
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&other);
}

#[test]
fn selfcheck_reports_key_mismatch() {
    let dir = tmpdir("selfcheck");
    let key = dir.join("selfcheck.key");
    let out = run(&dir, &["selfcheck"], None, &[key_env(&key)]);
    assert_eq!(out.code, 0, "stderr: {}", out.err);
    assert!(
        out.out.contains("key mismatch detection"),
        "selfcheck output: {}",
        out.out
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn broker_decrypts_fixture_profile_keys() {
    let dir = tmpdir("broker");
    let key = fixture(&dir);
    for (name, value) in [
        ("openrouter/broker", "fake-provider-key"),
        ("broker/token", "fake-broker-token"),
    ] {
        let out = run(
            &dir,
            &["set", "--stdin", name],
            Some(&format!("{value}\n")),
            &[key_env(&key)],
        );
        assert_eq!(out.code, 0, "set {name} failed: {}", out.err);
    }

    let broker = option_env!("CARGO_BIN_EXE_vkeys-broker")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let exe = std::env::current_exe().unwrap();
            let base = exe.parent().unwrap().parent().unwrap();
            base.join(if cfg!(windows) {
                "vkeys-broker.exe"
            } else {
                "vkeys-broker"
            })
        });
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let broker_toml = broker.to_string_lossy().replace('\\', "\\\\");
    let profiles = format!(
        "[profile.broker]\ncmd = [\"{}\", \"--host\", \"127.0.0.1\", \"--port\", \"{}\", \"--budget-usd\", \"0\"]\nagent-ok = true\n[profile.broker.keys]\nOPENROUTER_API_KEY = \"openrouter/broker\"\nVKEYS_BROKER_TOKEN = \"broker/token\"\n",
        broker_toml, port
    );
    fs::write(dir.join("profiles.toml"), profiles).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_vkeys"))
        .arg("--vault")
        .arg(dir.join("vault.bin"))
        .arg("--profiles")
        .arg(dir.join("profiles.toml"))
        .arg("run")
        .arg("broker")
        .env("VKEYS_MASTER_KEY_FILE", &key)
        .env("VKEYS_SCHEME", "xchacha20poly1305")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn vkeys run broker");

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut healthy = false;
    while Instant::now() < deadline {
        if let Ok(resp) = ureq::get(&format!("http://127.0.0.1:{port}/healthz"))
            .timeout(Duration::from_millis(300))
            .call()
        {
            if resp.status() == 200 {
                healthy = true;
                break;
            }
        }
        if let Some(status) = child.try_wait().expect("try_wait") {
            let out = child.wait_with_output().expect("wait broker");
            panic!(
                "broker exited early: {status}\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("pkill")
            .args(["-f", &format!("vkeys-broker.*{port}")])
            .status();
        let _ = child.kill();
    }
    let _ = child.wait();
    assert!(healthy, "broker did not become healthy on port {port}");
    let _ = fs::remove_dir_all(&dir);
}

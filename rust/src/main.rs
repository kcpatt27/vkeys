//! vkeys CLI. No command prints a stored value.

mod dpapi;
mod proc;
mod runtime;
mod vault;

use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};

use runtime::{
    audit_refuse, default_profiles, load_profiles, py_repr, py_splitlines, py_strip, run_profile,
    scrub, LoadError, Profile,
};
use vault::{audit_path, default_vault, parent_dir, Vault};

#[derive(Parser)]
#[command(
    name = "vkeys",
    bin_name = "vkeys",
    about = "personal virtual-key store",
    disable_version_flag = true
)]
struct Cli {
    #[arg(long, value_name = "VAULT", help = "vault path")]
    vault: Option<PathBuf>,
    #[arg(long, value_name = "PROFILES", help = "profiles TOML")]
    profiles: Option<PathBuf>,
    #[arg(
        long = "version",
        action = clap::ArgAction::SetTrue,
        help = "show program's version number and exit"
    )]
    version: bool,
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    #[command(about = "create the vault + ACL (once)")]
    Init,
    #[command(about = "store a value (hidden paste)")]
    Set {
        name: String,
        #[arg(long, help = "read from the Windows clipboard")]
        clipboard: bool,
        #[arg(long, help = "overwrite without asking")]
        yes: bool,
    },
    #[command(about = "list names (never values)")]
    Ls,
    #[command(about = "print redacted references for a profile")]
    Env { profile: String },
    #[command(about = "run a profile's fixed command")]
    Run { profile: String },
    #[command(about = "replace a value (old is unrecoverable)")]
    Rotate {
        name: String,
        #[arg(long)]
        clipboard: bool,
    },
    #[command(
        name = "import-env",
        about = "import names from a .env without printing"
    )]
    ImportEnv {
        path: String,
        #[arg(long, default_value = "", help = "comma-separated; default: all")]
        names: String,
    },
    #[command(about = "show recent audit events")]
    Audit {
        #[arg(long, default_value_t = 20, allow_negative_numbers = true)]
        tail: i64,
    },
    #[command(about = "runnable check")]
    Selfcheck,
}

/// Write text to stdout with Python-on-Windows newline translation.
pub fn emit_out(text: &str) {
    let mut out = io::stdout().lock();
    let _ = out.write_all(text.replace('\n', "\r\n").as_bytes());
    let _ = out.flush();
}

fn emit_err(text: &str) {
    let mut err = io::stderr().lock();
    let _ = err.write_all(text.replace('\n', "\r\n").as_bytes());
    let _ = err.flush();
}

fn fail(message: &str) -> i32 {
    emit_err(&format!("vkeys: {message}\n"));
    1
}

fn main() {
    std::process::exit(cli_main());
}

fn cli_main() -> i32 {
    let cli = Cli::parse();
    if cli.version {
        emit_out(&format!("vkeys {}\n", env!("CARGO_PKG_VERSION")));
        return 0;
    }
    let Some(command) = cli.command else {
        emit_err("vkeys: error: a command is required\n");
        return 2;
    };
    let vault_path = cli.vault.unwrap_or_else(default_vault);
    let profiles_path = cli.profiles.unwrap_or_else(default_profiles);

    match command {
        Cmd::Init => cmd_init(&vault_path, &profiles_path),
        Cmd::Set {
            name,
            clipboard,
            yes,
        } => cmd_set(&vault_path, &name, clipboard, yes),
        Cmd::Ls => cmd_ls(&vault_path),
        Cmd::Env { profile } => cmd_env(&vault_path, &profiles_path, &profile),
        Cmd::Run { profile } => cmd_run(&vault_path, &profiles_path, &profile),
        Cmd::Rotate { name, clipboard } => cmd_rotate(&vault_path, &name, clipboard),
        Cmd::ImportEnv { path, names } => cmd_import_env(&vault_path, &path, &names),
        Cmd::Audit { tail } => cmd_audit(&vault_path, tail),
        Cmd::Selfcheck => cmd_selfcheck(),
    }
}

fn open_vault(path: &Path) -> Result<Vault, i32> {
    Vault::open(path).map_err(|e| fail(&e))
}

fn read_secret(clipboard: bool) -> Result<String, i32> {
    if clipboard {
        // Deviation: no tkinter equivalent; always fall back like Python does
        // when Tk is unavailable. See PORT.md.
        emit_err("clipboard unavailable; falling back to hidden prompt\n");
    }
    let value = if io::stdin().is_terminal() {
        rpassword::prompt_password("value (hidden): ").map_err(|e| fail(&e.to_string()))?
    } else {
        emit_err("value (hidden): ");
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) | Err(_) => return Err(1),
            Ok(_) => line,
        }
    };
    Ok(py_strip(&value).to_string())
}

fn confirm_overwrite(name: &str, yes: bool) -> Result<bool, i32> {
    if yes {
        return Ok(true);
    }
    if !io::stdin().is_terminal() {
        emit_err(&format!(
            "{name} exists; refusing to overwrite non-interactively (use --yes)\n"
        ));
        return Ok(false);
    }
    emit_out(&format!(
        "{name} exists; overwrite (old value is unrecoverable)? [y/N] "
    ));
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
    Ok(py_strip(&line).to_lowercase() == "y")
}

fn cmd_init(vault_path: &Path, profiles_path: &Path) -> i32 {
    let directory = parent_dir(vault_path);
    let created = !directory.exists();
    if let Err(e) = fs::create_dir_all(&directory) {
        return fail(&format!("{}: {e}", directory.display()));
    }
    if created {
        let user = std::env::var("USERNAME").unwrap_or_default();
        let _ = Command::new("icacls")
            .arg(&directory)
            .arg("/inheritance:r")
            .arg("/grant:r")
            .arg(format!("{user}:(OI)(CI)F"))
            .arg("/grant:r")
            .arg("*S-1-5-32-544:(OI)(CI)F")
            .output();
    }
    let vault = match open_vault(vault_path) {
        Ok(v) => v,
        Err(code) => return code,
    };
    if !vault_path.exists() {
        if let Err(e) = vault.save() {
            return fail(&e);
        }
    }
    emit_out(&format!("vault: {}\n", vault_path.display()));
    emit_out(&format!(
        "profiles: {}  (copy profiles.example.toml there)\n",
        profiles_path.display()
    ));
    0
}

fn cmd_set(vault_path: &Path, name: &str, clipboard: bool, yes: bool) -> i32 {
    let mut vault = match open_vault(vault_path) {
        Ok(v) => v,
        Err(code) => return code,
    };
    if vault.names().iter().any(|n| n == name) {
        match confirm_overwrite(name, yes) {
            Ok(true) => {}
            Ok(false) => return 1,
            Err(code) => return code,
        }
    }
    let value = match read_secret(clipboard) {
        Ok(v) => v,
        Err(code) => return code,
    };
    if value.is_empty() {
        emit_err("empty value; aborted\n");
        return 1;
    }
    if let Err(e) = vault.set(name, &value) {
        return fail(&e);
    }
    emit_out(&format!("stored {name}\n"));
    0
}

fn cmd_ls(vault_path: &Path) -> i32 {
    let vault = match open_vault(vault_path) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let names = vault.names();
    if names.is_empty() {
        emit_out(&format!("no keys in {}\n", vault.path.display()));
        return 0;
    }
    emit_out(&format!(
        "{:<32} {:<20} {:<20} {:>4}\n",
        "NAME", "SET", "LAST USED", "USES"
    ));
    for name in names {
        let meta = vault.meta(&name).cloned().unwrap_or_default();
        let set_at = meta
            .set_at
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "-".to_string());
        let last_used = meta
            .last_used
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "-".to_string());
        emit_out(&format!(
            "{:<32} {:<20} {:<20} {:>4}\n",
            name, set_at, last_used, meta.uses
        ));
    }
    0
}

fn resolve(
    vault_path: &Path,
    profiles_path: &Path,
    profile_name: &str,
) -> Result<(Profile, Vault, Vec<(String, String)>), i32> {
    let profiles = match load_profiles(profiles_path) {
        Ok(p) => p,
        Err(LoadError::Missing) => {
            emit_err(&format!(
                "no profiles file at {}; copy profiles.example.toml there\n",
                profiles_path.display()
            ));
            return Err(1);
        }
        Err(LoadError::Other(e)) => return Err(fail(&e)),
    };

    let mut have: Vec<String> = profiles.iter().map(|p| p.name.clone()).collect();
    have.sort();
    let Some(profile) = profiles.into_iter().find(|p| p.name == profile_name) else {
        let have = if have.is_empty() {
            "(none)".to_string()
        } else {
            have.join(", ")
        };
        emit_err(&format!(
            "unknown profile {}; have: {}\n",
            py_repr(profile_name),
            have
        ));
        return Err(1);
    };

    let vault = match open_vault(vault_path) {
        Ok(v) => v,
        Err(code) => return Err(code),
    };
    let names = vault.names();
    let mut missing = Vec::new();
    for (_, store) in &profile.keys {
        if !names.contains(store) {
            missing.push(store.clone());
        }
    }
    if !missing.is_empty() {
        emit_err(&format!(
            "missing keys: {} (run: vkeys set <name>)\n",
            missing.join(", ")
        ));
        return Err(1);
    }

    let mut env_map = Vec::new();
    for (env_name, store) in &profile.keys {
        match vault.get(store) {
            Ok(value) => env_map.push((env_name.clone(), value)),
            Err(e) => return Err(fail(&e)),
        }
    }
    Ok((profile, vault, env_map))
}

fn cmd_env(vault_path: &Path, profiles_path: &Path, profile_name: &str) -> i32 {
    let (profile, mut vault, _) = match resolve(vault_path, profiles_path, profile_name) {
        Ok(x) => x,
        Err(code) => return code,
    };
    for (env_name, store) in &profile.keys {
        emit_out(&format!("{env_name}=<stored: {store}>\n"));
        if let Err(e) = vault.touch(store) {
            return fail(&e);
        }
    }
    0
}

fn cmd_run(vault_path: &Path, profiles_path: &Path, profile_name: &str) -> i32 {
    let (profile, mut vault, env_map) = match resolve(vault_path, profiles_path, profile_name) {
        Ok(x) => x,
        Err(code) => return code,
    };
    let audit_file = audit_path(vault_path);

    if !profile.agent_ok {
        if !io::stdin().is_terminal() {
            audit_refuse(&audit_file, profile_name);
            emit_err(&format!(
                "refusing: profile {} is human-only (agent-ok = false)\n",
                py_repr(profile_name)
            ));
            return 2;
        }
        emit_out(&format!(
            "human-only profile {}; type the name to run: ",
            py_repr(profile_name)
        ));
        let mut line = String::new();
        let typed = match io::stdin().read_line(&mut line) {
            Ok(_) => py_strip(&line).to_string(),
            Err(_) => String::new(),
        };
        if typed != profile_name {
            emit_err("aborted (confirmation required for a human-only profile)\n");
            return 2;
        }
    }

    for (_, store) in &profile.keys {
        if let Err(e) = vault.touch(store) {
            return fail(&e);
        }
    }
    let (code, hits) = match run_profile(&profile, &env_map, &audit_file) {
        Ok(x) => x,
        Err(e) => return fail(&e),
    };
    if hits > 0 {
        emit_err(&format!(
            "vkeys: redacted {hits} secret occurrence(s) from output\n"
        ));
    }
    code
}

fn cmd_rotate(vault_path: &Path, name: &str, clipboard: bool) -> i32 {
    let mut vault = match open_vault(vault_path) {
        Ok(v) => v,
        Err(code) => return code,
    };
    if vault.names().iter().any(|n| n == name) {
        emit_out(&format!(
            "rotating {name}; the old value is unrecoverable\n"
        ));
    }
    let value = match read_secret(clipboard) {
        Ok(v) => v,
        Err(code) => return code,
    };
    if value.is_empty() {
        emit_err("empty value; aborted\n");
        return 1;
    }
    if let Err(e) = vault.set(name, &value) {
        return fail(&e);
    }
    emit_out(&format!("rotated {name}\n"));
    0
}

fn parse_env(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in py_splitlines(text) {
        let line = py_strip(line);
        if line.is_empty() || line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let (key, value) = line.split_once('=').unwrap();
        let key = py_strip(key).to_string();
        let value = py_strip(value)
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        match out.iter_mut().find(|(k, _)| k == &key) {
            Some(slot) => slot.1 = value,
            None => out.push((key, value)),
        }
    }
    out
}

fn cmd_import_env(vault_path: &Path, path: &str, names: &str) -> i32 {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => return fail(&format!("{path}: {e}")),
    };
    let text = String::from_utf8_lossy(&bytes);
    let parsed = parse_env(&text);
    let wanted: Vec<String> = if names.is_empty() {
        parsed.iter().map(|(k, _)| k.clone()).collect()
    } else {
        names.split(',').map(|n| py_strip(n).to_string()).collect()
    };
    let mut vault = match open_vault(vault_path) {
        Ok(v) => v,
        Err(code) => return code,
    };
    for name in wanted {
        match parsed.iter().find(|(k, _)| k == &name) {
            None => emit_err(&format!("skip {name}: not in {path}\n")),
            Some((_, value)) => {
                if let Err(e) = vault.set(&name, value) {
                    return fail(&e);
                }
                emit_out(&format!("imported {name}\n"));
            }
        }
    }
    0
}

fn cmd_audit(vault_path: &Path, tail: i64) -> i32 {
    let path = audit_path(vault_path);
    if !path.exists() {
        emit_out("no audit log yet\n");
        return 0;
    }
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) => return fail(&format!("{}: {e}", path.display())),
    };
    let text = String::from_utf8_lossy(&bytes);
    let lines = py_splitlines(&text);
    let n = lines.len();
    // Python slicing: lines[-tail:]; -0 == 0, negative tail drops from the front.
    let start = if tail > 0 {
        n.saturating_sub(tail as usize)
    } else if tail == 0 {
        0
    } else {
        tail.unsigned_abs() as usize
    };
    for line in lines.iter().skip(start.min(n)) {
        emit_out(&format!("{line}\n"));
    }
    0
}

fn cmd_selfcheck() -> i32 {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("vkeys-selfcheck-{}-{stamp}", std::process::id()));
    let result = (|| -> Result<(), String> {
        fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let vault_path = dir.join("vault.bin");
        let mut vault = Vault::open(&vault_path)?;
        let secret = "s3cret-value-123";
        vault.set("k", secret)?;
        if vault.get("k")? != secret {
            return Err("dpapi roundtrip mismatch".to_string());
        }
        let raw = fs::read(&vault_path).map_err(|e| e.to_string())?;
        if raw.windows(secret.len()).any(|w| w == secret.as_bytes()) {
            return Err("plaintext found in vault".to_string());
        }
        if vault.names() != vec!["k".to_string()] {
            return Err("unexpected names".to_string());
        }
        let (out, hits) = scrub(&format!("a {secret} b"), &[secret]);
        if hits != 1 || out.contains(secret) || !out.contains("[REDACTED]") {
            return Err("scrub check failed".to_string());
        }
        let profiles_path = dir.join("profiles.toml");
        fs::write(
            &profiles_path,
            "[profile.x]\ncmd = [\"cmd\"]\nagent-ok = true\n[profile.x.keys]\nK = \"k\"\n",
        )
        .map_err(|e| e.to_string())?;
        let profiles = match load_profiles(&profiles_path) {
            Ok(p) => p,
            Err(LoadError::Missing) => return Err("profile parse: missing".to_string()),
            Err(LoadError::Other(e)) => return Err(e),
        };
        let Some(profile) = profiles.iter().find(|p| p.name == "x") else {
            return Err("profile parse: x missing".to_string());
        };
        if profile.keys != vec![("K".to_string(), "k".to_string())] || !profile.agent_ok {
            return Err("profile parse mismatch".to_string());
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&dir);
    match result {
        Ok(()) => {
            emit_out("selfcheck ok: dpapi roundtrip, no plaintext, scrub, profile parse\n");
            0
        }
        Err(e) => fail(&e),
    }
}

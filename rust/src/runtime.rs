//! Profiles, clean-env injection, output scrubbing, auditing, time.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::SystemInformation::GetLocalTime;

// Base env a child legitimately needs; everything else is dropped.
pub const KEEP: [&str; 23] = [
    "PATH",
    "PATHEXT",
    "SYSTEMROOT",
    "SYSTEMDRIVE",
    "WINDIR",
    "COMSPEC",
    "TEMP",
    "TMP",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "PROGRAMFILES(X86)",
    "ALLUSERSPROFILE",
    "PUBLIC",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "OS",
    "USERNAME",
    "LANG",
];

pub fn default_profiles() -> PathBuf {
    if let Ok(v) = std::env::var("VKEYS_PROFILES") {
        return PathBuf::from(v);
    }
    let home = std::env::var("APPDATA")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
        .or_else(
            || match (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
                (Ok(d), Ok(p)) => Some(PathBuf::from(format!("{d}{p}"))),
                _ => None,
            },
        )
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("vkeys").join("profiles.toml")
}

pub fn now() -> String {
    let t = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond
    )
}

pub fn ppid() -> u32 {
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return 0;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let me = std::process::id();
        let mut parent = 0;
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                if entry.th32ProcessID == me {
                    parent = entry.th32ParentProcessID;
                    break;
                }
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
        parent
    }
}

// -- Python string semantics ------------------------------------------------

/// Python's `str.strip()` whitespace set (Rust is missing \x1c-\x1f).
fn py_whitespace(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

pub fn py_strip(s: &str) -> &str {
    s.trim_matches(py_whitespace)
}

/// Python's `str.splitlines()` (splits on more than \n).
pub fn py_splitlines(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (idx, c) = chars[i];
        let is_break = matches!(
            c,
            '\n' | '\r'
                | '\u{0b}'
                | '\u{0c}'
                | '\u{1c}'
                | '\u{1d}'
                | '\u{1e}'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        );
        if is_break {
            out.push(&s[start..idx]);
            let mut next = idx + c.len_utf8();
            if c == '\r' && i + 1 < chars.len() && chars[i + 1].1 == '\n' {
                i += 1;
                next = chars[i].0 + 1;
            }
            start = next;
        }
        i += 1;
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

/// Python's `repr(str)` for the strings this CLI prints (profile names).
pub fn py_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::new();
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

// -- profiles ---------------------------------------------------------------

pub struct Profile {
    pub name: String,
    pub cmd: Vec<String>,
    pub cwd: Option<String>,
    /// ENV_NAME -> store name, in TOML document order.
    pub keys: Vec<(String, String)>,
    pub agent_ok: bool,
}

pub enum LoadError {
    Missing,
    Other(String),
}

fn truthy(v: &toml::Value) -> bool {
    match v {
        toml::Value::Boolean(b) => *b,
        toml::Value::Integer(i) => *i != 0,
        toml::Value::Float(f) => *f != 0.0,
        toml::Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

pub fn load_profiles(path: &Path) -> Result<Vec<Profile>, LoadError> {
    let text = fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            LoadError::Missing
        } else {
            LoadError::Other(format!("{}: {e}", path.display()))
        }
    })?;
    let value: toml::Value = text
        .parse()
        .map_err(|e| LoadError::Other(format!("{}: {e}", path.display())))?;

    let mut out = Vec::new();
    let Some(profiles) = value.get("profile").and_then(|v| v.as_table()) else {
        return Ok(out);
    };
    for (name, prof) in profiles {
        let Some(table) = prof.as_table() else {
            return Err(LoadError::Other(format!(
                "profile {name}: expected a table"
            )));
        };
        let keys = match table.get("keys") {
            None => Vec::new(),
            Some(v) => {
                let Some(t) = v.as_table() else {
                    return Err(LoadError::Other(format!(
                        "profile {name}: 'keys' must be a table of ENV = store-name"
                    )));
                };
                let mut keys = Vec::new();
                for (env_name, store) in t {
                    keys.push((
                        env_name.clone(),
                        store.as_str().unwrap_or_default().to_string(),
                    ));
                }
                keys
            }
        };
        let cmd: Vec<String> = match table.get("cmd") {
            None => Vec::new(),
            Some(toml::Value::Array(items)) => items
                .iter()
                .map(|v| v.as_str().unwrap_or_default().to_string())
                .collect(),
            Some(toml::Value::String(s)) => s.chars().map(|c| c.to_string()).collect(),
            Some(_) => Vec::new(),
        };
        if cmd.is_empty() {
            return Err(LoadError::Other(format!(
                "profile {name}: 'cmd' is required"
            )));
        }
        out.push(Profile {
            name: name.clone(),
            cmd,
            cwd: table
                .get("cwd")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            keys,
            agent_ok: table.get("agent-ok").map(truthy).unwrap_or(false),
        });
    }
    Ok(out)
}

// -- clean env / scrub ------------------------------------------------------

pub fn clean_env(env_map: &[(String, String)]) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = Vec::new();
    for key in KEEP {
        if let Ok(value) = std::env::var(key) {
            env.push((key.to_string(), value));
        }
    }
    for (key, value) in env_map {
        match env.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = value.clone(),
            None => env.push((key.clone(), value.clone())),
        }
    }
    env
}

/// Replace injected values in `text`. Ignores very short values.
pub fn scrub(text: &str, values: &[&str]) -> (String, u32) {
    let mut hits = 0u32;
    let mut text = text.to_string();
    for value in values {
        if value.chars().count() >= 8 {
            let n = text.matches(value).count() as u32;
            if n > 0 {
                hits += n;
                text = text.replace(value, "[REDACTED]");
            }
        }
    }
    (text, hits)
}

// -- audit ------------------------------------------------------------------

pub enum JVal {
    Str(String),
    Num(i64),
    Arr(Vec<String>),
}

fn jstr(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

/// Python `json.dumps(event, ensure_ascii=False)`: `", "` / `": "` separators.
fn json_line(pairs: &[(&str, JVal)]) -> String {
    let mut out = String::from("{");
    for (i, (key, value)) in pairs.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&jstr(key));
        out.push_str(": ");
        match value {
            JVal::Str(s) => out.push_str(&jstr(s)),
            JVal::Num(n) => out.push_str(&n.to_string()),
            JVal::Arr(items) => {
                out.push('[');
                for (j, item) in items.iter().enumerate() {
                    if j > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&jstr(item));
                }
                out.push(']');
            }
        }
    }
    out.push('}');
    out
}

pub fn audit(path: &Path, pairs: &[(&str, JVal)]) {
    let _ = (|| -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(json_line(pairs).as_bytes())?;
        file.write_all(b"\r\n")?;
        Ok(())
    })();
}

// -- run --------------------------------------------------------------------

pub fn run_profile(
    profile: &Profile,
    env_map: &[(String, String)],
    audit_path: &Path,
) -> Result<(i32, u32), String> {
    let env = clean_env(env_map);
    let values: Vec<&str> = env_map.iter().map(|(_, v)| v.as_str()).collect();
    let started = now();

    let (reader, w1, w2) = crate::proc::merged_pipe().map_err(|e| format!("pipe: {e}"))?;
    // The Command owns the pipe's write handles; drop it after spawn or the
    // parent keeps a writer open and the read never reaches EOF.
    let mut child = {
        let mut cmd = Command::new(&profile.cmd[0]);
        cmd.args(&profile.cmd[1..]);
        if let Some(cwd) = &profile.cwd {
            cmd.current_dir(cwd);
        }
        cmd.env_clear();
        for (k, v) in &env {
            cmd.env(k, v);
        }
        cmd.stdout(Stdio::from(w1)).stderr(Stdio::from(w2));
        cmd.spawn()
            .map_err(|e| format!("{}: {e}", profile.cmd[0]))?
    };

    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    let mut hits_total = 0u32;
    loop {
        buf.clear();
        let n = reader
            .read_until(b'\n', &mut buf)
            .map_err(|e| format!("read child output: {e}"))?;
        if n == 0 {
            break;
        }
        let text = String::from_utf8_lossy(&buf);
        // Python reads text mode with universal newlines, then writes with '\n'
        // translated to os.linesep (CRLF). Normalise here, emit_out adds CRLF.
        let translated = text.replace("\r\n", "\n").replace('\r', "\n");
        for line in translated.split_inclusive('\n') {
            let (out, hits) = scrub(line, &values);
            hits_total += hits;
            crate::emit_out(&out);
        }
    }

    let status = child.wait().map_err(|e| format!("wait: {e}"))?;
    let code = status.code().unwrap_or(1);

    let mut keys: Vec<String> = profile.keys.iter().map(|(k, _)| k.clone()).collect();
    keys.sort();
    audit(
        audit_path,
        &[
            ("ts", JVal::Str(started)),
            ("cmd", JVal::Str("run".to_string())),
            ("profile", JVal::Str(profile.name.clone())),
            ("keys", JVal::Arr(keys)),
            ("scrub_hits", JVal::Num(hits_total as i64)),
            ("exit", JVal::Num(code as i64)),
            ("ppid", JVal::Num(ppid() as i64)),
        ],
    );

    Ok((code, hits_total))
}

pub fn audit_refuse(audit_path: &Path, profile: &str) {
    audit(
        audit_path,
        &[
            ("ts", JVal::Str(now())),
            ("cmd", JVal::Str("refuse".to_string())),
            ("profile", JVal::Str(profile.to_string())),
            ("reason", JVal::Str("human-only, no tty".to_string())),
            ("ppid", JVal::Num(ppid() as i64)),
        ],
    );
}

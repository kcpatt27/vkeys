//! Encrypted value store. Metadata is plaintext; values are DPAPI blobs.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::dpapi;

#[derive(Serialize, Deserialize)]
struct VaultFile {
    version: u32,
    entries: BTreeMap<String, Entry>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Entry {
    pub ct: String,
    #[serde(default)]
    pub set_at: Option<String>,
    #[serde(default)]
    pub last_used: Option<String>,
    #[serde(default)]
    pub uses: u64,
}

pub struct Vault {
    pub path: PathBuf,
    data: VaultFile,
}

impl Vault {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, String> {
        let path = path.into();
        let mut data = VaultFile {
            version: 1,
            entries: BTreeMap::new(),
        };
        if path.exists() {
            let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            data = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        Ok(Self { path, data })
    }

    /// Python's `path.with_suffix(".tmp")`: replace the last suffix, else append.
    fn tmp_path(&self) -> PathBuf {
        let name = self
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let cut = match name.rfind('.') {
            Some(0) | None => name.len(),
            Some(i) => i,
        };
        let mut tmp_name = String::new();
        tmp_name.push_str(&name[..cut]);
        tmp_name.push_str(".tmp");
        match self.path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.join(tmp_name),
            Some(_) => PathBuf::from(tmp_name),
            None => PathBuf::from(tmp_name),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
        }
        let json = serde_json::to_string_pretty(&self.data).map_err(|e| e.to_string())?;
        // Python's text-mode write translates '\n' to os.linesep (CRLF on Windows).
        let text = json.replace('\n', "\r\n");
        let tmp = self.tmp_path();
        fs::write(&tmp, text.as_bytes()).map_err(|e| format!("{}: {e}", tmp.display()))?;
        fs::rename(&tmp, &self.path).map_err(|e| format!("{}: {e}", self.path.display()))
    }

    pub fn names(&self) -> Vec<String> {
        self.data.entries.keys().cloned().collect()
    }

    pub fn meta(&self, name: &str) -> Option<&Entry> {
        self.data.entries.get(name)
    }

    pub fn set(&mut self, name: &str, value: &str) -> Result<(), String> {
        let blob = dpapi::protect(value.as_bytes())?;
        self.data.entries.insert(
            name.to_string(),
            Entry {
                ct: base64::engine::general_purpose::STANDARD.encode(blob),
                set_at: Some(crate::runtime::now()),
                last_used: None,
                uses: 0,
            },
        );
        self.save()
    }

    /// Internal only. Never expose through the CLI.
    pub fn get(&self, name: &str) -> Result<String, String> {
        let entry = self
            .data
            .entries
            .get(name)
            .ok_or_else(|| format!("no such key: {name}"))?;
        let blob = base64::engine::general_purpose::STANDARD
            .decode(entry.ct.as_bytes())
            .map_err(|e| format!("{name}: invalid ciphertext: {e}"))?;
        let plain = dpapi::unprotect(&blob)?;
        String::from_utf8(plain).map_err(|e| format!("{name}: invalid utf-8: {e}"))
    }

    pub fn touch(&mut self, name: &str) -> Result<(), String> {
        if let Some(entry) = self.data.entries.get_mut(name) {
            entry.last_used = Some(crate::runtime::now());
            entry.uses += 1;
            return self.save();
        }
        Ok(())
    }
}

pub fn default_vault() -> PathBuf {
    match std::env::var("VKEYS_VAULT") {
        Ok(v) => PathBuf::from(v),
        Err(_) => PathBuf::from(r"C:\ProgramData\secrets\vault.bin"),
    }
}

/// Parent of a relative filename, matching Python's `Path("x").parent == Path(".")`.
pub fn parent_dir(path: &Path) -> PathBuf {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// audit.jsonl sits next to the vault (Python: `Path(vault).parent / "audit.jsonl"`).
pub fn audit_path(vault: &Path) -> PathBuf {
    match vault.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.join("audit.jsonl"),
        _ => PathBuf::from("audit.jsonl"),
    }
}

//! Encrypted value store. Metadata is plaintext; values are platform-encrypted
//! blobs (DPAPI on Windows, XChaCha20-Poly1305 elsewhere).

use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::crypto;

#[derive(Serialize, Deserialize)]
struct VaultFile {
    version: u32,
    /// Non-secret fingerprint of the master key, added on the next successful
    /// write. Old vaults do not have it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    key_id: Option<String>,
    entries: IndexMap<String, Entry>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Entry {
    pub ct: String,
    /// Missing on pre-2026-10-28 entries: those are DPAPI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
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
    exists: bool,
}

impl Vault {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, String> {
        let path = path.into();
        let exists = path.exists();
        let mut data = VaultFile {
            version: 1,
            key_id: None,
            entries: IndexMap::new(),
        };
        if exists {
            let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            data = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        Ok(Self { path, data, exists })
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
        // Python: sorted(self.data["entries"]) - order on disk, order on screen
        // are different things.
        let mut names: Vec<String> = self.data.entries.keys().cloned().collect();
        names.sort();
        names
    }

    pub fn meta(&self, name: &str) -> Option<&Entry> {
        self.data.entries.get(name)
    }

    /// Verify the resolved key against the vault before any mutation.
    ///
    /// A missing key is an error when the vault exists. A wrong key is detected
    /// by the stored fingerprint, or by decrypting one existing XChaCha entry
    /// on a vault written before fingerprints were added.
    pub fn validate_key(&self) -> Result<(), String> {
        let has_xchacha = self
            .data
            .entries
            .values()
            .any(|e| e.scheme.as_deref() == Some(crypto::XCHACHA));
        if !crypto::uses_master_key() && !has_xchacha {
            return Ok(());
        }

        let key = crate::secret::master_key(self.exists)?;
        if let Some(expected) = &self.data.key_id {
            if key.id() != *expected {
                return Err(crate::secret::key_mismatch(Some(expected), &key.id()));
            }
            return Ok(());
        }

        if let Some((name, entry)) = self
            .data
            .entries
            .iter()
            .find(|(_, e)| e.scheme.as_deref() == Some(crypto::XCHACHA))
        {
            let blob = base64::engine::general_purpose::STANDARD
                .decode(entry.ct.as_bytes())
                .map_err(|e| format!("{name}: invalid ciphertext: {e}"))?;
            if crypto::open_with_key(key.bytes(), &blob).is_err() {
                return Err(crate::secret::key_mismatch(None, &key.id()));
            }
        }
        Ok(())
    }

    pub fn set(&mut self, name: &str, value: &str) -> Result<(), String> {
        self.validate_key()?;
        let (scheme, blob, key_id) = crypto::protect(value.as_bytes(), self.exists)?;
        if let Some(id) = key_id {
            if let Some(expected) = &self.data.key_id {
                if expected != &id {
                    return Err(crate::secret::key_mismatch(Some(expected), &id));
                }
            }
            self.data.key_id = Some(id);
        }
        self.data.entries.insert(
            name.to_string(),
            Entry {
                ct: base64::engine::general_purpose::STANDARD.encode(blob),
                scheme: Some(scheme.to_string()),
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
        let scheme = entry.scheme.as_deref().unwrap_or(crypto::DPAPI);
        let plain = crypto::unprotect(scheme, &blob, self.exists, self.data.key_id.as_deref())
            .map_err(|e| format!("{}: {e}", self.path.display()))?;
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

#[allow(dead_code)]
pub fn home_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Directory holding `vault.bin` and `audit.jsonl`.
pub fn data_dir() -> PathBuf {
    if let Ok(v) = std::env::var("VKEYS_DATA_DIR") {
        return PathBuf::from(v);
    }
    #[cfg(windows)]
    {
        let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".to_string());
        return PathBuf::from(base).join("secrets");
    }
    #[cfg(target_os = "macos")]
    {
        return home_dir()
            .join("Library")
            .join("Application Support")
            .join("vkeys");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            return PathBuf::from(xdg).join("vkeys");
        }
        return home_dir().join(".local").join("share").join("vkeys");
    }
    #[allow(unreachable_code)]
    PathBuf::from(".")
}

pub fn default_vault() -> PathBuf {
    match std::env::var("VKEYS_VAULT") {
        Ok(v) => PathBuf::from(v),
        Err(_) => data_dir().join("vault.bin"),
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

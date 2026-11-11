//! Master key for the XChaCha20-Poly1305 vault scheme.
//!
//! Resolution order:
//! 1. `VKEYS_MASTER_KEY_FILE` if set (explicit; the headless/service path).
//! 2. The OS credential store (Windows Credential Manager, macOS Keychain,
//!    Linux Secret Service).
//! 3. A 0600 key file in the platform config dir, when no credential store is
//!    available (headless Linux).
//!
//! A key is created only when the vault does not exist yet. If a vault exists,
//! a missing key is an error: creating a new key would make the existing vault
//! unreadable. The key never goes into the vault file.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

const SERVICE: &str = "vkeys";
const USER: &str = "master-key";

pub struct MasterKey {
    key: [u8; 32],
}

impl MasterKey {
    /// Stable, non-secret fingerprint stored in the vault to detect a key
    /// mismatch before any mutation.
    pub fn id(&self) -> String {
        let digest = Sha256::digest(self.key);
        let mut out = String::with_capacity(32);
        for byte in &digest[..16] {
            use std::fmt::Write;
            let _ = write!(out, "{byte:02x}");
        }
        out
    }

    pub fn bytes(&self) -> &[u8; 32] {
        &self.key
    }
}

fn encode(key: &[u8; 32]) -> String {
    base64::engine::general_purpose::STANDARD.encode(key)
}

fn decode(text: &str) -> Result<[u8; 32], String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(text.trim())
        .map_err(|e| format!("master key: {e}"))?;
    bytes
        .try_into()
        .map_err(|_| "master key: wrong length".to_string())
}

fn from_bytes(key: [u8; 32]) -> MasterKey {
    MasterKey { key }
}

fn keyring_get() -> Result<Option<MasterKey>, String> {
    let entry =
        keyring::Entry::new(SERVICE, USER).map_err(|e| format!("keyring unavailable: {e}"))?;
    match entry.get_password() {
        Ok(text) => Ok(Some(from_bytes(decode(&text)?))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("keyring read: {e}")),
    }
}

fn keyring_get_or_create() -> Result<MasterKey, String> {
    if let Some(key) = keyring_get()? {
        return Ok(key);
    }
    let mut key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key);
    let entry =
        keyring::Entry::new(SERVICE, USER).map_err(|e| format!("keyring unavailable: {e}"))?;
    entry
        .set_password(&encode(&key))
        .map_err(|e| format!("keyring write: {e}"))?;
    Ok(from_bytes(key))
}

/// Platform config path for the fallback key file.
pub fn default_key_file() -> PathBuf {
    #[cfg(windows)]
    {
        let base = std::env::var("APPDATA")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        return base.join("vkeys").join("master.key");
    }
    #[cfg(target_os = "macos")]
    {
        return crate::vault::home_dir()
            .join("Library")
            .join("Application Support")
            .join("vkeys")
            .join("master.key");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("vkeys").join("master.key");
        }
        return crate::vault::home_dir()
            .join(".config")
            .join("vkeys")
            .join("master.key");
    }
    #[allow(unreachable_code)]
    PathBuf::from("master.key")
}

/// Read an existing key file. Never creates one.
pub fn load_key_file(path: &Path) -> Result<MasterKey, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(from_bytes(decode(&text)?))
}

/// Read the key file, or create it atomically with 0600 permissions.
///
/// Callers must only use this when no vault exists (or when the resolved path
/// is known to hold the existing key).
pub fn load_or_create_key_file(path: &Path) -> Result<MasterKey, String> {
    if path.exists() {
        return load_key_file(path);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }

    let mut key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key);

    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
            }
            file.write_all(encode(&key).as_bytes())
                .map_err(|e| format!("{}: {e}", path.display()))?;
            Ok(from_bytes(key))
        }
        // Another process won the race: use the key it wrote.
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => load_key_file(path),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Resolve the master key.
///
/// `existing_vault` is true when a vault file already exists. In that case the
/// key must already exist; this function must never create one.
pub fn master_key(existing_vault: bool) -> Result<MasterKey, String> {
    if let Ok(path) = std::env::var("VKEYS_MASTER_KEY_FILE") {
        let path = Path::new(&path);
        if existing_vault && !path.exists() {
            return Err(format!(
                "master key file not found: {}. A vault already exists, so vkeys will not create a new key.\nSet VKEYS_MASTER_KEY_FILE to the key that encrypted the vault (expected {}).",
                path.display(),
                path.display()
            ));
        }
        return load_or_create_key_file(path).map_err(|e| {
            format!(
                "VKEYS_MASTER_KEY_FILE={}: {e}\nSet VKEYS_MASTER_KEY_FILE to the key that encrypted the vault.",
                path.display()
            )
        });
    }

    if existing_vault {
        match keyring_get() {
            Ok(Some(key)) => return Ok(key),
            Ok(None) => {}
            Err(_) => {}
        }
        let path = default_key_file();
        if path.exists() {
            return load_key_file(&path).map_err(|e| {
                format!(
                    "{e}\nSet VKEYS_MASTER_KEY_FILE to the key that encrypted the vault (expected {}).",
                    path.display()
                )
            });
        }
        return Err(format!(
            "master key not found. A vault already exists, so vkeys will not create a new key.\nSet VKEYS_MASTER_KEY_FILE to the key that encrypted the vault (checked the OS keyring and {}).",
            path.display()
        ));
    }

    match keyring_get_or_create() {
        Ok(key) => Ok(key),
        Err(_) => load_or_create_key_file(&default_key_file()),
    }
}

/// Explain a key/vault mismatch without ever printing key material. Key ids are
/// non-secret fingerprints.
pub fn key_mismatch(expected: Option<&str>, actual: &str) -> String {
    match expected {
        Some(expected) => format!(
            "vault key mismatch: vault expects key id {expected}, resolved key id {actual}.\nSet VKEYS_MASTER_KEY_FILE to the key that encrypted the vault. The vault was not changed."
        ),
        None => format!(
            "vault key mismatch: the resolved key (id {actual}) does not decrypt this vault.\nSet VKEYS_MASTER_KEY_FILE to the key that encrypted the vault. The vault was not changed."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "vkeys-key-{tag}-{}-{stamp}.key",
            std::process::id()
        ))
    }

    #[test]
    fn key_file_roundtrip() {
        let path = temp_path("roundtrip");
        let first = load_or_create_key_file(&path).unwrap();
        let second = load_or_create_key_file(&path).unwrap();
        assert_eq!(first.bytes(), second.bytes());
        assert_eq!(first.id(), second.id());
        let _ = fs::remove_file(&path);
    }

    #[cfg(unix)]
    #[test]
    fn key_file_is_0600() {
        use std::os::unix::fs::PermissionsExt;
        let path = temp_path("perms");
        load_or_create_key_file(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let _ = fs::remove_file(&path);
    }
}

//! Master key for the XChaCha20-Poly1305 vault scheme.
//!
//! Resolution order:
//! 1. `VKEYS_MASTER_KEY_FILE` if set (explicit; the headless/Pi path).
//! 2. The OS credential store (Windows Credential Manager, macOS Keychain,
//!    Linux Secret Service).
//! 3. A 0600 key file in the platform config dir, when no credential store is
//!    available (headless Linux).
//!
//! The key never goes into the vault file.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine;
use rand::RngCore;

const SERVICE: &str = "vkeys";
const USER: &str = "master-key";

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

fn keyring_key() -> Result<[u8; 32], String> {
    let entry =
        keyring::Entry::new(SERVICE, USER).map_err(|e| format!("keyring unavailable: {e}"))?;
    match entry.get_password() {
        Ok(text) => decode(&text),
        Err(keyring::Error::NoEntry) => {
            let mut key = [0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut key);
            entry
                .set_password(&encode(&key))
                .map_err(|e| format!("keyring write: {e}"))?;
            Ok(key)
        }
        Err(e) => Err(format!("keyring read: {e}")),
    }
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

/// Read the key file, or create it atomically with 0600 permissions.
pub fn load_or_create_key_file(path: &Path) -> Result<[u8; 32], String> {
    if path.exists() {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        return decode(&text);
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
            Ok(key)
        }
        // Another process won the race: use the key it wrote.
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            decode(&text)
        }
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

pub fn master_key() -> Result<[u8; 32], String> {
    if let Ok(path) = std::env::var("VKEYS_MASTER_KEY_FILE") {
        return load_or_create_key_file(Path::new(&path));
    }
    match keyring_key() {
        Ok(key) => Ok(key),
        Err(_) => load_or_create_key_file(&default_key_file()),
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
        assert_eq!(first, second);
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

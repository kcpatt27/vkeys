//! Master key for the portable (non-DPAPI) vault scheme.
//!
//! The key lives in the OS credential store: Windows Credential Manager,
//! macOS Keychain, or Linux Secret Service. It never touches the vault file.

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

/// Fetch the master key, creating and storing one on first use.
pub fn master_key() -> Result<[u8; 32], String> {
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

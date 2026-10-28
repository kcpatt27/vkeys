//! Value encryption.
//!
//! Windows keeps the original DPAPI scheme (`dpapi`) for compatibility. Other
//! platforms (and any future portable vault) use XChaCha20-Poly1305 with the
//! master key held in the OS credential store (`xchacha20poly1305`).

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::RngCore;

pub const DPAPI: &str = "dpapi";
pub const XCHACHA: &str = "xchacha20poly1305";

pub fn protect(plain: &[u8]) -> Result<(&'static str, Vec<u8>), String> {
    #[cfg(windows)]
    {
        return Ok((DPAPI, crate::dpapi::protect(plain)?));
    }
    #[cfg(not(windows))]
    {
        let key = crate::secret::master_key()?;
        Ok((XCHACHA, seal_with_key(&key, plain)?))
    }
}

pub fn unprotect(scheme: &str, blob: &[u8]) -> Result<Vec<u8>, String> {
    match scheme {
        DPAPI => {
            #[cfg(windows)]
            {
                crate::dpapi::unprotect(blob)
            }
            #[cfg(not(windows))]
            {
                Err("this entry uses Windows DPAPI; open it on Windows to migrate".to_string())
            }
        }
        XCHACHA => {
            let key = crate::secret::master_key()?;
            open_with_key(&key, blob)
        }
        other => Err(format!("unknown vault scheme: {other}")),
    }
}

#[allow(dead_code)] // used by the non-Windows path and by tests
pub fn seal_with_key(key: &[u8; 32], plain: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let mut nonce = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&nonce), plain)
        .map_err(|e| format!("encrypt: {e}"))?;
    let mut out = Vec::with_capacity(24 + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

pub fn open_with_key(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>, String> {
    if blob.len() < 24 {
        return Err("ciphertext too short".to_string());
    }
    let (nonce, ciphertext) = blob.split_at(24);
    let cipher = XChaCha20Poly1305::new(key.into());
    cipher
        .decrypt(XNonce::from_slice(nonce), ciphertext)
        .map_err(|_| "decryption failed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aead_roundtrip() {
        let key = [7u8; 32];
        let blob = seal_with_key(&key, b"hello").unwrap();
        assert_eq!(open_with_key(&key, &blob).unwrap(), b"hello");
    }

    #[test]
    fn aead_rejects_tamper() {
        let key = [7u8; 32];
        let mut blob = seal_with_key(&key, b"hello").unwrap();
        let last = blob.len() - 1;
        blob[last] ^= 1;
        assert!(open_with_key(&key, &blob).is_err());
    }
}

//! Windows DPAPI (CurrentUser) protect/unprotect. Windows-only platform edge.

use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB,
};

const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    }
}

fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
    let bytes = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
    unsafe {
        let _ = LocalFree(Some(HLOCAL(out.pbData as *mut core::ffi::c_void)));
    }
    bytes
}

pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
    let src = blob(data);
    let mut out = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(
            &src,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out,
        )
        .map_err(|e| format!("CryptProtectData: {e}"))?;
    }
    Ok(take(out))
}

pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    let src = blob(data);
    let mut out = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(
            &src,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out,
        )
        .map_err(|e| format!("CryptUnprotectData: {e}"))?;
    }
    Ok(take(out))
}

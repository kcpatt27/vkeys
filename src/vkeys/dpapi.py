"""Windows DPAPI (CurrentUser) protect/unprotect. Windows-only."""

from __future__ import annotations

import ctypes
from ctypes import wintypes

CRYPTPROTECT_UI_FORBIDDEN = 0x1

_crypt32 = ctypes.windll.crypt32
_kernel32 = ctypes.windll.kernel32


class _BLOB(ctypes.Structure):
    _fields_ = [("cbData", wintypes.DWORD), ("pbData", ctypes.POINTER(ctypes.c_char))]


def _make(data: bytes):
    buf = ctypes.create_string_buffer(data, max(len(data), 1))
    return _BLOB(len(data), ctypes.cast(buf, ctypes.POINTER(ctypes.c_char))), buf


def protect(data: bytes) -> bytes:
    src, _keep = _make(data)
    out = _BLOB()
    ok = _crypt32.CryptProtectData(
        ctypes.byref(src), None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, ctypes.byref(out)
    )
    if not ok:
        raise ctypes.WinError()
    try:
        return ctypes.string_at(out.pbData, out.cbData)
    finally:
        _kernel32.LocalFree(out.pbData)


def unprotect(blob: bytes) -> bytes:
    src, _keep = _make(blob)
    out = _BLOB()
    ok = _crypt32.CryptUnprotectData(
        ctypes.byref(src), None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, ctypes.byref(out)
    )
    if not ok:
        raise ctypes.WinError()
    try:
        return ctypes.string_at(out.pbData, out.cbData)
    finally:
        _kernel32.LocalFree(out.pbData)

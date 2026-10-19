"""Encrypted value store. Metadata is plaintext; values are DPAPI blobs."""

from __future__ import annotations

import base64
import json
import os
import time
from pathlib import Path

from . import dpapi

DEFAULT_VAULT = Path(os.environ.get("VKEYS_VAULT", r"C:\ProgramData\secrets\vault.bin"))


def _now() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%S")


class Vault:
    def __init__(self, path: Path | str = DEFAULT_VAULT):
        self.path = Path(path)
        self.data = {"version": 1, "entries": {}}
        if self.path.exists():
            self.data = json.loads(self.path.read_text(encoding="utf-8"))

    # -- internals -------------------------------------------------------
    def _save(self) -> None:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        tmp = self.path.with_suffix(".tmp")
        tmp.write_text(json.dumps(self.data, indent=2), encoding="utf-8")
        os.replace(tmp, self.path)

    # -- public ----------------------------------------------------------
    def names(self) -> list[str]:
        return sorted(self.data["entries"])

    def meta(self, name: str) -> dict:
        return dict(self.data["entries"].get(name, {}))

    def set(self, name: str, value: str) -> None:
        self.data["entries"][name] = {
            "ct": base64.b64encode(dpapi.protect(value.encode("utf-8"))).decode("ascii"),
            "set_at": _now(),
            "last_used": None,
            "uses": 0,
        }
        self._save()

    def get(self, name: str) -> str:
        """Internal only. Never expose through the CLI."""
        entry = self.data["entries"][name]
        return dpapi.unprotect(base64.b64decode(entry["ct"])).decode("utf-8")

    def touch(self, name: str) -> None:
        entry = self.data["entries"].get(name)
        if entry is not None:
            entry["last_used"] = _now()
            entry["uses"] = int(entry.get("uses", 0)) + 1
            self._save()

    def remove(self, name: str) -> bool:
        if name not in self.data["entries"]:
            return False
        del self.data["entries"][name]
        self._save()
        return True

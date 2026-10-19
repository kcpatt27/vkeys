"""Profiles, clean-env injection, output scrubbing, and auditing."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

# Base env a child legitimately needs; everything else is dropped.
KEEP = (
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
)

DEFAULT_PROFILES = Path(
    os.environ.get(
        "VKEYS_PROFILES",
        Path(os.environ.get("APPDATA", str(Path.home()))) / "vkeys" / "profiles.toml",
    )
)


def load_profiles(path: Path | str = DEFAULT_PROFILES) -> dict[str, dict]:
    import tomllib

    path = Path(path)
    with open(path, "rb") as f:
        raw = tomllib.load(f)
    out: dict[str, dict] = {}
    for name, prof in (raw.get("profile") or {}).items():
        keys = prof.get("keys") or {}
        if not isinstance(keys, dict):
            raise ValueError(f"profile {name}: 'keys' must be a table of ENV = store-name")
        cmd = list(prof.get("cmd") or [])
        if not cmd:
            raise ValueError(f"profile {name}: 'cmd' is required")
        out[name] = {
            "cmd": cmd,
            "cwd": prof.get("cwd"),
            "keys": dict(keys),
            "agent_ok": bool(prof.get("agent-ok", False)),
        }
    return out


def clean_env(env_map: dict[str, str]) -> dict[str, str]:
    env = {k: os.environ[k] for k in KEEP if k in os.environ}
    env.update(env_map)
    return env


def scrub(text: str, values) -> tuple[str, int]:
    """Replace injected values in `text`. Ignores very short values."""
    hits = 0
    for value in values:
        if value and len(value) >= 8 and value in text:
            hits += text.count(value)
            text = text.replace(value, "[REDACTED]")
    return text, hits


def audit(event: dict, path: Path | str) -> None:
    try:
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "a", encoding="utf-8") as f:
            f.write(json.dumps(event, ensure_ascii=False) + "\n")
    except OSError:
        pass


def run_profile(name: str, prof: dict, env_map: dict[str, str], audit_path: Path | str) -> tuple[int, int]:
    """Run a profile's fixed command. Streams output through the scrubber."""
    env = clean_env(env_map)
    values = list(env_map.values())
    started = time.strftime("%Y-%m-%dT%H:%M:%S")
    hits_total = 0
    proc = subprocess.Popen(
        prof["cmd"],
        cwd=prof.get("cwd"),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
        bufsize=1,
    )
    assert proc.stdout is not None
    for line in proc.stdout:
        out, hits = scrub(line, values)
        hits_total += hits
        sys.stdout.write(out)
        sys.stdout.flush()
    code = proc.wait()
    audit(
        {
            "ts": started,
            "cmd": "run",
            "profile": name,
            "keys": sorted(prof["keys"]),
            "scrub_hits": hits_total,
            "exit": code,
            "ppid": os.getppid(),
        },
        audit_path,
    )
    return code, hits_total

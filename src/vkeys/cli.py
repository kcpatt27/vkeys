"""vkeys CLI. No command prints a stored value."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from . import __version__
from .runtime import DEFAULT_PROFILES, audit, load_profiles, run_profile, scrub
from .vault import DEFAULT_VAULT, Vault


def _audit_path(args) -> Path:
    return Path(args.vault).parent / "audit.jsonl"


def _read_secret(clipboard: bool) -> str:
    if clipboard:
        try:
            import tkinter

            root = tkinter.Tk()
            root.withdraw()
            try:
                return str(root.clipboard_get()).strip()
            finally:
                root.destroy()
        except Exception:
            print("clipboard unavailable; falling back to hidden prompt", file=sys.stderr)
    import getpass

    return getpass.getpass("value (hidden): ").strip()


def _confirm_overwrite(name: str, yes: bool) -> bool:
    if yes:
        return True
    if not sys.stdin.isatty():
        print(f"{name} exists; refusing to overwrite non-interactively (use --yes)", file=sys.stderr)
        return False
    return input(f"{name} exists; overwrite (old value is unrecoverable)? [y/N] ").strip().lower() == "y"


def cmd_init(args) -> int:
    vault_path = Path(args.vault)
    directory = vault_path.parent
    created = not directory.exists()
    directory.mkdir(parents=True, exist_ok=True)
    if created:
        user = os.environ.get("USERNAME", "")
        subprocess.run(
            [
                "icacls",
                str(directory),
                "/inheritance:r",
                "/grant:r",
                f"{user}:(OI)(CI)F",
                "/grant:r",
                "*S-1-5-32-544:(OI)(CI)F",
            ],
            capture_output=True,
            text=True,
        )
    vault = Vault(vault_path)
    if not vault_path.exists():
        vault._save()
    print(f"vault: {vault_path}")
    print(f"profiles: {args.profiles}  (copy profiles.example.toml there)")
    return 0


def cmd_set(args) -> int:
    vault = Vault(args.vault)
    if args.name in vault.names() and not _confirm_overwrite(args.name, args.yes):
        return 1
    value = _read_secret(args.clipboard)
    if not value:
        print("empty value; aborted", file=sys.stderr)
        return 1
    vault.set(args.name, value)
    print(f"stored {args.name}")
    return 0


def cmd_ls(args) -> int:
    vault = Vault(args.vault)
    names = vault.names()
    if not names:
        print(f"no keys in {vault.path}")
        return 0
    print(f"{'NAME':<32} {'SET':<20} {'LAST USED':<20} {'USES':>4}")
    for name in names:
        meta = vault.meta(name)
        print(
            f"{name:<32} {str(meta.get('set_at') or '-'):<20} "
            f"{str(meta.get('last_used') or '-'):<20} {int(meta.get('uses', 0)):>4}"
        )
    return 0


def _resolve(args, profile_name: str):
    try:
        profiles = load_profiles(args.profiles)
    except FileNotFoundError:
        print(f"no profiles file at {args.profiles}; copy profiles.example.toml there", file=sys.stderr)
        return None
    if profile_name not in profiles:
        print(
            f"unknown profile {profile_name!r}; have: {', '.join(sorted(profiles)) or '(none)'}",
            file=sys.stderr,
        )
        return None
    prof = profiles[profile_name]
    vault = Vault(args.vault)
    missing = [store for store in prof["keys"].values() if store not in vault.names()]
    if missing:
        print(f"missing keys: {', '.join(missing)} (run: vkeys set <name>)", file=sys.stderr)
        return None
    env_map = {env_name: vault.get(store) for env_name, store in prof["keys"].items()}
    return prof, vault, env_map


def cmd_env(args) -> int:
    resolved = _resolve(args, args.profile)
    if resolved is None:
        return 1
    prof, vault, _ = resolved
    for env_name, store in prof["keys"].items():
        print(f"{env_name}=<stored: {store}>")
        vault.touch(store)
    return 0


def cmd_run(args) -> int:
    resolved = _resolve(args, args.profile)
    if resolved is None:
        return 1
    prof, vault, env_map = resolved
    if not prof["agent_ok"]:
        if not sys.stdin.isatty():
            audit(
                {
                    "ts": time.strftime("%Y-%m-%dT%H:%M:%S"),
                    "cmd": "refuse",
                    "profile": args.profile,
                    "reason": "human-only, no tty",
                    "ppid": os.getppid(),
                },
                _audit_path(args),
            )
            print(f"refusing: profile {args.profile!r} is human-only (agent-ok = false)", file=sys.stderr)
            return 2
        try:
            typed = input(f"human-only profile {args.profile!r}; type the name to run: ")
        except (EOFError, KeyboardInterrupt):
            typed = ""
        if typed.strip() != args.profile:
            audit(
                {
                    "ts": time.strftime("%Y-%m-%dT%H:%M:%S"),
                    "cmd": "refuse",
                    "profile": args.profile,
                    "reason": "human-only, confirmation failed",
                    "ppid": os.getppid(),
                },
                _audit_path(args),
            )
            print("aborted (confirmation required for a human-only profile)", file=sys.stderr)
            return 2
    for store in prof["keys"].values():
        vault.touch(store)
    code, hits = run_profile(args.profile, prof, env_map, _audit_path(args))
    if hits:
        print(f"vkeys: redacted {hits} secret occurrence(s) from output", file=sys.stderr)
    return code


def cmd_rotate(args) -> int:
    vault = Vault(args.vault)
    if args.name in vault.names():
        print(f"rotating {args.name}; the old value is unrecoverable")
    value = _read_secret(args.clipboard)
    if not value:
        print("empty value; aborted", file=sys.stderr)
        return 1
    vault.set(args.name, value)
    print(f"rotated {args.name}")
    return 0


def _parse_env(path: Path) -> dict[str, str]:
    out = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        out[key.strip()] = value.strip().strip('"').strip("'")
    return out


def cmd_import_env(args) -> int:
    parsed = _parse_env(Path(args.path))
    wanted = [n.strip() for n in args.names.split(",")] if args.names else list(parsed)
    vault = Vault(args.vault)
    for name in wanted:
        if name not in parsed:
            print(f"skip {name}: not in {args.path}", file=sys.stderr)
            continue
        vault.set(name, parsed[name])
        print(f"imported {name}")
    return 0


def cmd_audit(args) -> int:
    path = _audit_path(args)
    if not path.exists():
        print("no audit log yet")
        return 0
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    for line in lines[-args.tail :]:
        print(line)
    return 0


def cmd_selfcheck(args) -> int:
    with tempfile.TemporaryDirectory() as td:
        vault_path = Path(td) / "vault.bin"
        vault = Vault(vault_path)
        secret = "s3cret-value-123"
        vault.set("k", secret)
        assert vault.get("k") == secret
        assert secret.encode() not in vault_path.read_bytes(), "plaintext found in vault"
        assert vault.names() == ["k"]
        out, hits = scrub(f"a {secret} b", [secret])
        assert hits == 1 and secret not in out and "[REDACTED]" in out
        prof_path = Path(td) / "profiles.toml"
        prof_path.write_text(
            '[profile.x]\ncmd = ["cmd"]\nagent-ok = true\n[profile.x.keys]\nK = "k"\n',
            encoding="utf-8",
        )
        prof = load_profiles(prof_path)["x"]
        assert prof["keys"] == {"K": "k"} and prof["agent_ok"] is True
    print("selfcheck ok: dpapi roundtrip, no plaintext, scrub, profile parse")
    return 0


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="vkeys", description="personal virtual-key store")
    p.add_argument("--version", action="version", version=f"vkeys {__version__}")
    p.add_argument("--vault", default=str(DEFAULT_VAULT), help="vault path (default: %(default)s)")
    p.add_argument("--profiles", default=str(DEFAULT_PROFILES), help="profiles TOML (default: %(default)s)")
    sub = p.add_subparsers(dest="command", required=True)

    s = sub.add_parser("init", help="create the vault + ACL (once)")
    s.set_defaults(func=cmd_init)

    s = sub.add_parser("set", help="store a value (hidden paste)")
    s.add_argument("name")
    s.add_argument("--clipboard", action="store_true", help="read from the Windows clipboard")
    s.add_argument("--yes", action="store_true", help="overwrite without asking")
    s.set_defaults(func=cmd_set)

    s = sub.add_parser("ls", help="list names (never values)")
    s.set_defaults(func=cmd_ls)

    s = sub.add_parser("env", help="print redacted references for a profile")
    s.add_argument("profile")
    s.set_defaults(func=cmd_env)

    s = sub.add_parser("run", help="run a profile's fixed command")
    s.add_argument("profile")
    s.set_defaults(func=cmd_run)

    s = sub.add_parser("rotate", help="replace a value (old is unrecoverable)")
    s.add_argument("name")
    s.add_argument("--clipboard", action="store_true")
    s.set_defaults(func=cmd_rotate)

    s = sub.add_parser("import-env", help="import names from a .env without printing")
    s.add_argument("path")
    s.add_argument("--names", default="", help="comma-separated; default: all")
    s.set_defaults(func=cmd_import_env)

    s = sub.add_parser("audit", help="show recent audit events")
    s.add_argument("--tail", type=int, default=20)
    s.set_defaults(func=cmd_audit)

    s = sub.add_parser("selfcheck", help="runnable check")
    s.set_defaults(func=cmd_selfcheck)
    return p


def main(argv=None) -> int:
    args = build_parser().parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())

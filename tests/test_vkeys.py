import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from vkeys.cli import main  # noqa: E402
from vkeys.runtime import clean_env, load_profiles, scrub  # noqa: E402
from vkeys.vault import Vault  # noqa: E402


@unittest.skipUnless(os.name == "nt", "DPAPI is Windows-only")
class VaultTests(unittest.TestCase):
    def test_roundtrip_and_no_plaintext(self):
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "vault.bin"
            vault = Vault(path)
            vault.set("k", "s3cret-value-123")
            self.assertEqual(vault.get("k"), "s3cret-value-123")
            self.assertNotIn(b"s3cret-value-123", path.read_bytes())
            self.assertEqual(vault.names(), ["k"])

    def test_touch_and_remove(self):
        with tempfile.TemporaryDirectory() as td:
            vault = Vault(Path(td) / "vault.bin")
            vault.set("k", "s3cret-value-123")
            vault.touch("k")
            self.assertEqual(vault.meta("k")["uses"], 1)
            self.assertTrue(vault.remove("k"))
            self.assertEqual(vault.names(), [])

    def test_human_only_confirmation_failure_is_audited(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            vault = Vault(root / "vault.bin")
            vault.set("k", "s3cret-value-123")
            profiles = root / "profiles.toml"
            profiles.write_text(
                '[profile.human]\ncmd = ["cmd"]\nagent-ok = false\n'
                '[profile.human.keys]\nK = "k"\n',
                encoding="utf-8",
            )

            with patch("sys.stdin.isatty", return_value=True), patch(
                "builtins.input", return_value="wrong"
            ):
                code = main(
                    [
                        "--vault",
                        str(root / "vault.bin"),
                        "--profiles",
                        str(profiles),
                        "run",
                        "human",
                    ]
                )

            self.assertEqual(code, 2)
            events = [
                json.loads(line)
                for line in (root / "audit.jsonl").read_text(encoding="utf-8").splitlines()
                if line.strip()
            ]
            self.assertEqual(events[-1]["cmd"], "refuse")
            self.assertEqual(events[-1]["profile"], "human")
            self.assertEqual(events[-1]["reason"], "human-only, confirmation failed")


class RuntimeTests(unittest.TestCase):
    def test_scrub_replaces_and_counts(self):
        out, hits = scrub("x abcdefgh123 y abcdefgh123", ["abcdefgh123"])
        self.assertEqual(hits, 2)
        self.assertNotIn("abcdefgh123", out)
        self.assertIn("[REDACTED]", out)

    def test_scrub_ignores_short_values(self):
        out, hits = scrub("a b c", ["a"])
        self.assertEqual((out, hits), ("a b c", 0))

    def test_clean_env_drops_inherited_secrets(self):
        os.environ["VKEYS_TEST_SECRET"] = "should-not-pass"
        try:
            env = clean_env({"ALLOWED_KEY": "value"})
            self.assertEqual(env["ALLOWED_KEY"], "value")
            self.assertNotIn("VKEYS_TEST_SECRET", env)
        finally:
            del os.environ["VKEYS_TEST_SECRET"]

    def test_profile_parse(self):
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "profiles.toml"
            path.write_text(
                '[profile.x]\ncmd = ["cmd"]\nagent-ok = true\n[profile.x.keys]\nK = "k"\n',
                encoding="utf-8",
            )
            prof = load_profiles(path)["x"]
            self.assertEqual(prof["keys"], {"K": "k"})
            self.assertTrue(prof["agent_ok"])

    def test_profile_requires_cmd(self):
        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "profiles.toml"
            path.write_text('[profile.x]\nagent-ok = true\n', encoding="utf-8")
            with self.assertRaises(ValueError):
                load_profiles(path)


if __name__ == "__main__":
    unittest.main()

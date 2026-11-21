"""Stable release notices use official metadata and never control test gates."""
import contextlib
import io
from http.client import IncompleteRead
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch
from urllib.error import URLError

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import verify


class StableUpdateTests(unittest.TestCase):
    def setUp(self):
        self.compiler = self.enterContext(patch.object(verify.subprocess, "check_output", side_effect=[
            "rustc 1.102.0-nightly (fixture 2026-11-13)\n",
            "rustc 1.99.0 (fixture 2026-10-01)\n",
        ]))
        self.network = self.enterContext(patch.object(verify.urllib.request, "urlopen"))
        self.network.return_value.__enter__.return_value.read.return_value = (
            b'[pkg.rust]\nversion = "1.100.0 (fixture 2026-11-09)"\n')

    def test_update_compares_stable_numerically_and_shows_active_nightly(self):
        notice = verify.stable_update_notice(Path.cwd())
        self.assertIn("Active verification compiler: rustc 1.102.0-nightly", notice)
        self.assertIn("Installed Stable compiler: rustc 1.99.0", notice)
        self.assertIn("Stable update available: 1.99.0 -> 1.100.0. Run: rustup update stable", notice)
        self.network.assert_called_once_with(verify.STABLE_MANIFEST, timeout=5)
        self.assertEqual(self.compiler.call_args_list[1].args[0],
                         ["rustup", "run", "stable", "rustc", "--version"])

    def test_current_and_newer_local_stable_need_no_update(self):
        for installed in ("1.100.0", "1.101.0"):
            with self.subTest(installed=installed):
                self.compiler.side_effect = ["rustc 1.102.0-nightly", f"rustc {installed}"]
                notice = verify.stable_update_notice(Path.cwd())
                self.assertIn("Stable is up to date", notice)
                self.assertNotIn("rustup update stable", notice)

    def test_invalid_metadata_is_informational(self):
        for response in (b"not valid TOML", b"", b"[pkg]\nrust = []",
                         b'[pkg.rust]\nversion = "1.102.0-nightly"',
                         b'[pkg.rust]\nversion = 100', bytes([255])):
            with self.subTest(response=response):
                self.compiler.side_effect = ["rustc 1.100.0", "rustc 1.100.0"]
                self.network.return_value.__enter__.return_value.read.return_value = response
                self.assertIn("Could not check Stable updates", verify.stable_update_notice(Path.cwd()))

    def test_network_failure_is_informational(self):
        for error in (TimeoutError("timed out"), URLError("offline"), IncompleteRead(b"partial")):
            with self.subTest(error=error):
                self.compiler.side_effect = ["rustc 1.100.0", "rustc 1.100.0"]
                self.network.side_effect = error
                notice = verify.stable_update_notice(Path.cwd())
                self.assertIn("Could not check Stable updates", notice)
                self.assertIn("Active verification compiler: rustc 1.100.0", notice)

    def test_missing_stable_is_informational(self):
        self.compiler.side_effect = ["rustc 1.102.0-nightly", subprocess.CalledProcessError(1, "rustup")]
        self.assertIn("Could not check Stable updates", verify.stable_update_notice(Path.cwd()))

    def test_active_compiler_failure_still_checks_stable(self):
        self.compiler.side_effect = [OSError("compiler unavailable"), "rustc 1.100.0"]
        notice = verify.stable_update_notice(Path.cwd())
        self.assertIn("Active verification compiler unavailable", notice)
        self.assertIn("Stable is up to date", notice)
        self.network.assert_called_once()


    def test_commit_plan_performs_no_network_or_compiler_query(self):
        with patch.object(verify, "collect_changes", return_value=set()), \
             patch.object(verify, "select_phases", return_value=[]), \
             contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(verify.main(["commit", "--plan"]), 0)
        self.network.assert_not_called()
        self.compiler.assert_not_called()


if __name__ == "__main__":
    unittest.main()

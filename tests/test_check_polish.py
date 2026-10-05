"""Verify explicit language scanning without executable hook entry points."""

import importlib.util
import io
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "tools/harness/check_polish.py"
SPEC = importlib.util.spec_from_file_location("check_polish", SCRIPT)
SCANNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SCANNER)


class TestCheckPolish(unittest.TestCase):
    def invoke(self, *args):
        with patch.object(sys, "argv", [str(SCRIPT), *args]), patch("sys.stdout", new_callable=io.StringIO):
            return SCANNER.main()

    def test_file_scan_reports_polish_and_accepts_english(self):
        with tempfile.TemporaryDirectory() as temp:
            target = Path(temp) / "note.md"
            target.write_text("The processor reads the next instruction.\n", encoding="utf-8")
            self.assertEqual(self.invoke(str(target)), 0)
            target.write_text("Poniewaz procesor teraz wykonuje instrukcje.\n", encoding="utf-8")
            self.assertEqual(self.invoke(str(target)), 1)

    def test_staged_scan_checks_added_lines_only(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)

            def git(*args):
                return subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True)

            git("init", "--quiet")
            git("config", "user.email", "fixture@example.invalid")
            git("config", "user.name", "Scanner Fixture")
            # Prevent inherited hooks from running inside this isolated fixture.
            hooks = root / "disabled-hooks"
            hooks.mkdir()
            git("config", "core.hooksPath", str(hooks))
            target = root / "note.md"
            target.write_text("Poniewaz procesor teraz wykonuje instrukcje.\n", encoding="utf-8")
            git("add", "note.md")
            git("commit", "--quiet", "-m", "fixture")
            target.write_text("The processor reads the next instruction.\n", encoding="utf-8")
            git("add", "note.md")
            with patch.object(SCANNER, "REPO_ROOT", root):
                self.assertEqual(self.invoke("--staged"), 0)
                target.write_text("Poniewaz procesor teraz wykonuje instrukcje.\n" * 2, encoding="utf-8")
                git("add", "note.md")
                self.assertEqual(self.invoke("--staged"), 1)

    def test_hook_and_unknown_options_are_rejected(self):
        for option in ("--hook", "--git", "--unknown"):
            with self.subTest(option=option), patch("sys.stderr", new_callable=io.StringIO):
                with self.assertRaises(SystemExit) as raised:
                    self.invoke(option)
                self.assertEqual(raised.exception.code, 2)


if __name__ == "__main__":
    unittest.main()

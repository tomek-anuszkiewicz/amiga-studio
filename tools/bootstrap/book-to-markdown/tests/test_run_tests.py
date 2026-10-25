"""Dependency setup must make the review tests runnable on a clean checkout."""

import importlib.util
import io
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from unittest.mock import patch


RUNNER = Path(__file__).resolve().parents[1] / "run_tests.py"
SPEC = importlib.util.spec_from_file_location("book_review_test_runner", RUNNER)
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class DependencySetupTests(unittest.TestCase):
    def test_missing_python_package_installs_pinned_requirements(self):
        with patch.object(runner.importlib.metadata, "version", side_effect=runner.importlib.metadata.PackageNotFoundError), \
             patch.object(runner.subprocess, "run") as execute:
            with redirect_stdout(io.StringIO()):
                runner.ensure_python_packages()
        execute.assert_called_once_with(
            [sys.executable, "-m", "pip", "install", "-r", str(runner.HERE / "requirements.txt")],
            cwd=runner.REPO_ROOT, check=True)

    def test_missing_katex_uses_locked_npm_install_without_scripts(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "package.json").write_text('{"dependencies":{"katex":"0.16.22"}}', encoding="utf-8")

            def install(command, **kwargs):
                asset = root / "node_modules/katex/dist/katex.min.js"
                asset.parent.mkdir(parents=True)
                asset.write_text("installed", encoding="utf-8")
                (root / "node_modules/katex/package.json").write_text('{"version":"0.16.22"}',
                                                                         encoding="utf-8")
                self.assertEqual(command[1:3], ["ci", "--prefix"])
                self.assertIn("--ignore-scripts", command)
                self.assertEqual(kwargs["cwd"], runner.REPO_ROOT)

            with patch.object(runner, "HERE", root), \
                 patch.object(runner.shutil, "which", return_value="npm.cmd"), \
                 patch.object(runner.subprocess, "run", side_effect=install) as execute:
                with redirect_stdout(io.StringIO()):
                    runner.ensure_katex()
                    runner.ensure_katex()
            execute.assert_called_once()

    def test_install_failure_stops_before_test_discovery(self):
        with patch.object(runner, "ensure_python_packages", side_effect=subprocess.CalledProcessError(1, "pip")), \
             patch.object(runner, "ensure_katex") as katex, \
             patch.object(runner.subprocess, "run") as execute, \
             patch.object(sys, "argv", [str(RUNNER)]):
            with redirect_stderr(io.StringIO()):
                self.assertEqual(runner.main(), 1)
        katex.assert_not_called()
        execute.assert_not_called()


if __name__ == "__main__":
    unittest.main()

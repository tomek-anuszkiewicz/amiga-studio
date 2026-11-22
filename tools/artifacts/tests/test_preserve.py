"""Preserve real ignored task outputs without deleting or overwriting work."""
from contextlib import redirect_stderr, redirect_stdout
import errno
import importlib.util
from io import StringIO
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1] / "preserve.py"
spec = importlib.util.spec_from_file_location("artifact_preservation", SCRIPT)
preservation = importlib.util.module_from_spec(spec)
spec.loader.exec_module(preservation)


class PreserveTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.primary = self.root / "primary"
        self.primary.mkdir()
        self.git(self.primary, "init", "-q")
        (self.primary / ".gitignore").write_text(".agent/\nbenchmarks/results/\ntarget/\n", encoding="utf-8")
        self.git(self.primary, "add", ".gitignore")
        self.git(self.primary, "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                 "-c", "commit.gpgsign=false", "commit", "-qm", "Fixture")
        self.worktree = self.root / "task"
        self.git(self.primary, "worktree", "add", "-qb", "task", str(self.worktree))
        self.destination = self.primary / ".agent/task-archives/task/outputs"

    def git(self, root, *arguments):
        return subprocess.run(["git", *arguments], cwd=root, check=True, capture_output=True).stdout

    def output(self, relative, content=b"actual output"):
        path = self.worktree / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        return path

    def test_defaults_keep_reports_and_skip_build_assets_and_tracked_files(self):
        paths = (".agent/tmp/verification/diagnostics/run/test-runtime-report.md",
                 ".agent/tmp/verify/command.log", ".agent/tmp/singlestep/summary.json",
                 ".agent/tmp/coverage/gaps.md", "benchmarks/results/run/result.csv")
        for path in paths:
            self.output(path)
        self.output("target/debug/application.exe")
        self.output(".agent/tmp/other/rom.bin")
        self.output(".agent/tmp/verification/tracked.txt")
        self.git(self.worktree, "add", "-f", ".agent/tmp/verification/tracked.txt")
        result = preservation.preserve(self.worktree, self.destination)
        self.assertEqual({item["source"] for item in result["files"]}, set(paths))
        for path in paths:
            self.assertEqual((self.destination / path).read_bytes(), (self.worktree / path).read_bytes())
        self.assertTrue((self.worktree / "target/debug/application.exe").exists())

    def test_custom_outputs_retry_and_collision_keep_both_versions(self):
        source = self.output(".agent/screenshots/frame.png", b"first")
        include = [".agent/screenshots"]
        first = preservation.preserve(self.worktree, self.destination, include)
        self.assertEqual(first["copied"], 1)
        second = preservation.preserve(self.worktree, self.destination, include)
        self.assertEqual((second["copied"], second["reused"]), (0, 1))
        source.write_bytes(b"second")
        third = preservation.preserve(self.worktree, self.destination, include)
        self.assertNotEqual(first["files"][0]["archive"], third["files"][0]["archive"])
        self.assertEqual((self.primary / first["files"][0]["archive"]).read_bytes(), b"first")
        self.assertEqual((self.primary / third["files"][0]["archive"]).read_bytes(), b"second")

    def test_invalid_locations_missing_selection_and_tracked_input_are_rejected(self):
        for destination in (self.worktree / ".agent/archive", self.root / "outside", self.primary / "public"):
            with self.subTest(destination=destination), self.assertRaises(ValueError):
                preservation.preserve(self.worktree, destination)
        for include in ("../primary/.gitignore", ".agent/missing", ".gitignore"):
            with self.subTest(include=include), self.assertRaises(ValueError):
                preservation.preserve(self.worktree, self.destination, [include])
        with self.assertRaises(ValueError):
            preservation.preserve(self.primary, self.destination)

    def test_unreadable_nested_directory_fails_preservation(self):
        include = ".agent/screenshots"
        readable = self.output(f"{include}/overview.png", b"overview")
        hidden = self.output(f"{include}/nested/detail.png", b"detail")
        directory = hidden.parent.resolve()
        native = os.scandir

        def scan(path):
            if Path(path) == directory:
                raise PermissionError(errno.EACCES, "Access denied", path)
            return native(path)

        arguments = [str(SCRIPT), "--worktree", str(self.worktree), "--destination",
                     str(self.destination), "--include", include]
        stdout, stderr = StringIO(), StringIO()
        with (patch("os.scandir", side_effect=scan), patch.object(sys, "argv", arguments),
              redirect_stdout(stdout), redirect_stderr(stderr)):
            try:
                status = preservation.main()
            except SystemExit as error:
                status = error.code

        self.assertEqual(status, 1, stdout.getvalue())
        self.assertEqual(stdout.getvalue(), "")
        self.assertIn("Preservation failed; retain the worktree", stderr.getvalue())
        self.assertIn(str(directory), stderr.getvalue())
        self.assertEqual(readable.read_bytes(), b"overview")
        self.assertEqual(hidden.read_bytes(), b"detail")

        result = preservation.preserve(self.worktree, self.destination, [include])
        expected = {f"{include}/overview.png": b"overview", f"{include}/nested/detail.png": b"detail"}
        self.assertEqual({item["source"] for item in result["files"]}, set(expected))
        for path, content in expected.items():
            self.assertEqual((self.destination / path).read_bytes(), content)

    def test_copy_failure_keeps_source_and_already_preserved_results(self):
        self.output(".agent/tmp/coverage/a.json")
        self.output(".agent/tmp/coverage/b.json")
        native = preservation.copy_whole

        def copy(primary, source, target):
            if source.name == "b.json":
                raise OSError("disk unavailable")
            return native(primary, source, target)

        with patch.object(preservation, "copy_whole", side_effect=copy), self.assertRaises(OSError):
            preservation.preserve(self.worktree, self.destination)
        self.assertTrue((self.destination / ".agent/tmp/coverage/a.json").exists())
        self.assertTrue((self.worktree / ".agent/tmp/coverage/b.json").exists())
        result = preservation.preserve(self.worktree, self.destination)
        self.assertEqual((result["copied"], result["reused"]), (1, 1))

    def test_copy_permission_error_identifies_source_when_error_has_no_filename(self):
        source = self.output(".agent/tmp/coverage/report.json")
        with (patch("shutil.copyfileobj", side_effect=PermissionError("Access denied")),
              self.assertRaises(PermissionError) as failure):
            preservation.preserve(self.worktree, self.destination)
        self.assertIn(str(source.resolve()), str(failure.exception))
        self.assertEqual(source.read_bytes(), b"actual output")

    def test_changed_producer_cannot_report_a_successful_copy(self):
        source = self.output(".agent/tmp/coverage/report.json", b"before")
        native = preservation.copy_whole

        def copy(primary, original, target):
            result = native(primary, original, target)
            source.write_bytes(b"changed after copy")
            return result

        with patch.object(preservation, "copy_whole", side_effect=copy), self.assertRaises(ValueError):
            preservation.preserve(self.worktree, self.destination)
        self.assertTrue(source.exists())


if __name__ == "__main__":
    unittest.main()

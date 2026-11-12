"""Task plans remain outside language checks in every scan mode."""

import contextlib
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "checks"))
import check_english as english


class EnglishScope(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name).resolve()

    def write(self, name, content):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        return path

    def scan(self, files, repository_files=()):
        output = io.StringIO()
        with patch.object(english, "REPO_ROOT", self.root), \
             patch.object(english, "repository_targets", return_value=repository_files), \
             patch.object(english, "worker_count", return_value=1), \
             patch.object(sys, "argv", ["check_english.py", *map(str, files)]), \
             contextlib.chdir(self.root), contextlib.redirect_stdout(output):
            result = english.main()
        return result, output.getvalue()

    def test_explicit_task_paths_are_ignored(self):
        task = self.write(".agent/tasks/nested/plan.md", "wrong" * 8)
        for path in (task, task.relative_to(self.root),
                     self.root / ".agent/tasks/../tasks/nested/plan.md"):
            with self.subTest(path=path):
                result, output = self.scan([path])
                self.assertEqual(result, 0, output)
                self.assertIn("English scan: 0 files;", output)

    def test_repository_scan_ignores_task_plans(self):
        task = self.write(".agent/tasks/plan.md", "wrong" * 8)
        document = self.write("docs/example.md", "Valid English text.")
        result, output = self.scan([], [task, document])
        self.assertEqual(result, 0, output)
        self.assertIn("English scan: 1 files;", output)

    def test_neighboring_paths_and_other_explicit_exclusions_still_scan(self):
        task = self.write(".agent/tasks/plan.md", "wrong" * 8)
        for name in (".agent/tasks-other/example.md", ".agent/tmp/example.md"):
            document = self.write(name, "wrong" * 8)
            with self.subTest(path=name):
                result, output = self.scan([task, document])
                self.assertEqual(result, 1, output)
                self.assertIn(str(document), output)
                self.assertNotIn(str(task), output)
                self.assertIn("English scan: 1 files;", output)


if __name__ == "__main__":
    unittest.main()

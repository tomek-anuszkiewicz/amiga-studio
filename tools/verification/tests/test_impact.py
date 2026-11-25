"""Git input discovery for changed-input diagnostics."""
from pathlib import Path
import sys
import tempfile
import unittest

from git_fixtures import restore_repository

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from repository import collect_changes, git


class ChangedInputTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()

    def test_ignored_working_inputs_do_not_count_as_changed_paths(self):
        restore_repository("empty", self.root)
        ignore = (Path(__file__).resolve().parents[3] / ".gitignore").read_bytes()
        (self.root / ".gitignore").write_bytes(ignore)
        git(self.root, "add", ".gitignore")
        git(self.root, "commit", "--amend", "--no-edit", "-q")
        inputs = ("2026-11-18-plan.md", "test-design.md", "decisions.md", "environment.json")
        working = self.root / ".agent/tasks"
        working.mkdir(parents=True)
        for name in inputs:
            (working / name).write_text("working input", encoding="utf-8")
        self.assertEqual(git(self.root, "ls-files", "--others", "--exclude-standard", "--", ".agent/tasks"), b"")
        self.assertEqual(collect_changes(self.root), set())
        (self.root / "DIARY.md").write_text("actual change", encoding="utf-8")
        self.assertEqual(collect_changes(self.root), {"DIARY.md"})

    def test_git_change_boundary_includes_index_worktree_new_files_and_both_rename_ends(self):
        restore_repository("impact", self.root)
        self.assertEqual(collect_changes(self.root), {"old.txt", "new.txt", "staged.txt", "working.txt", "untracked.txt"})


if __name__ == "__main__":
    unittest.main()

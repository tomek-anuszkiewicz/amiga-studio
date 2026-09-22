"""Regression coverage for isolated worktree asset synchronization."""

import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
WORKTREE_SCRIPT = REPOSITORY_ROOT / "tools" / "git" / "worktree.ps1"


class WorktreeContractTests(unittest.TestCase):
    def test_worktree_copies_and_refreshes_a_private_graphify_seed(self):
        source = WORKTREE_SCRIPT.read_text(encoding="utf-8")

        self.assertIn('"graphify-out"', source)
        self.assertIn("function Update-WorktreeGraph", source)
        self.assertIn("graphifyCommand.Path update .", source)
        self.assertIn("Update-WorktreeGraph -WorktreeRoot $targetPath", source)
        self.assertIn("Update-WorktreeGraph -WorktreeRoot $currentRepo", source)


if __name__ == "__main__":
    unittest.main()

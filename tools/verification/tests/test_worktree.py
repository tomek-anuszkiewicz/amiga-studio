"""Detached diagnostics and safe teardown through the actual PowerShell tool."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


class WorktreeToolTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.parent = Path(self.temporary.name)
        self.root = self.parent / "primary"
        self.root.mkdir()
        self.tool = Path(__file__).resolve().parents[2] / "worktree.ps1"
        self.shell = "powershell" if os.name == "nt" else "pwsh"
        self.git("init", "-q", "-b", "master")
        (self.root / ".gitignore").write_text("ignored*\n.agent/\n")
        (self.root / "tracked.txt").write_text("baseline\n")
        self.git("add", ".")
        tree = self.git("write-tree")
        commit = self.git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit-tree", tree, "-m", "fixture")
        self.git("update-ref", "HEAD", commit)

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, stderr=subprocess.PIPE).decode().strip()

    def run_tool(self, *args, root=None):
        # This fixture has no copied assets or graph: keep the installed graph
        # CLI outside PATH so testing worktree behavior cannot index the host.
        environment = os.environ.copy()
        environment["PATH"] = os.pathsep.join({str(Path(shutil.which("git")).parent),
                                               str(Path(shutil.which(self.shell)).parent),
                                               str(Path(os.environ.get("SystemRoot", "/")) / "System32")})
        return subprocess.run([shutil.which(self.shell), "-NoProfile", "-File", str(self.tool), *args],
                              cwd=root or self.root, env=environment, capture_output=True, text=True)

    def test_usage_and_invalid_input_do_not_mutate_the_repository(self):
        before = self.git("worktree", "list", "--porcelain")
        self.assertEqual(self.run_tool().returncode, 0)
        self.assertIn("Usage:", self.run_tool().stdout)
        for args in (("unknown",), ("remove",), ("list", "extra"), ("sync", "-Base", "HEAD")):
            with self.subTest(args=args):
                result = self.run_tool(*args)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertIn("Usage:", result.stdout)
        self.assertEqual(self.git("worktree", "list", "--porcelain"), before)

    def test_detached_checkout_has_no_task_branch_and_safe_removal(self):
        result = self.run_tool("test", "probe", "-Base", "HEAD")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        target = self.parent / "primary-probe"
        self.assertTrue(target.is_dir())
        self.assertEqual(self.git("branch", "--list", "probe"), "")
        self.assertIn("detached", self.git("worktree", "list", "--porcelain"))
        (target / "tracked.txt").write_text("diagnostic changes\n")
        result = self.run_tool("remove", str(target))
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(target.is_dir())
        self.assertEqual((target / "tracked.txt").read_text(), "diagnostic changes\n")
        # Return the disposable fixture to its known baseline before teardown.
        (target / "tracked.txt").write_text("baseline\n")
        result = self.run_tool("remove", str(target))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertFalse(target.exists())
        self.assertNotIn(str(target).replace("\\", "/"), self.git("worktree", "list", "--porcelain"))

    def test_primary_current_unregistered_and_locked_directories_are_preserved(self):
        neighbor = self.parent / "primary-neighbor"
        neighbor.mkdir()
        (neighbor / "unique.txt").write_text("user data\n")
        for target in (self.root, neighbor):
            self.assertNotEqual(self.run_tool("remove", str(target)).returncode, 0)
            self.assertTrue(target.exists())
        target = self.parent / "primary-task"
        self.git("worktree", "add", "-q", "-b", "task", str(target))
        self.assertNotEqual(self.run_tool("remove", str(target), root=target).returncode, 0)
        self.git("worktree", "lock", str(target))
        self.assertNotEqual(self.run_tool("remove", "task").returncode, 0)
        self.assertTrue(target.exists())
        self.git("worktree", "unlock", str(target))
        result = self.run_tool("remove", "task")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue(neighbor.exists())


if __name__ == "__main__":
    unittest.main()

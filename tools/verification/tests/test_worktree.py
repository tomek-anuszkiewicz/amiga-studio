"""Detached diagnostics and safe teardown through the actual PowerShell tool."""
import os
import json
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
        self.assertIn(str(self.tool.parent / "graphify/refresh.py"), result.stdout)
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

    def test_asset_copy_preserves_target_configuration_and_copies_reference_graph(self):
        (self.root / ".gitignore").write_text("/ref_src/*\n!/ref_src/.graphifyignore\n.agent/\n")
        (self.root / "ref_src").mkdir()
        config = self.root / "ref_src/.graphifyignore"
        config.write_text("target-specific/\n")
        self.git("add", ".gitignore", "ref_src/.graphifyignore")
        tree = self.git("write-tree")
        commit = self.git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                          "commit-tree", tree, "-p", "HEAD", "-m", "configuration fixture")
        self.git("update-ref", "HEAD", commit)
        nested = self.root / "ref_src/graphify-out"
        nested.mkdir()
        (nested / "graph.json").write_text("reference seed")
        created = self.run_tool("add", "assets", "-Base", "HEAD")
        self.assertEqual(created.returncode, 0, created.stdout + created.stderr)
        target = self.parent / "primary-assets"
        config.write_text("source-only/\n")
        result = self.run_tool("sync", root=target)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((target / "ref_src/.graphifyignore").read_text(), "target-specific/\n")
        self.assertEqual((target / "ref_src/graphify-out/graph.json").read_text(), "reference seed")
        (nested / "graph.json").write_text("changed reference seed")
        self.assertEqual((target / "ref_src/graphify-out/graph.json").read_text(), "reference seed")
        self.assertEqual(self.run_tool("remove", str(target)).returncode, 0)

    def test_graph_refresh_uses_the_invoking_checkout_script_in_the_target_root(self):
        wrapper = self.parent / "refresh-probe.ps1"
        wrapper.write_text("""param([string]$ToolPath)
function graphify {}
function python {
    param([string]$ScriptPath)
    @{ script = $ScriptPath; root = (Get-Location).Path; selectors = $args } |
        ConvertTo-Json | Set-Content -LiteralPath $env:GRAPHIFY_PROBE_LOG -Encoding UTF8
    $global:LASTEXITCODE = 0
}
& $ToolPath add refresh-probe -Base HEAD
""", encoding="utf-8")
        log = self.parent / "refresh.json"
        environment = dict(os.environ, GRAPHIFY_PROBE_LOG=str(log))
        result = subprocess.run([shutil.which(self.shell), "-NoProfile", "-File", str(wrapper), str(self.tool)],
                                cwd=self.root, env=environment, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        observed = json.loads(log.read_text(encoding="utf-8-sig"))
        target = self.parent / "primary-refresh-probe"
        self.assertEqual(Path(observed["root"]), target)
        self.assertEqual(Path(observed["script"]), self.tool.parent / "graphify/refresh.py")
        self.assertFalse((target / "tools/graphify/refresh.py").exists())
        self.assertEqual(observed["selectors"], ["--graph", "project", "--root", str(target)])
        self.assertEqual(self.run_tool("remove", str(target)).returncode, 0)


if __name__ == "__main__":
    unittest.main()

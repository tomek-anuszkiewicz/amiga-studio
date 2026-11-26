"""Worktree creation, asset copying, and safe teardown through PowerShell."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from git_fixtures import restore_repository


class WorktreeToolTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.parent = Path(self.temporary.name)
        self.root = self.parent / "primary"
        self.tool = Path(__file__).resolve().parents[2] / "worktree.ps1"
        self.shell = shutil.which("pwsh")
        self.assertIsNotNone(self.shell, "PowerShell 7 (pwsh) is required")
        restore_repository("worktree", self.parent)

    def restore(self, state):
        restore_repository("worktree-" + state, self.parent)
        return (self.root / ".git/refs/heads/master").read_text().strip()

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, stderr=subprocess.PIPE).decode().strip()

    def run_tool(self, *args, root=None):
        # Keep these fixture invocations independent of optional host tools.
        environment = os.environ.copy()
        environment["PATH"] = os.pathsep.join({str(Path(shutil.which("git")).parent),
                                               str(Path(self.shell).parent),
                                               str(Path(os.environ.get("SystemRoot", "/")) / "System32")})
        return subprocess.run([self.shell, "-NoProfile", "-File", str(self.tool), *args],
                              cwd=root or self.root, env=environment, capture_output=True, text=True,
                              errors="replace", timeout=30)

    def test_usage_and_invalid_input_do_not_mutate_the_repository(self):
        before = self.git("worktree", "list", "--porcelain")
        usage = self.run_tool()
        self.assertEqual(usage.returncode, 0)
        self.assertIn("Usage:", usage.stdout)
        self.assertNotRegex(usage.stdout, r"\bsync\b")
        for args in (("unknown",), ("remove",), ("list", "extra")):
            with self.subTest(args=args):
                result = self.run_tool(*args)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertIn("Usage:", result.stdout)
        self.assertEqual(self.git("worktree", "list", "--porcelain"), before)

    def test_sync_is_rejected_without_changing_a_linked_checkout(self):
        target = self.parent / "primary-existing"
        self.restore("existing")
        (self.root / ".env").write_text("FIXTURE=source\n")
        registrations = self.git("worktree", "list", "--porcelain")
        refs = self.git("show-ref")

        result = self.run_tool("sync", root=target)

        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("Invalid action or parameter combination", result.stdout)
        self.assertEqual(self.git("worktree", "list", "--porcelain"), registrations)
        self.assertEqual(self.git("show-ref"), refs)
        self.assertFalse((target / ".env").exists())

    def test_detached_checkout_has_no_task_branch_and_safe_removal(self):
        result = self.run_tool("test", "probe", "-Base", "HEAD")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        target = self.parent / "primary-probe"
        self.assertTrue(target.is_dir())
        self.assertNotIn("refresh", result.stdout.lower())
        self.assertEqual(self.git("branch", "--list", "probe"), "")
        self.assertIn("detached", self.git("worktree", "list", "--porcelain"))
        (target / "tracked.txt").write_text("diagnostic changes\n")
        result = self.run_tool("remove", str(target))
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(target.is_dir())
        self.assertEqual((target / "tracked.txt").read_text(), "diagnostic changes\n")
        # Restore the exact fixture bytes before teardown, including line endings.
        (target / "tracked.txt").write_bytes(b"baseline\n")
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
        self.restore("task")
        self.assertNotEqual(self.run_tool("remove", str(target), root=target).returncode, 0)
        mismatch = self.run_tool("remove", str(target), "-ExpectedBranch", "refs/heads/other")
        self.assertNotEqual(mismatch.returncode, 0)
        self.assertIn("Worktree branch changed before removal.", mismatch.stderr)
        self.assertTrue(target.exists())
        self.git("worktree", "lock", str(target))
        self.assertNotEqual(self.run_tool("remove", "task").returncode, 0)
        self.assertTrue(target.exists())
        self.git("worktree", "unlock", str(target))
        result = self.run_tool("remove", "task")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue(neighbor.exists())

    def test_creation_copies_only_selected_local_assets(self):
        old_base = self.restore("assets")
        excluded_directories = (".agent/tmp/singlestep", ".agent/tmp/benchmarks")
        config = "ref_src/.graphifyignore"
        reference = "Obsidian/Amiga/Reference"
        kickstarts = "tests/kickstarts"
        kickstart = f"{kickstarts}/kickstart-1.3.rom"
        ignore_file = self.root / ".gitignore"
        old_ignore = ignore_file.read_bytes()
        ignore_file.write_bytes(old_ignore + b"\n/tests/kickstarts/\n")
        self.git("add", ".gitignore")
        self.git("commit", "--quiet", "--message", "Ignore local Kickstarts")
        commit = self.git("rev-parse", "HEAD")

        (self.root / config).write_text("source-only/\n")
        (self.root / reference / "tracked.txt").write_bytes(b"source copy of the tracked note\n")
        self.git("rm", "--cached", f"{reference}/tracked.txt")
        (self.root / reference / "source-only.md").write_bytes(b"source branch note\n")
        self.git("add", f"{reference}/source-only.md")
        self.git("commit", "--quiet", "--message", "Change source reference paths")
        # Leave a draft unignored only in the source checkout.
        ignore_file.write_bytes(old_ignore + b"\n/tests/kickstarts/**/*.rom\n")

        for directory in excluded_directories:
            source = self.root / directory
            (source / "tracked.txt").write_text("locally changed\n")
            (source / "ignored-extra").write_text("ignored content\n")
            (source / "untracked-extra").write_text("untracked content\n")
        assets = {
            "ref_src/graphify-out/graph.json": b"reference seed",
            "ref_src/graphify-out/cache/manifest.json": b"reference sidecar",
            "tools/AmigaTestKit/fixture.bin": b"test kit fixture",
            kickstart: b"\x00synthetic Kickstart\xff",
            f"{kickstarts}/local [copies]/alternate.rom": b"\x00alternate synthetic Kickstart\xff",
            f"{reference}/ignored-manual/Chapter [1] \u0142.md": b"local manual chapter",
            "graphify-out/graph.json": b"project seed",
            "graphify-out/cache/manifest.json": b"project sidecar",
            ".env": b"FIXTURE=source\r\n",
        }
        for name, data in assets.items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)

        draft = f"{reference}/Research draft [local].md"
        draft_content = b"unfinished research"
        (self.root / draft).write_bytes(draft_content)
        kickstart_draft = f"{kickstarts}/local-draft.txt"
        (self.root / kickstart_draft).write_text("untracked draft\n")
        manual = f"{reference}/ignored-manual/Chapter [1] \u0142.md"
        self.assertEqual(self.git("-c", "core.quotePath=false", "check-ignore", "--", manual), manual)
        self.assertEqual(self.git("ls-files", "--others", "--exclude-standard", "--", draft), draft)
        self.assertEqual(self.git("check-ignore", "--", kickstart), kickstart)
        self.assertEqual(self.git("ls-files", "--others", "--exclude-standard", "--", kickstart_draft), kickstart_draft)

        for action in ("add", "test"):
            with self.subTest(action=action):
                name = f"{action}-assets"
                target = self.parent / f"primary-{name} [local] \u0142"
                result = self.run_tool(action, name, "-Base", commit, "-Path", str(target))
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual((target / config).read_text(), "target-specific/\n")
                self.assertEqual((target / reference / "tracked.txt").read_bytes(), b"checked-out content\n")
                self.assertFalse((target / reference / "source-only.md").exists())
                for asset, data in assets.items():
                    self.assertEqual((target / asset).read_bytes(), data, asset)
                self.assertFalse((target / draft).exists())
                self.assertFalse((target / kickstart_draft).exists())
                self.assertEqual((self.root / draft).read_bytes(), draft_content)
                for directory in excluded_directories:
                    checkout = target / directory
                    self.assertEqual((checkout / "tracked.txt").read_text(), "checked-out content\n")
                    self.assertFalse((checkout / "ignored-extra").exists(), directory)
                    self.assertFalse((checkout / "untracked-extra").exists(), directory)
                ref_graph = "ref_src/graphify-out/graph.json"
                (target / ref_graph).write_bytes(b"updated target graph")
                self.assertEqual((self.root / ref_graph).read_bytes(), assets[ref_graph])
                (target / kickstart).write_bytes(b"updated target Kickstart")
                self.assertEqual((self.root / kickstart).read_bytes(), assets[kickstart])

        result = self.run_tool("test", "before-kickstart-ignore", "-Base", old_base)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        old_target = self.parent / "primary-before-kickstart-ignore"
        self.assertFalse((old_target / kickstart).exists())
        self.assertEqual(self.git("-C", str(old_target), "status", "--porcelain", "--", kickstarts), "")
        # Remove the detached checkout using its exact path.
        target = self.parent / "primary-test-assets [local] \u0142"
        removed = self.run_tool("remove", str(target))
        self.assertEqual(removed.returncode, 0, removed.stdout + removed.stderr)
        self.assertFalse(target.exists())

    def test_creation_preserves_index_ownership_across_asset_roots(self):
        self.restore("assets")
        ignore_file = self.root / ".gitignore"
        ignore_file.write_bytes(ignore_file.read_bytes() + b"\n/tests/kickstarts/\n")
        self.git("add", ".gitignore")
        roots = ("ref_src", "tools/AmigaTestKit", "graphify-out", "Obsidian/Amiga/Reference/ignored-manual",
                 "tests/kickstarts")
        target_owned = {f"{root}/release [\u0142].txt": b"target branch content" for root in roots}
        for name, data in target_owned.items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        self.git("add", "--force", "--", *target_owned)
        self.git("commit", "--quiet", "--message", "Track target branch assets")
        commit = self.git("rev-parse", "HEAD")
        self.git("rm", "--cached", "--", *target_owned)
        for name in target_owned:
            (self.root / name).write_bytes(b"local source content with a different size")

        source_owned = tuple(f"{root}/pending [\u0142].txt" for root in roots) + (".env",)
        for name in source_owned:
            (self.root / name).write_bytes(b"source index only")
        self.git("add", "--force", "--", *source_owned)
        indexed = self.git("-c", "core.quotePath=false", "ls-files", "--", *source_owned, *target_owned)
        self.assertCountEqual(indexed.splitlines(), source_owned)

        result = self.run_tool("test", "index-ownership", "-Base", commit)

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        target = self.parent / "primary-index-ownership"
        for name in source_owned:
            with self.subTest(source_index=name):
                self.assertFalse((target / name).exists())
                self.assertEqual((self.root / name).read_bytes(), b"source index only")
        for name, data in target_owned.items():
            with self.subTest(target_index=name):
                self.assertEqual((target / name).read_bytes(), data)
                self.assertEqual((self.root / name).read_bytes(), b"local source content with a different size")

    def test_creation_preserves_an_existing_env_file(self):
        commit = self.restore("environment")
        env_file = self.root / ".env"
        env_file.write_text("FIXTURE=source\n")

        result = self.run_tool("add", "existing-env", "-Base", commit)

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual((self.parent / "primary-existing-env/.env").read_text(), "FIXTURE=target\n")
        self.assertEqual(env_file.read_text(), "FIXTURE=source\n")

    def test_git_creation_failure_stops_before_asset_copy(self):
        (self.root / ".env").write_text("FIXTURE=source\n")
        registrations = self.git("worktree", "list", "--porcelain")
        refs = self.git("show-ref")

        result = self.run_tool("add", "bad-base", "-Base", "refs/heads/missing-base")

        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("Git failed with exit code", result.stderr)
        self.assertNotIn("Copying ignored assets", result.stdout)
        self.assertFalse((self.parent / "primary-bad-base").exists())
        self.assertEqual(self.git("worktree", "list", "--porcelain"), registrations)
        self.assertEqual(self.git("show-ref"), refs)

    def test_copy_failure_stops_creation_and_preserves_the_partial_worktree(self):
        commit = self.restore("conflict")
        (self.root / "ref_src").unlink()
        for directory in ("ref_src", "tools/AmigaTestKit", "graphify-out"):
            source = self.root / directory
            source.mkdir(parents=True)
            (source / "fixture.txt").write_text("local asset\n")
        (self.root / ".env").write_text("FIXTURE=source\n")

        result = self.run_tool("add", "copy-failure", "-Base", commit)

        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertRegex(result.stderr, r"Asset copy failed with exit code \d+ for ref_src")
        self.assertEqual(result.stdout.count("[COPYING] ref_src"), 1)
        self.assertNotIn("Graphify", result.stdout)
        self.assertNotIn("created successfully", result.stdout)
        target = self.parent / "primary-copy-failure"
        self.assertIn(f"worktree {target.as_posix()}", self.git("worktree", "list", "--porcelain"))
        self.assertEqual(self.git("rev-parse", "refs/heads/copy-failure"), commit)
        self.assertEqual((target / "ref_src").read_text(), "destination conflict\n")
        self.assertFalse((target / "tools/AmigaTestKit").exists())
        self.assertFalse((target / "graphify-out").exists())
        self.assertFalse((target / ".env").exists())

    def test_creation_does_not_launch_graph_producers(self):
        wrapper = self.parent / "graph-probe.ps1"
        wrapper.write_text("""param([string]$ToolPath, [string]$Action)
function graphify {
    Add-Content -LiteralPath $env:GRAPHIFY_PROBE_LOG -Value 'graphify'
}
function python {
    Add-Content -LiteralPath $env:GRAPHIFY_PROBE_LOG -Value 'python'
}
function uv {
    Add-Content -LiteralPath $env:GRAPHIFY_PROBE_LOG -Value 'uv'
}
& $ToolPath $Action "$Action-graph-probe" -Base HEAD
""", encoding="utf-8")
        for action in ("add", "test"):
            with self.subTest(action=action):
                log = self.parent / f"{action}-graph.log"
                environment = dict(os.environ, GRAPHIFY_PROBE_LOG=str(log))
                result = subprocess.run(
                    [self.shell, "-NoProfile", "-File", str(wrapper), str(self.tool), action],
                    cwd=self.root, env=environment, capture_output=True, text=True, timeout=30,
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertTrue((self.parent / f"primary-{action}-graph-probe").is_dir())
                commands = log.read_text().splitlines() if log.exists() else []
                self.assertEqual(commands, [], result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()

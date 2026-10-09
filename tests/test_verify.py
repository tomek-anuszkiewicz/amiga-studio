"""Protect change selection and exact-index verification from false green results."""

import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools/harness"))
SPEC = importlib.util.spec_from_file_location("verify", ROOT / "tools/harness/verify.py")
VERIFY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VERIFY)
import verify_selection as SELECT


class TestSelection(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.graph = SELECT.package_graph(ROOT)

    def commands(self, *paths, mode="commit"):
        with patch.object(SELECT, "package_graph", return_value=self.graph):
            return SELECT.select_tests(ROOT, set(paths), mode)

    def suites(self, commands):
        return {cmd[i + 1] for _, cmd, _ in commands for i, value in enumerate(cmd) if value == "--test"}

    def test_physical_memory_selects_real_cpu_and_machine_but_not_synthetic_dma(self):
        commands = self.commands("crates/physical_memory/src/map.rs")
        crates = {cmd[i + 1] for _, cmd, _ in commands for i, value in enumerate(cmd) if value == "-p"}
        self.assertTrue({"cpu", "physical_memory", "memory_bus", "machine_loop", "debugger", "gui"} <= crates)
        self.assertNotIn("test_dma_cartesian", self.suites(commands))
        self.assertNotIn("test_singlestep", self.suites(commands))

    def test_shared_bus_contract_and_cpu_force_full_silicon_verification(self):
        for path in ("crates/physical_memory/src/address_bus.rs", "crates/cpu/src/state.rs"):
            commands = self.commands(path)
            self.assertTrue({"test_dma_cartesian", "test_singlestep"} <= self.suites(commands))
            self.assertTrue(any(env.get("SINGLESTEP_FULL") == "1" for _, _, env in commands))

    def test_nested_blitter_selects_parent_environments(self):
        commands = self.commands("crates/blitter/src/blitter.rs")
        crates = {cmd[i + 1] for _, cmd, _ in commands for i, value in enumerate(cmd) if value == "-p"}
        self.assertTrue({"blitter", "agnus", "memory_bus", "machine_loop", "debugger", "gui"} <= crates)
        self.assertIn("test_vamiga_blitter", self.suites(commands))
        self.assertNotIn("test_singlestep", self.suites(commands))

    def test_deleted_test_and_manifest_changes_are_not_lost(self):
        self.assertIn("test_singlestep", self.suites(self.commands("crates/test_runner/src/runner.rs")))
        self.assertIn("test_dma_cartesian", self.suites(self.commands("Cargo.lock")))
        # This named target is also imported by another target via #[path].
        self.assertIn("test_benchmark_csv", self.suites(self.commands("crates/test_runner/tests/test_golden_row_hashes.rs")))

    def test_milestone_selects_all_tests_and_external_capture_comparison(self):
        commands = self.commands(mode="milestone")
        expected = {p.stem for p in (ROOT / "crates/test_runner/tests").glob("test_*.rs")} - {"test_architecture_rules"}
        self.assertEqual(expected, self.suites(commands))
        self.assertTrue(any("--compare" in cmd for _, cmd, _ in commands))
        self.assertTrue(any("vamiga" in cmd and "all" in cmd for _, cmd, _ in commands))

    def test_python_contracts_follow_the_tool_environment(self):
        harness = self.commands("tools/harness/verify.py")
        names = {name for name, _, _ in harness}
        self.assertIn("Python: test_verify", names)
        self.assertNotIn("Python: test_conversion_publish", names)
        converter = self.commands("tools/bootstrap/conversion/lineage.py")
        self.assertIn("Python: test_conversion_publish", {name for name, _, _ in converter})

    def test_cli_rejects_unknown_or_conflicting_modes(self):
        for args in (("--commit", "--quick"), ("--commit", "--milestone"), ()):
            result = subprocess.run([sys.executable, str(ROOT / "tools/harness/verify.py"), *args], capture_output=True)
            self.assertEqual(result.returncode, 2)

    def test_vamiga_printed_failure_is_not_a_successful_gate(self):
        command = ["cargo", "run"]
        for output in ("Executed: 2\nActive Baseline: 2\nFailed / Regressed: 1\n", ""):
            with patch.object(VERIFY, "run_cmd", return_value=(0, output, "", 0)):
                self.assertFalse(VERIFY.execute_test("All runnable vAmigaTS captures", command, {})[0])
        with patch.object(VERIFY, "run_cmd", return_value=(0, "Executed: 2\nActive Baseline: 2\nFailed / Regressed: 0\n", "", 0)):
            self.assertTrue(VERIFY.execute_test("All runnable vAmigaTS captures", command, {})[0])


class TestIndexSnapshot(unittest.TestCase):
    def test_partial_staging_and_untracked_sources_are_rejected_and_rename_keeps_both_paths(self):
        scratch = ROOT / ".agent/tmp"
        scratch.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=scratch) as directory:
            root = Path(directory)
            def git(*args):
                subprocess.run(["git", *args], cwd=root, check=True, capture_output=True)
            git("init", "-q")
            (root / "old.rs").write_text("old", encoding="utf-8")
            git("add", "old.rs")
            git("-c", "user.name=Verification Fixture", "-c", "user.email=fixture@example.invalid",
                "-c", "commit.gpgsign=false", "commit", "-qm", "Initial fixture")
            git("mv", "old.rs", "new.rs")
            self.assertEqual(SELECT.collect_changes(root, "commit"), {"old.rs", "new.rs"})
            before = SELECT.snapshot(root)
            (root / "new.rs").write_text("unstaged", encoding="utf-8")
            self.assertNotEqual(before, SELECT.snapshot(root))
            with self.assertRaisesRegex(RuntimeError, "unstaged"):
                SELECT.collect_changes(root, "commit")
            git("add", "new.rs")
            (root / "extra.rs").write_text("untracked", encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "Untracked"):
                SELECT.collect_changes(root, "commit")


if __name__ == "__main__":
    unittest.main()

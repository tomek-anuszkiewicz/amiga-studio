"""Protect working-tree selection and verification from false green results."""

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
    def commands(self, *paths, mode="commit", **forced):
        return SELECT.select_tests(ROOT, set(paths), mode, **forced)

    def suites(self, commands):
        return {cmd[i + 1] for _, cmd, _ in commands for i, value in enumerate(cmd) if value == "--test"}

    def test_standard_profile_is_identical_for_unrelated_changes(self):
        expected = self.commands("docs/testing.md")
        for path in ("crates/physical_memory/src/map.rs", "crates/blitter/src/blitter.rs",
                     "crates/gui/src/app.rs", "tools/bootstrap/conversion/lineage.py"):
            self.assertEqual(expected, self.commands(path))
        suites = self.suites(expected)
        self.assertNotIn("test_dma_cartesian", suites)
        self.assertNotIn("test_singlestep", suites)
        self.assertIn("test_vamiga_blitter", suites)
        self.assertIn("test_benchmark_trace", suites)
        self.assertTrue(any("--workspace" in cmd for _, cmd, _ in expected))
        self.assertTrue(any(name == "All Python tool contracts" for name, _, _ in expected))

    def test_shared_bus_contract_and_cpu_force_full_silicon_verification(self):
        for path in ("crates/physical_memory/src/address_bus.rs", "crates/cpu/src/state.rs",
                     "crates/cpu/Cargo.toml", "crates/config/src/registers.rs", "Cargo.lock"):
            commands = self.commands(path)
            self.assertTrue({"test_dma_cartesian", "test_singlestep"} <= self.suites(commands))
            self.assertTrue(any(env.get("SINGLESTEP_FULL") == "1" for _, _, env in commands))

    def test_each_harness_triggers_only_its_expensive_suite(self):
        for path in ("crates/test_runner/src/dma_harness.rs", "crates/test_runner/tests/test_dma_cartesian.rs"):
            suites = self.suites(self.commands(path))
            self.assertIn("test_dma_cartesian", suites)
            self.assertNotIn("test_singlestep", suites)
        for path in ("crates/test_runner/src/runner.rs", "crates/test_runner/tests/test_singlestep.rs"):
            suites = self.suites(self.commands(path))
            self.assertIn("test_singlestep", suites)
            self.assertNotIn("test_dma_cartesian", suites)

    def test_force_flags_are_independent_and_additive(self):
        for flag, expected, absent in (("verify_dma", "test_dma_cartesian", "test_singlestep"),
                                      ("verify_singlestep", "test_singlestep", "test_dma_cartesian")):
            suites = self.suites(self.commands("docs/testing.md", **{flag: True}))
            self.assertIn(expected, suites)
            self.assertNotIn(absent, suites)
        commands = self.commands("docs/testing.md", verify_dma=True, verify_singlestep=True)
        self.assertTrue({"test_dma_cartesian", "test_singlestep"} <= self.suites(commands))
        for suite in ("test_dma_cartesian", "test_singlestep"):
            self.assertEqual(sum(cmd.count(suite) for _, cmd, _ in commands), 1)

    def test_standalone_selection_contains_only_explicit_sets(self):
        for flag, expected in (("verify_dma", "test_dma_cartesian"),
                               ("verify_singlestep", "test_singlestep")):
            commands = self.commands("crates/cpu/src/state.rs", mode="standalone", **{flag: True})
            self.assertEqual(len(commands), 1)
            self.assertEqual(self.suites(commands), {expected})
        commands = self.commands(mode="standalone", verify_dma=True, verify_singlestep=True)
        self.assertEqual(len(commands), 2)
        self.assertEqual(self.suites(commands), {"test_dma_cartesian", "test_singlestep"})

    def test_standalone_cli_executes_without_staging_or_quality_gates(self):
        import contextlib
        import io
        for flag, expected in (("--verify-dma", "Cartesian DMA"),
                               ("--verify-singlestep", "Full SingleStepTests")):
            with (
                patch.object(VERIFY, "collect_changes") as collect,
                patch.object(VERIFY, "snapshot", return_value="stable"),
                patch.object(VERIFY, "quality_gates") as gates,
                patch.object(VERIFY, "execute_test", return_value=(True, "passed", 0)) as execute,
                contextlib.redirect_stdout(io.StringIO()),
            ):
                self.assertEqual(VERIFY.main([flag]), 0)
            collect.assert_not_called()
            gates.assert_not_called()
            self.assertEqual(execute.call_count, 1)
            self.assertEqual(execute.call_args.args[0], expected)

    def test_milestone_is_disjoint_from_commit_even_when_cpu_changed(self):
        commands = self.commands("crates/cpu/src/state.rs", mode="milestone")
        self.assertEqual({name for name, _, _ in commands},
                         {"Chipset throughput", "All runnable vAmigaTS captures"})
        self.assertEqual(self.suites(commands), set())
        self.assertTrue(any("--compare" in cmd for _, cmd, _ in commands))
        self.assertTrue(any("vamiga" in cmd and "all" in cmd for _, cmd, _ in commands))
        commit = {name for name, _ in VERIFY.quality_gates(False)}
        milestone = {name for name, _ in VERIFY.quality_gates(True)}
        self.assertFalse(commit & milestone)
        self.assertEqual(len(milestone), 4)
        forced = self.commands(mode="milestone", verify_dma=True, verify_singlestep=True)
        self.assertEqual(self.suites(forced), {"test_dma_cartesian", "test_singlestep"})

    def test_cli_passes_both_force_flags_to_selection(self):
        import contextlib
        import io
        with patch.object(VERIFY, "collect_changes", return_value={"docs/testing.md"}), \
             patch.object(VERIFY, "snapshot", return_value="stable"), \
             patch.object(VERIFY, "select_tests", return_value=[]) as select, \
             contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(VERIFY.main(["--commit", "--verify-dma", "--verify-singlestep", "--plan"]), 0)
        self.assertEqual(select.call_args.kwargs, {"verify_dma": True, "verify_singlestep": True})

    def test_completion_profiles_run_and_plan_on_a_clean_tree(self):
        import contextlib
        import io
        for mode in ("--commit", "--milestone"):
            for plan in (False, True):
                with (
                    patch.object(VERIFY, "collect_changes", return_value=set()),
                    patch.object(VERIFY, "snapshot", return_value="stable"),
                    patch.object(VERIFY, "quality_gates", return_value=[]),
                    patch.object(VERIFY, "execute_test", return_value=(True, "passed", 0)) as execute,
                    contextlib.redirect_stdout(io.StringIO()),
                ):
                    self.assertEqual(VERIFY.main([mode] + (["--plan"] if plan else [])), 0)
                self.assertEqual(execute.call_count, 0 if plan else 3 if mode == "--commit" else 2)

    def test_cli_rejects_unknown_or_conflicting_modes(self):
        for args in (("--commit", "--quick"), ("--commit", "--milestone"), ("--plan",)):
            result = subprocess.run([sys.executable, str(ROOT / "tools/harness/verify.py"), *args], capture_output=True)
            self.assertEqual(result.returncode, 2)

    def test_no_arguments_show_instructions_without_running_verification(self):
        import contextlib
        import io
        output = io.StringIO()
        with (
            patch.object(VERIFY, "collect_changes") as collect,
            patch.object(VERIFY, "snapshot") as snapshot,
            patch.object(VERIFY, "quality_gates") as gates,
            patch.object(VERIFY, "execute_test") as execute,
            contextlib.redirect_stdout(output),
        ):
            self.assertEqual(VERIFY.main([]), 0)
        for operation in (collect, snapshot, gates, execute):
            operation.assert_not_called()
        for text in ("--commit", "--milestone", "--verify-dma", "--verify-singlestep",
                     "--plan", "staged, unstaged or new", "without\n    executing"):
            self.assertIn(text, output.getvalue())

    def test_vamiga_printed_failure_is_not_a_successful_gate(self):
        command = ["cargo", "run"]
        for output in ("Executed: 2\nActive Baseline: 2\nFailed / Regressed: 1\n", ""):
            with patch.object(VERIFY, "run_cmd", return_value=(0, output, "", 0)):
                self.assertFalse(VERIFY.execute_test("All runnable vAmigaTS captures", command, {})[0])
        with patch.object(VERIFY, "run_cmd", return_value=(0, "Executed: 2\nActive Baseline: 2\nFailed / Regressed: 0\n", "", 0)):
            self.assertTrue(VERIFY.execute_test("All runnable vAmigaTS captures", command, {})[0])


class TestWorkingTreeSnapshot(unittest.TestCase):
    def test_clean_partial_staging_and_untracked_sources_are_allowed_and_fingerprinted(self):
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
            self.assertEqual(SELECT.collect_changes(root), set())
            git("mv", "old.rs", "new.rs")
            self.assertEqual(SELECT.collect_changes(root), {"old.rs", "new.rs"})
            before = SELECT.snapshot(root)
            (root / "new.rs").write_text("unstaged", encoding="utf-8")
            self.assertNotEqual(before, SELECT.snapshot(root))
            self.assertEqual(SELECT.collect_changes(root), {"old.rs", "new.rs"})
            (root / "extra.rs").write_text("untracked", encoding="utf-8")
            self.assertEqual(SELECT.collect_changes(root), {"old.rs", "new.rs", "extra.rs"})
            before = SELECT.snapshot(root)
            (root / "extra.rs").write_text("changed untracked", encoding="utf-8")
            self.assertNotEqual(before, SELECT.snapshot(root))


if __name__ == "__main__":
    unittest.main()

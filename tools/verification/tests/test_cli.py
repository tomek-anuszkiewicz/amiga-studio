"""Behavioral contracts for focused selection and completion verification."""
import contextlib
import io
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from cli import parse_args
from commands import select_phases, SINGLESTEP_PATHS, DMA_PATHS
import verify


def selected(sentence, changes=()):
    _, args = parse_args(sentence.split())
    return [item for _, commands in select_phases(args, set(changes)) for item in commands]


class SelectionTests(unittest.TestCase):
    def test_focused_rust_preserves_suite_and_case_without_completion(self):
        commands = selected("test rust crate test_runner suite test_singlestep case test_sub")
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][1], ["cargo", "test", "-p", "test_runner", "--test",
                                         "test_singlestep", "--", "--quiet", "test_sub"])
        self.assertEqual(commands[0][2], {})

    def test_python_selection_preserves_directory_file_and_case(self):
        commands = selected("test python directory tools/bootstrap/tests file test_bootstrap_contract.py case test_all")
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][1][1:], ["-m", "unittest", "discover", "-s", "tools/bootstrap/tests",
                                            "-q", "-p", "test_bootstrap_contract.py", "-k", "test_all"])

    def test_full_and_sampled_silicon_runs_override_inherited_selection(self):
        commands = selected("test runner singlestep case test_add_b sample 500")
        self.assertEqual(commands[0][2], {"SINGLESTEP_FULL": "0", "SINGLESTEP_LIMIT": "500"})
        self.assertEqual(commands[0][1][-1], "test_add_b")
        self.assertEqual(selected("test runner singlestep")[0][2],
                         {"SINGLESTEP_FULL": "1", "SINGLESTEP_LIMIT": "all"})

    def test_completion_selects_cpu_suites_only_for_relevant_changes(self):
        for path in SINGLESTEP_PATHS[-2:]:
            self.assertTrue(any("test_singlestep" in item[1] for item in selected("commit", [path])))
        self.assertTrue(any("test_dma_cartesian" in item[1] for item in selected("commit", [DMA_PATHS[-1]])))
        self.assertFalse(any("test_singlestep" in item[1] for item in selected("commit", ["docs/testing.md"])))
        self.assertFalse(any("test_dma_cartesian" in item[1] for item in selected("commit", ["docs/testing.md"])))

    def test_explicit_cpu_sets_run_once_alongside_automatic_selection(self):
        commands = selected("commit singlestep dma", ["crates/cpu/src/cpu.rs"])
        for suite in ("test_singlestep", "test_dma_cartesian"):
            self.assertEqual(sum(suite in item[1] for item in commands), 1)

    def test_milestone_retains_separate_scope(self):
        commands = selected("milestone", ["crates/cpu/src/cpu.rs"])
        self.assertEqual(len(commands), 1)
        self.assertTrue(commands[0][1][-1].endswith("check_english.py"))
        self.assertEqual(len(selected("milestone singlestep dma")), 3)

    def test_commit_keeps_coverage_quality_and_all_python_directories(self):
        commands = selected("commit")
        self.assertTrue(any("llvm-cov" in item[1] for item in commands))
        self.assertTrue(any(any(part.endswith("checks/check_architecture.py") for part in item[1]) for item in commands))
        self.assertEqual(sum("discover" in item[1] for item in commands), 5)

    def test_focused_architecture_filter_does_not_select_other_checks(self):
        commands = selected("check architecture case test_file_size_limits")
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][1][:-1], [sys.executable, "tools/verification/checks/check_architecture.py"])
        self.assertEqual(commands[0][1][-1], "test_file_size_limits")

    def test_explicit_language_files_bypass_repository_selection(self):
        commands = selected("check english files docs/testing.md AGENTS.md")
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][1][-2:], ["docs/testing.md", "AGENTS.md"])

    def test_settings_can_follow_subcommands_without_eating_runner_arguments(self):
        _, args = parse_args(["runner", "vamiga", "--log-level", "full", "--plan", "category", "all"])
        self.assertTrue(args.plan)
        self.assertEqual(args.log_level, "full")
        command = select_phases(args, set())[0][1][0][1]
        self.assertEqual(command[-3:], ["vamiga", "category", "all"])

    def test_report_output_is_visible_by_default_and_can_be_overridden(self):
        self.assertEqual(parse_args(["runner", "summary"])[1].log_level, "full")
        self.assertEqual(parse_args(["runner", "summary", "--log-level", "error"])[1].log_level, "error")
        self.assertEqual(parse_args(["commit"])[1].log_level, "warn")

    def test_runner_sentences_reach_the_binary_and_profiler_unchanged(self):
        self.assertEqual(selected("runner chipset compare frames 10")[0][1][-4:],
                         ["chipset", "compare", "frames", "10"])
        self.assertEqual(selected("runner profile vamiga test coptim1")[1][1][-3:],
                         ["vamiga", "test", "coptim1"])
        self.assertEqual(selected("runner bench quick pin off")[0][1][-4:],
                         ["bench", "quick", "pin", "off"])
        self.assertEqual(selected("runner suite ADD.b")[0][1][-2:], ["suite", "ADD.b"])
        self.assertEqual(selected("runner summary")[0][1][-1], "summary")
        self.assertEqual(selected("runner diff")[0][1][-1], "diff")

    def test_gui_keeps_its_separate_native_options(self):
        self.assertEqual(selected("runner gui -- --frames 10")[0][1][-2:], ["--frames", "10"])

    def test_invalid_sentences_and_baseline_mutation_are_rejected(self):
        for sentence in ("test rust suite missing", "test rust crate cpu crate gui",
                         "test python directory missing", "test runner dma sample 50",
                         "test runner singlestep sample 0", "test runner singlestep sample 50 full",
                         "commit singlestep singlestep", "check english case name",
                         "runner chipset record", "runner profile chipset record",
                         "runner chipset --record", "runner vamiga -- --category all"):
            with self.subTest(sentence=sentence), self.assertRaises(ValueError):
                selected(sentence)

    def test_legacy_selectors_are_rejected(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as result:
            parse_args(["--commit"])
        self.assertEqual(result.exception.code, 2)


class ExecutionTests(unittest.TestCase):
    def test_failed_profile_build_does_not_launch_a_stale_binary(self):
        with patch.object(verify, "execute_command", return_value=(False, "build failed", 0)) as execute, \
             patch.object(verify, "snapshot", return_value="same"), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(verify.main(["runner", "profile", "vamiga"]), 1)
            self.assertEqual(execute.call_count, 1)
            self.assertEqual(execute.call_args.args[0][:2], ["cargo", "build"])

    def test_plan_never_executes_or_fingerprints_source(self):
        with patch.object(verify, "execute_command") as execute, patch.object(verify, "snapshot") as snapshot:
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(verify.main(["check", "architecture", "--plan"]), 0)
            execute.assert_not_called()
            snapshot.assert_not_called()

    def test_child_failure_and_changed_source_cannot_report_success(self):
        for child_ok, snapshots in ((False, ["same", "same"]), (True, ["before", "after"])):
            with self.subTest(child_ok=child_ok), patch.object(verify, "execute_command", return_value=(child_ok, "result", 0)), \
                 patch.object(verify, "snapshot", side_effect=snapshots), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(verify.main(["check", "english"]), 1)


if __name__ == "__main__":
    unittest.main()

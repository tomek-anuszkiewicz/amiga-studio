"""Behavioral contracts for focused selection and completion verification."""
import contextlib
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from cli import parse_args
from commands import select_phases
from repository import REPO_ROOT
import verify
import commands as selection
import execution
from impact import RustScope


def selected(sentence, changes=()):
    _, args = parse_args(sentence.split())
    return [item for _, commands in select_phases(args, set(changes)) for item in commands]


class SelectionTests(unittest.TestCase):
    def test_startup_help_shows_nested_command_tree_without_selecting_work(self):
        for argv in ([], ["--help"]):
            with self.subTest(argv=argv), contextlib.redirect_stdout(io.StringIO()) as output:
                if argv:
                    with self.assertRaises(SystemExit) as exit_result:
                        parse_args(argv)
                    self.assertEqual(exit_result.exception.code, 0)
                else:
                    _, args = parse_args(argv)
                    self.assertIsNone(args)
            help_text = output.getvalue()
            for command in ("milestone [singlestep]", "crate NAME [suite NAME] [case FILTER]",
                            "python [directory PATH [file PATTERN] [case FILTER]]",
                            "singlestep [case FILTER] [full|sample N]",
                            "architecture [case FILTER]",
                            "analyze RUN_DIRECTORY"):
                self.assertIn(command, help_text)

    def test_focused_rust_preserves_suite_and_case_without_completion(self):
        commands = selected("test rust crate test_runner suite test_singlestep case test_sub")
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][1], [sys.executable, "tools/verification/run_singlestep.py",
                                         "--profile", "test", "--case", "test_sub"])
        self.assertEqual(commands[0][2], {})

    def test_exact_selection_targets_one_named_rust_test_or_singlestep_group(self):
        rust = selected("test rust crate cpu suite test_add case test_add_b --exact")
        self.assertEqual(rust[0][1][-2:], ["test_add_b", "--exact"])
        silicon = selected("test runner singlestep case test_add_b sample 5 --exact")
        self.assertEqual(silicon[0][1][-3:], ["--case", "test_add_b", "--exact"])
        self.assertEqual(silicon[0][2], {"SINGLESTEP_FULL": "0", "SINGLESTEP_LIMIT": "5"})

    def test_exact_without_supported_case_is_rejected(self):
        for sentence in ("test rust unit --exact", "test rust crate cpu --exact",
                         "test rust crate cpu case test_add_b --exact",
                         "test python directory tools/verification/tests case test_cli --exact",
                         "test runner standard --exact", "test runner singlestep --exact",
                         "commit --exact"):
            with self.subTest(sentence=sentence), self.assertRaises(ValueError):
                selected(sentence)

    def test_runner_build_compiles_test_and_benchmark_targets_without_execution(self):
        commands = selected("runner build")
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][1], ["cargo", "test", "--release", "--no-run",
                                         "-p", "test_runner", "--tests", "--benches"])

    def test_python_selection_preserves_directory_file_and_case(self):
        commands = selected("test python directory tools/bootstrap/tests file test_bootstrap_contract.py case test_all")
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][1][1:], ["-m", "unittest", "discover", "-s", "tools/bootstrap/tests",
                                            "-q", "-p", "test_bootstrap_contract.py", "-k", "test_all"])
        book = selected("test python directory tools/bootstrap/book-to-markdown/tests "
                        "file test_book_review_overflow.py case test_pdf_page")
        self.assertEqual(book[0][1][1:], ["tools/bootstrap/book-to-markdown/run_tests.py",
                                         "-p", "test_book_review_overflow.py", "-k", "test_pdf_page"])

    def test_default_python_selection_skips_unrelated_changes(self):
        for changes in ([], ["crates/cpu/src/state.rs"], ["docs/testing.md"],
                        ["tools/bootstrap/conversion-old/client.py"]):
            self.assertEqual(selected("test python", changes), [])

    def test_default_python_selection_covers_owned_code_and_shared_dependencies(self):
        cases = (
            ("tools/bootstrap/ensure_qdrant.ps1", {"Bootstrap contracts"}),
            ("tools/bootstrap/tests/test_bootstrap_contract.py", {"Bootstrap contracts"}),
            ("tools/bootstrap/html-to-markdown/pipeline.py", {"Conversion contracts", "HTML conversion contracts"}),
            ("tools/bootstrap/conversion/client.py", {"Conversion contracts", "HTML conversion contracts", "PDF conversion contracts",
                                                      "Book review contracts"}),
            ("tools/bootstrap/pdf-to-markdown/pipeline.py", {"Conversion contracts", "PDF conversion contracts",
                                                              "Book review contracts"}),
            ("tools/bootstrap/book-to-markdown/review/model.py", {"Book review contracts"}),
            ("tools/amiga-rag-mcp-server/server.py", {"RAG MCP contracts"}),
            (".codex/config.toml", {"RAG MCP contracts"}),
            ("tools/artifacts/report.py", {"Artifact report contracts"}),
            ("tools/artifacts/tests/test_report.py", {"Artifact report contracts"}),
            ("tools/verification/commands.py", {"Verification CLI contracts"}),
        )
        for path, expected in cases:
            with self.subTest(path=path):
                self.assertEqual({item[0] for item in selected("test python", [path])}, expected)

    def test_full_and_sampled_silicon_runs_override_inherited_selection(self):
        commands = selected("test runner singlestep case test_add_b sample 500")
        self.assertEqual(commands[0][2], {"SINGLESTEP_FULL": "0", "SINGLESTEP_LIMIT": "500"})
        self.assertEqual(commands[0][1][-1], "test_add_b")
        self.assertEqual(selected("test runner singlestep")[0][2],
                         {"SINGLESTEP_FULL": "1", "SINGLESTEP_LIMIT": "all"})

    def test_commit_excludes_full_singlestep_even_for_cpu_changes(self):
        for changes in ([], ["crates/cpu/src/cpu.rs"], ["Cargo.toml"],
                        ["crates/cpu/tests/test_cck_bus.rs"], ["docs/testing.md"]):
            commands = selected("commit", changes)
            self.assertFalse(any("tools/verification/run_singlestep.py" in item[1] for item in commands))

    def test_commit_checks_obsidian_frontmatter_without_git_change_selection(self):
        for changes in ([], ["tools/verification/commands.py"]):
            commands = selected("commit", changes)
            self.assertTrue(any(item[1][-1] == "tools/verification/checks/check_obsidian_frontmatter.py"
                                for item in commands))

    def test_empty_standard_harness_never_selects_all_cargo_targets(self):
        with patch.object(Path, "glob", return_value=[Path("test_singlestep.rs")]):
            commands = selected("commit")
            self.assertFalse(any(item[1][:2] == ["cargo", "test"] for item in commands))
            self.assertEqual(selected("test runner standard"), [])

    def test_standard_harness_keeps_explicit_ordinary_targets(self):
        with patch.object(Path, "glob", return_value=[Path("test_singlestep.rs"),
                                                   Path("test_reports.rs")]):
            commands = selection.harness_commands(["standard"])
            self.assertEqual(commands[0][1], ["cargo", "test", "--no-fail-fast", "-p",
                                              "test_runner", "--test", "test_reports",
                                              "--", "--quiet"])

    def test_explicit_singlestep_runs_once_alongside_automatic_selection(self):
        commands = selected("milestone singlestep", ["crates/cpu/src/cpu.rs"])
        self.assertEqual(sum("tools/verification/run_singlestep.py" in item[1]
                             for item in commands), 1)
        self.assertEqual(selected("test runner silicon")[0][1:],
                         selected("test runner singlestep")[0][1:])

    def test_ordinary_cpu_wait_tests_remain_in_normal_selection(self):
        for sentence in ("test rust unit", "commit"):
            commands = selected(sentence, ["crates/cpu/tests/test_cck_bus.rs"])
            self.assertTrue(any("cpu" in item[1] for item in commands
                                if "test" in item[1] or "llvm-cov" in item[1]))

    def test_milestone_retains_separate_scope(self):
        for changes in ([], ["crates/cpu/src/cpu.rs"], ["docs/testing.md"]):
            commands = selected("milestone", changes)
            self.assertEqual(len(commands), 2)
            self.assertTrue(commands[0][1][-1].endswith("check_english.py"))
            self.assertEqual(commands[1][2], {"SINGLESTEP_FULL": "1", "SINGLESTEP_LIMIT": "all"})
            self.assertEqual(commands[1][1][-2:], ["--profile", "verification"])

    def test_commit_skips_rust_but_keeps_quality_and_python_suites_for_bootstrap_changes(self):
        commands = selected("commit", ["tools/bootstrap/book-to-markdown/review/service.py"])
        self.assertFalse(any("llvm-cov" in item[1] or item[1][:2] == ["cargo", "test"] for item in commands))
        self.assertTrue(any(any(part.endswith("checks/check_architecture.py") for part in item[1]) for item in commands))
        self.assertEqual(sum("discover" in item[1] for item in commands), 7)
        self.assertTrue(any("tools/bootstrap/book-to-markdown/run_tests.py" in item[1] for item in commands))

    def test_partial_coverage_runs_once_and_exports_the_same_packages(self):
        with patch.object(selection, "select_rust_scope", return_value=RustScope(("cpu", "gui", "test_runner"), "fixture")):
            commands = selected("commit", ["crates/cpu/src/cpu.rs"])
        run = next(command for _, command, _ in commands if "--no-report" in command)
        export = next(command for _, command, _ in commands if "--json" in command)
        self.assertNotIn("--summary-only", export)
        self.assertEqual(export[-2:], ["--output-path", ".agent/tmp/coverage/coverage.json"])
        summary = next(command for _, command, _ in commands
                       if "tools/verification/coverage_report.py" in command and "--prepare" not in command)
        for command in (run, export):
            self.assertEqual([command[i + 1] for i, word in enumerate(command) if word == "-p"], ["cpu", "gui"])
            self.assertNotIn("--workspace", command)
        self.assertEqual(summary[-4:], ["--package", "cpu", "--package", "gui"])
        plain = [command for _, command, _ in commands if command[:2] == ["cargo", "test"]]
        self.assertEqual(len(plain), 1)
        self.assertIn("test_runner", plain[0])
        self.assertNotIn("test_singlestep", plain[0])

    def test_development_coverage_reuses_pipeline_without_completion_gates(self):
        for sentence, package in (("test coverage crate cpu", "cpu"),
                                  ("test coverage workspace", None)):
            with self.subTest(sentence=sentence):
                commands = selected(sentence)
                self.assertEqual(len(commands), 5)
                run = next(command for _, command, _ in commands if "--no-report" in command)
                self.assertNotIn("cargo test", " ".join(run))
                if package:
                    self.assertEqual(run[run.index("-p") + 1], package)
                    self.assertNotIn("--workspace", run)
                else:
                    self.assertIn("--workspace", run)
                    self.assertEqual(run[run.index("--exclude") + 1], "test_runner")
                self.assertFalse(any("check_" in " ".join(command) for _, command, _ in commands))
                self.assertFalse(any("run_singlestep.py" in " ".join(command) for _, command, _ in commands))

    def test_coverage_rejects_unknown_packages_harness_and_partial_test_filters(self):
        for sentence in ("test coverage crate unknown_package", "test coverage crate test_runner",
                         "test coverage crate cpu suite test_add", "test coverage workspace --exact"):
            with self.subTest(sentence=sentence), self.assertRaises(ValueError):
                selected(sentence)

    def test_unaffected_runner_is_skipped_and_harness_only_change_has_no_coverage(self):
        for packages, harness, coverage in ((("gui",), False, True), (("test_runner",), True, False), ((), False, False)):
            with self.subTest(packages=packages), patch.object(selection, "select_rust_scope", return_value=RustScope(packages, "fixture")):
                commands = selected("commit", ["docs/testing.md"])
            self.assertEqual(any(command[:2] == ["cargo", "test"] for _, command, _ in commands), harness)
            self.assertEqual(any("llvm-cov" in command for _, command, _ in commands), coverage)
            self.assertIn(["cargo", "clippy", "--workspace", "--all-targets"], [command for _, command, _ in commands])

    def test_commit_skips_bootstrap_contracts_without_bootstrap_changes(self):
        bootstrap = {"Bootstrap contracts", "Conversion contracts", "HTML conversion contracts",
                     "PDF conversion contracts", "Book review contracts"}
        for changes in ([], ["crates/cpu/src/cpu.rs"], ["tools/verification/commands.py"]):
            with self.subTest(changes=changes):
                names = {name for name, _, _ in selected("commit", changes)}
                self.assertFalse(names & bootstrap)
                self.assertIn("RAG MCP contracts", names)
                self.assertIn("Artifact report contracts", names)
                self.assertIn("Verification CLI contracts", names)
        for path in ("tools/bootstrap/conversion/client.py",
                     "tools/bootstrap/books/new/source.json"):
            with self.subTest(path=path):
                names = {name for name, _, _ in selected("commit", [path])}
                self.assertEqual(names & bootstrap, bootstrap)

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
        _, args = parse_args(["runner", "bench", "--log-level", "full", "--plan", "case", "add/add_w_d1_d0"])
        self.assertTrue(args.plan)
        self.assertEqual(args.log_level, "full")
        command = select_phases(args, set())[0][1][0][1]
        self.assertEqual(command[1:3], ["tools/verification/cpu_performance_tests.py", "measure"])
        self.assertEqual(command[-2:], ["case", "add/add_w_d1_d0"])

    def test_results_directory_is_a_common_option(self):
        _, args = parse_args(["test", "runner", "singlestep", "case", "test_add_b",
                              "--results-dir", ".agent/tmp/experiments/add"])
        self.assertEqual(args.results_dir, ".agent/tmp/experiments/add")

    def test_report_output_is_visible_by_default_and_can_be_overridden(self):
        self.assertEqual(parse_args(["runner", "analyze", "fixture.csv"])[1].log_level, "full")
        self.assertEqual(parse_args(["runner", "analyze", "fixture.csv", "--log-level", "error"])[1].log_level, "error")
        self.assertEqual(parse_args(["commit"])[1].log_level, "warn")

    def test_benchmark_selects_cpu_performance_tests(self):
        self.assertEqual(selected("runner bench case add/add_w_d1_d0")[0][1][1:],
                         ["tools/verification/cpu_performance_tests.py", "measure", "case", "add/add_w_d1_d0"])
        self.assertEqual(selected("runner analyze full_run")[0][1][1:],
                         ["tools/verification/cpu_performance_tests.py", "analyze", "full_run"])
        self.assertEqual(selected("runner bench cases selected.txt")[0][1][1:],
                         ["tools/verification/cpu_performance_tests.py", "measure", "cases",
                          "selected.txt"])
        self.assertEqual(selected("runner refresh filtered_run")[0][1][1:],
                         ["tools/verification/cpu_performance_tests.py", "refresh", "filtered_run"])

    def test_refresh_allows_intended_current_snapshot_change(self):
        with patch.object(verify, "execute_command", return_value=(True, "refreshed", 0.1)), \
             patch.object(verify, "snapshot") as snapshot, \
             contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(verify.main(["runner", "refresh", "filtered_run"]), 0)
        snapshot.assert_not_called()

    def test_gui_keeps_its_separate_native_options(self):
        self.assertEqual(selected("runner gui -- --frames 10")[0][1][-2:], ["--frames", "10"])

    def test_invalid_sentences_are_rejected(self):
        for sentence in ("test rust suite missing", "test rust crate cpu crate gui",
                         "test python directory missing", "test runner dma", "test runner harness",
                         "test runner dma sample 50", "milestone dma",
                         "test runner singlestep sample 0", "test runner singlestep sample 50 full",
                         "milestone singlestep singlestep", "check english case name",
                         "runner bench -- --quick", "runner bench measure retry",
                         "runner bench duration-ms 0", "runner bench series 0",
                         "runner bench case add/add_w_d1_d0 mnemonic ADD",
                         "runner bench measure filter ADD", "runner bench measure unroll 500",
                         "runner bench measure passes 9", "runner bench measure output reports",
                         "runner bench measure pin off", "runner bench unroll"):
            with self.subTest(sentence=sentence), self.assertRaises(ValueError):
                selected(sentence)

class ExecutionTests(unittest.TestCase):
    def test_failed_coverage_step_stops_export_but_keeps_other_commit_checks(self):
        phases = [("Coverage", [(name, [name], {}) for name in ("prepare", "reset", "tests", "export", "summary")]),
                  ("Verification", [("remaining gate", ["gate"], {})])]
        for failing in ("prepare", "reset", "tests", "export"):
            with self.subTest(failing=failing), \
                 patch.object(verify, "select_phases", return_value=phases), \
                 patch.object(verify, "collect_changes", return_value=set()), \
                 patch.object(verify, "snapshot", return_value="same"), \
                 patch.object(verify, "execute_command", side_effect=lambda command, *args, **kwargs:
                              (command[0] != failing, "fixture", 0)) as execute, \
                 contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(verify.main(["commit"]), 1)
            invoked = [call.args[0][0] for call in execute.call_args_list]
            expected = ["prepare", "reset", "tests", "export"]
            self.assertEqual(invoked, expected[:expected.index(failing) + 1] + ["gate"])

    def test_exact_rust_run_rejects_zero_matching_tests(self):
        for count, expected in ((0, 1), (1, 0)):
            process = unittest.mock.MagicMock()
            process.__enter__.return_value = process
            process.stdout = [f"running {count} test{'s' if count != 1 else ''}\n"]
            process.wait.return_value = 0
            with self.subTest(count=count), tempfile.TemporaryDirectory() as temporary, \
                 patch.object(execution.subprocess, "Popen", return_value=process), \
                 contextlib.redirect_stdout(io.StringIO()):
                code, _ = execution.run_cmd(["cargo", "test", "--", "case", "--exact"],
                                             log_dir=temporary)
            self.assertEqual(code, expected)

    def test_plan_shows_isolated_logs_and_singlestep_reports_without_writing(self):
        with patch.object(verify, "execute_command") as execute, \
             contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(verify.main(["test", "runner", "singlestep", "case", "test_add_b",
                                          "--exact", "--results-dir", ".agent/tmp/experiments/add",
                                          "--plan"]), 0)
        execute.assert_not_called()
        self.assertIn("experiments\\add\\verify", output.getvalue())
        self.assertIn("SINGLESTEP_RESULTS_DIR", output.getvalue())
        self.assertIn(repr(str(REPO_ROOT / ".agent/tmp/experiments/add/singlestep")),
                      output.getvalue())

    def test_results_directory_reaches_execution_and_singlestep_environment(self):
        with patch.object(verify, "execute_command", return_value=(True, "ok", 0)) as execute, \
             patch.object(verify, "snapshot", return_value="same"), \
             contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(verify.main(["test", "runner", "singlestep", "case", "test_add_b",
                                          "--results-dir", ".agent/tmp/experiments/add"]), 0)
        self.assertEqual(execute.call_args.kwargs["log_dir"],
                         REPO_ROOT / ".agent/tmp/experiments/add/verify")
        self.assertEqual(execute.call_args.args[1]["SINGLESTEP_RESULTS_DIR"],
                         str(REPO_ROOT / ".agent/tmp/experiments/add/singlestep"))
    def test_inventory_runs_without_candidate_generation(self):
        for success in (True, False):
            with self.subTest(success=success), \
                 patch.object(verify, "execute_command", return_value=(success, "result", 0)) as execute, \
                 patch.object(verify, "snapshot", return_value="same"), \
                 contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(verify.main(["runner", "bench", "inventory"]), 0 if success else 1)
            self.assertEqual(execute.call_count, 1)
            self.assertEqual(execute.call_args_list[0].args[0][:2], ["cargo", "bench"])

    def test_unrelated_changes_execute_no_python_contracts(self):
        with patch.object(verify, "collect_changes", return_value={"crates/cpu/src/state.rs"}), \
             patch.object(verify, "execute_command") as execute, patch.object(verify, "snapshot") as snapshot:
            with contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(verify.main(["test", "python"]), 0)
        execute.assert_not_called()
        snapshot.assert_not_called()
        self.assertIn("inputs are unchanged", output.getvalue())

    def test_default_python_command_collects_working_tree_changes(self):
        with patch.object(verify, "collect_changes", return_value={"tools/amiga-rag-mcp-server/server.py"}) as collect:
            with contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(verify.main(["test", "python", "--plan"]), 0)
        collect.assert_called_once_with(REPO_ROOT)
        self.assertIn("RAG MCP contracts", output.getvalue())
        self.assertNotIn("Bootstrap contracts", output.getvalue())

    def test_commit_plan_collects_working_tree_changes(self):
        with patch.object(verify, "collect_changes", return_value={"tools/bootstrap/conversion/client.py"}) as collect:
            with contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(verify.main(["commit", "--plan"]), 0)
        collect.assert_called_once_with(REPO_ROOT)
        self.assertIn("Book review contracts", output.getvalue())
        self.assertIn("Rust test scope: none", output.getvalue())
        self.assertIn("previous summary is not evidence", output.getvalue())

    def test_commit_captures_source_before_change_selection(self):
        events = []
        def fingerprint(root):
            events.append("snapshot")
            return "before" if len(events) == 1 else "after"
        def changes(root):
            events.append("changes")
            return {"docs/testing.md"}
        with patch.object(verify, "snapshot", side_effect=fingerprint), \
             patch.object(verify, "collect_changes", side_effect=changes), \
             patch.object(verify, "execute_command", return_value=(True, "ok", 0)), \
             contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(verify.main(["commit"]), 1)
        self.assertEqual(events, ["snapshot", "changes", "snapshot"])
        self.assertIn("Source or index changed", output.getvalue())

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

"""Focused process and case adapter contracts for the reporting launchers."""

from collections import Counter
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from textwrap import dedent
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from case_report import CONTEXT, CaseReport
from run_rust_tests import CargoCases, NextestCases, rust_command
from test_case_report import TASK_CONTEXT, events, init_repository


TOOLS = Path(__file__).resolve().parents[1]


class NativeReportsTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        init_repository(self.root)
        self.env = {key: value for key, value in os.environ.items() if key not in CONTEXT.values()}
        self.env.update(TASK_CONTEXT)

    def test_unittest_direct_and_discovery_keep_original_case_ids_and_outcomes(self):
        source = self.root / "test_fixture.py"
        source.write_text(dedent(r"""
            import time
            import unittest
            from pathlib import Path

            class Example(unittest.TestCase):
                def test_pass(self):
                    started = time.perf_counter()
                    time.sleep(0.02)
                    with Path("ledger").open("a") as log:
                        print("pass", time.perf_counter() - started, file=log)

                def test_failure(self):
                    with Path("ledger").open("a") as log:
                        print("failure", file=log)
                    print("native test marker")
                    self.assertEqual(17, 23)

                def test_decoy(self):
                    raise AssertionError("unexpected decoy selection")
            """), encoding="utf-8")

        selected = [("test_pass", ["test_fixture.Example.test_pass"], 0),
                    ("test_failure", ["discover", "-s", ".", "-p", "test_fixture.py", "-k", "test_failure"], 1)]
        for method, args, exit_code in selected:
            command = [sys.executable, "-B", str(TOOLS / "run_unittest.py"), *args]
            process = subprocess.run(command, cwd=self.root, env=self.env, stdout=subprocess.PIPE,
                                     stderr=subprocess.STDOUT, encoding="utf-8", errors="replace", timeout=15)
            self.assertEqual(process.returncode, exit_code, process.stdout)
            path = next(Path(line.removeprefix("[REPORT] ")) for line in process.stdout.splitlines()
                        if line.startswith("[REPORT] "))
            rows = [json.loads(line) for line in path.with_suffix(".jsonl").read_text(
                encoding="utf-8").splitlines()]
            case, = [row for row in rows if row["event"] == "case_finished"]
            self.assertEqual(case["id"], f"test_fixture.Example.{method}")
            self.assertEqual(rows[-2]["completed"], 1)
            if exit_code:
                self.assertIn("17 != 23", case["detail"])
                self.assertIn("test_fixture.py", case["detail"])
                self.assertIn("native test marker", (path.parent / case["artifact"]).read_text())
            else:
                seconds = float((self.root / "ledger").read_text().splitlines()[0].split()[1])
                self.assertGreaterEqual(case["seconds"], seconds)
                self.assertLess(case["seconds"], rows[-1]["seconds"])
        self.assertEqual([line.split()[0] for line in (self.root / "ledger").read_text().splitlines()],
                         ["pass", "failure"])

    def test_empty_selection_returns_the_same_exit_as_standard_unittest(self):
        selected = ["discover", "-s", ".", "-p", "test_no_cases.py"]
        commands = [[sys.executable, "-m", "unittest", *selected],
                    [sys.executable, str(TOOLS / "run_unittest.py"), *selected]]
        native, reporting = [subprocess.run(command, cwd=self.root, env=self.env, encoding="utf-8",
                                            errors="replace", capture_output=True, timeout=10)
                             for command in commands]
        self.assertEqual(reporting.returncode, native.returncode, reporting.stderr)
        [location] = [line.removeprefix("[REPORT] ") for line in reporting.stdout.splitlines()
                      if line.startswith("[REPORT] ")]
        journal = Path(location).with_suffix(".jsonl")
        counts = [json.loads(line) for line in journal.read_text(encoding="utf-8").splitlines()][-2]
        self.assertEqual((counts["started"], counts["completed"]), (0, 0))

    def test_nextest_adapts_native_durations_and_keeps_abort_and_fail_diagnostics(self):
        archive = CaseReport.reserve("fixture", ["rust"], self.root, self.env)
        parser = NextestCases(archive, None)
        stream = [
            {"event": "started", "name": "sample::test_target$passed"},
            {"event": "started", "name": "sample::test_target$failed"},
            {"event": "started", "name": "sample::test_target$interrupted"},
            {"event": "ignored", "name": "sample::test_target$ignored"},
            {"event": "ok", "name": "sample::test_target$passed", "exec_time": 0.125},
            {"event": "failed", "name": "sample::test_target$failed", "exec_time": 0.25,
             "stdout": "tests/test_fixture.rs:34\nleft: 17\nright: 23"},
        ]
        for row in stream:
            parser.consume(json.dumps({"type": "test", **row}))
        parser.finish()
        rows = events(archive)
        finished = {row["id"]: row for row in rows if row["event"] == "case_finished"}
        self.assertEqual(finished["passed"]["seconds"], 0.125)
        self.assertEqual(finished["failed"]["seconds"], 0.25)
        self.assertIn("right: 23", finished["failed"]["detail"])
        self.assertEqual(finished["ignored"]["outcome"], "skipped")
        self.assertIsNone(finished["ignored"]["seconds"])
        interrupted, = [row for row in rows if row["event"] == "case_unfinished"]
        self.assertEqual((interrupted["id"], interrupted["outcome"]), ("interrupted", "aborted"))
        self.assertEqual(rows[-1]["started"], 3)
        self.assertEqual(rows[-1]["completed"], 2)

    def test_cargo_keeps_release_exact_and_ignored_options_and_marks_unavailable_time(self):
        cargo = ["cargo", "test", "-p", "sample", "--release", "--test", "test_target",
                 "--", "module::case", "--exact", "--ignored", "--nocapture"]
        routed, structured, _ = rust_command(cargo)
        self.assertEqual(routed, cargo + ["--format", "pretty"])
        self.assertFalse(structured)
        archive = CaseReport.reserve("fixture", cargo, self.root, self.env)
        parser = CargoCases(archive, None)
        for line in ["  Running tests/test_target.rs (test_target)\n",
                     "test module::case ... FAILED\n",
                     "---- module::case stdout ----\n", "tests/test_target.rs:34\n",
                     "left: 17\n", "right: 23\n", "failures:\n"]:
            parser.consume(line)
        parser.finish()
        rows = events(archive)
        self.assertEqual(Counter(row["event"] for row in rows)["case_finished"], 1)
        case, = [row for row in rows if row["event"] == "case_finished"]
        self.assertEqual(case["id"], "module::case")
        self.assertIsNone(case["seconds"])
        diagnostic, = [row for row in rows if row["event"] == "case_diagnostic"]
        self.assertIn("right: 23", diagnostic["detail"])
        self.assertEqual(diagnostic["id"], case["id"])


    def test_nextest_uses_retry_suffix_and_original_status_timing(self):
        metadata = {"crate": "sample", "test_binary": "sample", "kind": "lib"}
        suite = {"type": "suite", "nextest": metadata}
        name = "sample::sample$module::case"
        for show_retry_timing in (True, False):
            with self.subTest(show_retry_timing=show_retry_timing):
                report = CaseReport.reserve("fixture", ["rust"], self.root, self.env)
                native = report.attachment("native-output.log")
                lines = [json.dumps({**suite, "event": "started", "test_count": 1}),
                         json.dumps({"type": "test", "event": "started", "name": name})]
                if show_retry_timing:
                    lines += [
                        "  TRY 1 FAIL [   0.375s] (---) sample module::case",
                        "    left: 17",
                        "    right: 23",
                    ]
                lines += [json.dumps({"type": "test", "event": "ok", "name": name + "#2", "exec_time": 0.125}),
                          json.dumps({**suite, "event": "ok", "passed": 1, "failed": 0, "ignored": 0})]
                native.write_text("\n".join(lines), encoding="utf-8")
                parser = NextestCases(report, native)
                for line in lines:
                    parser.consume(line)
                parser.finish()
                report.finish(0)
                rows = events(report)
                attempts = [row for row in rows if row["event"] == "case_finished"]
                self.assertEqual([(row["suite"], row["id"], row["attempt"], row["outcome"]) for row in attempts],
                                 [("sample::sample", "module::case", 1, "failed"),
                                  ("sample::sample", "module::case", 2, "passed")])
                self.assertEqual([row["seconds"] for row in attempts],
                                 [0.375 if show_retry_timing else None, 0.125])
                if show_retry_timing:
                    first = attempts[0]
                    linked = (report.path.parent / first["artifact"]).read_text(encoding="utf-8").splitlines()
                    self.assertEqual(linked[first["native_output_line"] - 1:first["native_output_line"] + 2], lines[2:5])
                self.assertEqual({key: rows[-2][key] for key in ("started", "completed", "aborted")},
                                 {"started": 2, "completed": 2, "aborted": 0})
                self.assertEqual(rows[-1]["evidence"], "recorded" if show_retry_timing else "incomplete")

    def test_nextest_reconciles_ignored_starts_across_native_suite_blocks(self):
        report = CaseReport.reserve("fixture", ["rust"], self.root, self.env)
        parser = NextestCases(report, None)
        suite = {"type": "suite", "nextest": {"crate": "sample", "test_binary": "tests", "kind": "test"}}
        start = {**suite, "event": "started", "test_count": 2}
        finish = {**suite, "event": "ok", "passed": 0, "failed": 0, "ignored": 1}
        native = [
            start,
            {"type": "test", "event": "started", "name": "sample::tests$ignored"},
            finish,
            start,
            {"type": "test", "event": "started", "name": "sample::tests$passes"},
            {"type": "test", "event": "ok", "name": "sample::tests$passes", "exec_time": 0.125},
            {**finish, "passed": 1},
        ]
        for row in native:
            parser.consume(json.dumps(row))
        parser.finish()
        report.finish(0)
        rows = events(report)
        [executed] = [row for row in rows if row["event"] == "case_started"]
        self.assertEqual(executed["id"], "passes")
        self.assertEqual([(row["id"], row["outcome"]) for row in rows if row["event"] == "case_finished"],
                         [("passes", "passed"), ("ignored", "skipped")])
        self.assertEqual({key: rows[-2][key] for key in ("started", "completed", "skipped", "aborted", "unresolved")},
                         {"started": 1, "completed": 1, "skipped": 1, "aborted": 0, "unresolved": 0})
        self.assertEqual(rows[-1]["evidence"], "recorded")

    def test_cargo_summary_preserves_counts_without_inventing_case_names(self):
        cargo = ["cargo", "test", "-p", "sample", "--", "--format", "terse", "test_selected"]
        report = CaseReport.reserve("fixture", cargo, self.root, self.env)
        parser = CargoCases(report, None)
        for line in [
            "  Running tests/test_target.rs (test_target)\n",
            "running 2 tests\n", ".i\n",
            "test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 2 filtered out; finished in 0.00s\n",
            "  Running tests/other_target.rs (other_target)\n",
            "running 1 test\n", "test selected_other ... ok\n",
            "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n",
        ]:
            parser.consume(line)
        parser.finish()
        report.finish(0)
        rows = events(report)
        [case] = [row for row in rows if row["event"] == "case_finished"]
        [summary] = [row for row in rows if row["event"] == "suite_counts"]
        self.assertEqual(case["id"], "selected_other")
        self.assertEqual(summary["suite"], "tests/test_target.rs (test_target)")
        self.assertEqual(summary["unnamed"], {"completed": 1, "skipped": 1})
        self.assertEqual({key: rows[-2][key] for key in ("started", "completed", "skipped")},
                         {"started": 2, "completed": 2, "skipped": 1})
        self.assertEqual((rows[-1]["exit_code"], rows[-1]["evidence"]), (0, "incomplete"))
        self.assertIn("Cargo omitted individual case results", report.path.read_text(encoding="utf-8"))

    def test_cargo_reporting_format_keeps_filters_and_other_formats(self):
        cargo = ["cargo", "test", "--release", "--lib", "--"]
        options = ["--skip", "--format=terse", "--format", "terse", "--", "--quiet"]
        expected = ["--skip", "--format=terse", "--format", "pretty", "--", "--quiet"]
        self.assertEqual(rust_command(cargo + options), (cargo + expected, False, ["pretty"]))
        for format_name in ("pretty", "json", "junit"):
            with self.subTest(format=format_name):
                command = cargo + [f"--format={format_name}", "module::case", "--exact"]
                self.assertEqual(rust_command(command), (command, False, [format_name]))

    def test_cargo_terse_reports_full_case_names_from_a_single_native_run(self):
        (self.root / "Cargo.toml").write_text(dedent("""
            [package]
            name = "native_terse_fixture"
            version = "0.1.0"
            edition = "2021"
        """), encoding="utf-8")
        (self.root / "src").mkdir()
        (self.root / "src/lib.rs").write_text(dedent(r"""
            #[cfg(test)]
            mod sample {
                use std::io::Write;

                fn record(file: &str) {
                    let mut ledger = std::fs::OpenOptions::new()
                        .create(true).append(true).open(file).unwrap();
                    writeln!(ledger, "entered").unwrap();
                }

                #[test]
                fn passes() { record("passes"); }

                #[test]
                fn fails() {
                    record("fails");
                    assert_eq!(17, 23);
                }

                #[test]
                #[ignore = "requires an external service"]
                fn skipped() { record("skipped"); }

                #[test]
                fn decoy() { record("decoy"); }
            }
        """), encoding="utf-8")
        environment = self.env | {"CARGO_TARGET_DIR": str(self.root / "target")}
        launcher = [sys.executable, "-B", str(TOOLS / "run_rust_tests.py"), "--"]
        cargo = ["cargo", "test", "--lib", "--", "--exact", "--test-threads", "2"]
        selections = [
            (["sample::passes", "--format", "terse"], 0, {"sample::passes": "passed"}),
            (["sample::fails", "sample::skipped", "--format=terse"], 101,
             {"sample::fails": "failed", "sample::skipped": "skipped"}),
        ]
        for args, exit_code, expected in selections:
            with self.subTest(args=args):
                process = subprocess.run(launcher + cargo + args, cwd=self.root, env=environment,
                                         capture_output=True, text=True, encoding="utf-8", errors="replace",
                                         timeout=120)
                self.assertEqual(process.returncode, exit_code, process.stdout + process.stderr)
                [location] = [line.removeprefix("[REPORT] ") for line in process.stdout.splitlines()
                              if line.startswith("[REPORT] ")]
                report = Path(location)
                journal = report.with_suffix(".jsonl")
                rows = [json.loads(line) for line in journal.read_text(encoding="utf-8").splitlines()]
                cases = [row for row in rows if row["event"] == "case_finished"]
                self.assertCountEqual([(row["id"], row["outcome"]) for row in cases], list(expected.items()))
                self.assertEqual((rows[-1]["exit_code"], rows[-1]["evidence"]), (exit_code, "recorded"))
                for case in cases:
                    self.assertEqual(case["attempt"], 1)
                    self.assertIsNone(case["seconds"])
                    native = (report.parent / case["artifact"]).read_text(encoding="utf-8")
                    self.assertIn(f"test {case['id']} ...", native)
                if exit_code:
                    [diagnostic] = [row for row in rows if row["event"] == "case_diagnostic"]
                    self.assertEqual(diagnostic["id"], "sample::fails")
                    self.assertIn("src/lib.rs", diagnostic["detail"].replace("\\", "/"))
                    self.assertIn("left: 17", diagnostic["detail"])
                    self.assertIn("right: 23", diagnostic["detail"])
                    [skipped] = [row for row in cases if row["outcome"] == "skipped"]
                    self.assertEqual(skipped["native_reason"], "requires an external service")
        for file in ("passes", "fails"):
            self.assertEqual((self.root / file).read_text(encoding="utf-8").splitlines(), ["entered"])
        for file in ("skipped", "decoy"):
            self.assertFalse((self.root / file).exists())

    def test_cargo_abrupt_exit_preserves_named_start_and_exit_code(self):
        (self.root / "Cargo.toml").write_text(dedent("""
            [package]
            name = "native_crash_fixture"
            version = "0.1.0"
            edition = "2021"
        """), encoding="utf-8")
        (self.root / "src").mkdir()
        (self.root / "src/lib.rs").write_text(dedent("""
            #[test]
            fn stops_before_return() { std::process::exit(2); }
            #[test]
            fn decoy() { panic!("unexpected selection"); }
        """), encoding="utf-8")
        cargo = ["cargo", "test", "--lib", "--", "--exact", "stops_before_return",
                 "--nocapture", "--test-threads=1"]
        environment = self.env | {"CARGO_TARGET_DIR": str(self.root / "target")}
        command = [sys.executable, "-B", str(TOOLS / "run_rust_tests.py"), "--", *cargo]
        process = subprocess.run(command, cwd=self.root, env=environment, capture_output=True,
                                 text=True, encoding="utf-8", errors="replace", timeout=120)
        self.assertEqual(process.returncode, 2, process.stdout + process.stderr)
        [location] = [line.removeprefix("[REPORT] ") for line in process.stdout.splitlines()
                      if line.startswith("[REPORT] ")]
        path = Path(location)
        rows = [json.loads(line) for line in path.with_suffix(".jsonl").read_text(
            encoding="utf-8").splitlines()]
        [started] = [row for row in rows if row["event"] == "case_started"]
        [aborted] = [row for row in rows if row["event"] == "case_unfinished"]
        self.assertEqual((started["id"], aborted["id"]), ("stops_before_return", "stops_before_return"))
        self.assertEqual((started["suite"], started["attempt"]), (aborted["suite"], aborted["attempt"]))
        self.assertEqual((aborted["outcome"], aborted["seconds"], aborted["stack"]), ("aborted", None, None))
        native = (path.parent / aborted["artifact"]).read_text(encoding="utf-8")
        self.assertIn("test stops_before_return ...", native)
        self.assertNotIn("test decoy ", native)
        self.assertFalse(any(row["event"] == "case_finished" for row in rows))
        self.assertEqual({key: rows[-2][key] for key in ("started", "completed", "skipped", "aborted")},
                         {"started": 1, "completed": 0, "skipped": 0, "aborted": 1})
        self.assertEqual({key: rows[-1][key] for key in ("phase", "exit_code", "signal", "evidence")},
                         {"phase": "test execution", "exit_code": 2, "signal": None, "evidence": "incomplete"})

    def test_unittest_interrupt_preserves_completed_active_and_pending_cases(self):
        (self.root / "test_interrupt.py").write_text(dedent("""
            import unittest
            class Cases(unittest.TestCase):
                def test_first(self):
                    pass
                def test_interrupted(self):
                    print("entered interrupted case")
                    raise KeyboardInterrupt
                def test_pending(self):
                    self.fail("entered pending case")
            """), encoding="utf-8")
        command = [sys.executable, str(TOOLS / "run_unittest.py"), "test_interrupt"]
        process = subprocess.run(command, cwd=self.root, env=self.env, capture_output=True,
                                 encoding="utf-8", errors="replace", timeout=10)
        self.assertEqual(process.returncode, 130, process.stderr)
        [location] = [line.removeprefix("[REPORT] ") for line in process.stdout.splitlines()
                      if line.startswith("[REPORT] ")]
        path = Path(location)
        rows = [json.loads(line) for line in path.with_suffix(".jsonl").read_text(
            encoding="utf-8").splitlines()]
        [completed] = [row for row in rows if row["event"] == "case_finished"]
        [aborted] = [row for row in rows if row["event"] == "case_unfinished"]
        self.assertEqual((completed["id"], completed["outcome"]), ("test_interrupt.Cases.test_first", "passed"))
        self.assertEqual((aborted["id"], aborted["outcome"], aborted["seconds"]),
                         ("test_interrupt.Cases.test_interrupted", "aborted", None))
        self.assertIn("entered interrupted case", (path.parent / aborted["artifact"]).read_text(encoding="utf-8"))
        self.assertNotIn("entered pending case", process.stdout + process.stderr)
        counts = next(row for row in rows if row["event"] == "counts")
        self.assertEqual({key: counts[key] for key in ("started", "completed", "aborted", "never_started")},
                         {"started": 2, "completed": 1, "aborted": 1, "never_started": 1})
        self.assertEqual([row["event"] for row in rows[-2:]], ["interrupted", "finished"])


if __name__ == "__main__":
    unittest.main()

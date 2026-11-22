"""Check the verification runner's independent-case execution contract."""

import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
from textwrap import dedent
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run_tests
from test_case_report import TASK_CONTEXT, init_repository


ARCHIVE = Path(".agent/task-archives/task/execution/test-runs")


class WorkerCapacityTests(unittest.TestCase):
    def test_default_reserves_twenty_percent_rounded_up_and_at_least_two(self):
        for logical, reserve, workers in ((1, 2, 1), (2, 2, 1), (4, 2, 2), (11, 3, 8), (20, 4, 16), (21, 5, 16)):
            with self.subTest(logical=logical), \
                 patch.object(os, "process_cpu_count", return_value=logical, create=True), \
                 patch.object(run_tests, "process_affinity_count", return_value=None):
                self.assertEqual(run_tests.worker_capacity(), (logical, reserve, workers))

    def test_unknown_process_cpu_count_falls_back_to_host_then_one(self):
        with patch.object(os, "process_cpu_count", return_value=None, create=True), \
             patch.object(run_tests, "process_affinity_count", return_value=None):
            with patch.object(os, "cpu_count", return_value=8):
                self.assertEqual(run_tests.worker_capacity(), (8, 2, 6))
            with patch.object(os, "cpu_count", return_value=None):
                self.assertEqual(run_tests.worker_capacity(), (1, 2, 1))

    @unittest.skipUnless(os.name == "nt" or hasattr(os, "sched_setaffinity"), "Process affinity is unavailable")
    def test_capacity_respects_the_process_affinity(self):
        source = dedent(f"""
            import json
            import os
            import sys
            sys.path.insert(0, {str(run_tests.RUNNER.parent)!r})
            import run_tests

            if os.name == "nt":
                import psutil
                process = psutil.Process()
                process.cpu_affinity(process.cpu_affinity()[:2])
                count = len(process.cpu_affinity())
            else:
                os.sched_setaffinity(0, sorted(os.sched_getaffinity(0))[:2])
                count = len(os.sched_getaffinity(0))
            print(json.dumps([count, run_tests.worker_capacity()]))
            """)
        output = subprocess.check_output([sys.executable, "-B", "-c", source], text=True, timeout=10)
        count, capacity = json.loads(output)
        self.assertIn(count, (1, 2))
        self.assertEqual(capacity, [count, 2, 1])


class RunnerTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        init_repository(self.root)
        self.suite = self.root / "tests"
        self.suite.mkdir()
        self.report = self.root / "report.json"
        self.module("fixture_support", """
            from pathlib import Path
            import time

            SHARED = Path(__file__).resolve().parent

            def wait_for_marker(name):
                path = SHARED / name
                deadline = time.monotonic() + 10
                while not path.is_file() and time.monotonic() < deadline:
                    time.sleep(0.02)
                if not path.is_file():
                    raise AssertionError(f"missing concurrent marker: {name}")
            """)

    def module(self, name, source):
        (self.suite / f"{name}.py").write_text(dedent(source), encoding="utf-8")

    def run_tests(self, *options, code=0, invocation=None, environment=None, cwd=None, assigned=False):
        command = invocation or [str(run_tests.RUNNER)]
        arguments = ["-s", str(self.suite), "--report", str(self.report), "-q"]
        self.report.unlink(missing_ok=True)
        process = subprocess.run([sys.executable, "-B", *command, *arguments, *options], cwd=cwd or self.root,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                                 encoding="utf-8", errors="replace", timeout=40,
                                 env=self.environment(environment, assigned=assigned))
        self.assertEqual(process.returncode, code, process.stdout)
        self.assertTrue(self.report.is_file(), process.stdout)
        result = json.loads(self.report.read_text(encoding="utf-8"))
        self.assertEqual(result["exit_code"], code)
        return result, process.stdout

    def environment(self, extra=None, *, assigned=False):
        environment = {key: value for key, value in (os.environ if extra is None else extra).items()
                       if key not in run_tests.CONTEXT.values()}
        if assigned:
            environment.update(TASK_CONTEXT)
        return environment

    def archive_report(self, output):
        [line] = [line for line in output.splitlines() if line.startswith("[REPORT] ")]
        markdown = Path(line.removeprefix("[REPORT] "))
        self.assertEqual(markdown.parent, self.root / ARCHIVE)
        events = [json.loads(line) for line in markdown.with_suffix(".jsonl").read_text(
            encoding="utf-8").splitlines()]
        return markdown, events

    def test_unassigned_launcher_runs_outside_git_with_native_outcomes_and_no_case_report(self):
        self.module("test_unassigned", """
            import unittest

            class Cases(unittest.TestCase):
                def test_pass(self):
                    self.assertEqual(17, 17)

                def test_failure(self):
                    self.assertEqual(17, 23)
            """)
        launch = dedent(f"""
            from pathlib import Path
            import runpy
            import sys

            def audit(event, args):
                if event != "subprocess.Popen":
                    return
                executable, command, *_ = args
                if executable is None:
                    executable = command.split(maxsplit=1)[0]
                if Path(executable).stem.lower() == "git":
                    raise AssertionError("Unassigned launcher accessed Git")

            sys.addaudithook(audit)
            sys.path.insert(0, {str(run_tests.RUNNER.parent)!r})
            runpy.run_path({str(run_tests.RUNNER)!r}, run_name="__main__")
            """)
        with tempfile.TemporaryDirectory() as outside:
            result, output = self.run_tests(code=1, invocation=["-c", launch], cwd=outside)
            self.assertFalse((Path(outside) / ".agent").exists())
        self.assertEqual({row["id"].rsplit(".", 1)[-1]: row["outcome"] for row in result["cases"]},
                         {"test_pass": "passed", "test_failure": "failure"})
        self.assertIn("AssertionError: 17 != 23", output)
        self.assertNotIn("[REPORT]", output)
        self.assertNotIn("evidence is incomplete", output)
        self.assertFalse((self.root / ".agent").exists())

    def test_assigned_launcher_outside_git_uses_its_project_checkout_for_reporting(self):
        self.module("test_assigned", """
            import unittest

            class Cases(unittest.TestCase):
                def test_pass(self):
                    self.assertEqual(17, 17)
            """)
        launch = dedent(f"""
            from pathlib import Path
            import sys
            sys.path.insert(0, {str(run_tests.RUNNER.parent)!r})
            import run_tests

            run_tests.PROJECT_ROOT = Path({str(self.root)!r})
            raise SystemExit(run_tests.main())
            """)
        with tempfile.TemporaryDirectory() as outside:
            _, output = self.run_tests(invocation=["-c", launch], cwd=outside, assigned=True)
            self.assertFalse((Path(outside) / ".agent").exists())
        _, rows = self.archive_report(output)
        [case] = [row for row in rows if row["event"] == "case_finished"]
        self.assertEqual(case["id"], "test_assigned.Cases.test_pass")
        self.assertEqual(rows[-1]["exit_code"], 0)

    def test_exact_discovery_and_nested_subtests_report_original_timings(self):
        self.module("test_native", r"""
            import json
            import time
            import unittest
            from fixture_support import SHARED

            class Cases(unittest.TestCase):
                def test_selected(self):
                    with (SHARED / "entered").open("a") as ledger:
                        ledger.write("selected\n")
                    with self.subTest(outer=True):
                        for param, delay in (("fast", 0.01), ("slow", 0.07)):
                            with self.subTest(param=param):
                                started = time.perf_counter()
                                time.sleep(delay)
                                print(f"native marker {param}", flush=True)
                                with (SHARED / "intervals").open("a") as ledger:
                                    ledger.write(json.dumps([param, time.perf_counter() - started]) + "\n")
                                self.assertEqual(17, 23 if param == "slow" else 17)

                def test_decoy(self):
                    with (SHARED / "entered").open("a") as ledger:
                        ledger.write("decoy\n")
            """)
        name = "test_native.Cases.test_selected"
        aggregate, output = self.run_tests(name, code=1, assigned=True)
        self.assertEqual((self.suite / "entered").read_text().splitlines(), ["selected"])
        intervals = dict(json.loads(line) for line in (self.suite / "intervals").read_text().splitlines())
        markdown, rows = self.archive_report(output)
        [case] = [row for row in rows if row["event"] == "case_finished"]
        self.assertEqual(case["id"], name)
        self.assertEqual(case["seconds"], aggregate["cases"][0]["seconds"])
        subtests = {row["id"]: row for row in case["subtests"]}
        log = (markdown.parent / case["artifact"]).read_bytes()
        for param, observed in intervals.items():
            subtest = subtests[f"{name} (param={param!r}, outer=True)"]
            self.assertGreaterEqual(subtest["seconds"], observed)
            self.assertLessEqual(subtest["seconds"], case["seconds"])
            content = log[subtest["output_start"]:subtest["output_end"]]
            self.assertIn(f"native marker {param}".encode(), content)
            self.assertNotIn(b"native marker slow" if param == "fast" else b"native marker fast", content)
        failure = subtests[f"{name} (param='slow', outer=True)"]
        self.assertEqual(failure["outcome"], "failure")
        self.assertIn("AssertionError: 17 != 23", failure["detail"])
        self.assertIn("test_native.py", failure["detail"])

    def test_file_and_case_filters_select_one_case_and_reject_an_empty_run(self):
        (self.root / "local_fixture.py").write_text("EXPECTED = 42\n", encoding="utf-8")
        self.module("test_other", "raise RuntimeError('file filter ignored')")
        self.module("test_choose", """
            import unittest
            from local_fixture import EXPECTED

            class Cases(unittest.TestCase):
                def test_choose_me(self):
                    self.assertEqual(EXPECTED, 42)

                def test_leave_out(self):
                    self.fail("case filter ignored")
            """)
        result, _ = self.run_tests("-p", "test_choose.py", "-k", "choose_me", "--workers", "4")
        self.assertEqual(result["selected"], ["test_choose.Cases.test_choose_me"])
        self.assertEqual([case["id"] for case in result["cases"]], result["selected"])
        self.assertEqual(result["workers"], 1)
        self.assertGreaterEqual(result["cases"][0]["seconds"], 0)

        result, output = self.run_tests("-p", "test_choose.py", "-k", "unknown_test", code=5)
        self.assertEqual(result["selected"], [])
        self.assertIn("NO TESTS RAN", output)

    def test_same_class_cases_overlap_without_sharing_writable_state(self):
        self.assert_case_isolation(workers=2)

    def test_one_worker_still_starts_each_case_with_fresh_state(self):
        self.assert_case_isolation(workers=1)

    def assert_case_isolation(self, workers):
        self.module("test_isolation", f"""
            import json
            import os
            from pathlib import Path
            import tempfile
            import unittest

            from fixture_support import SHARED, wait_for_marker

            class Cases(unittest.TestCase):
                visits = []

                def check_private_state(self, own, peer):
                    self.assertEqual(Cases.visits, [])
                    Cases.visits.append(own)
                    self.assertNotIn("VERIFICATION_TEST_CASE_PROBE", os.environ)
                    os.environ["VERIFICATION_TEST_CASE_PROBE"] = own
                    self.assertNotIn("GIT_INDEX_FILE", os.environ)
                    cwd_file = Path("same-name.txt")
                    temp_file = Path(tempfile.gettempdir()) / "same-temp-name.txt"
                    for target in (cwd_file, temp_file):
                        with target.open("x", encoding="utf-8") as stream:
                            stream.write(own)
                    pending = SHARED / (own + ".tmp")
                    pending.write_text(json.dumps([os.getpid(), str(Path.cwd()), str(temp_file.parent)]),
                                       encoding="utf-8")
                    pending.replace(SHARED / own)
                    if {workers == 2!r}:
                        wait_for_marker(peer)
                    for target in (cwd_file, temp_file):
                        self.assertEqual(target.read_text(encoding="utf-8"), own)
                    self.assertEqual(os.environ["VERIFICATION_TEST_CASE_PROBE"], own)

                def test_first(self):
                    self.check_private_state("first", "second")

                def test_second(self):
                    self.check_private_state("second", "first")
            """)
        environment = {**os.environ, "GIT_INDEX_FILE": str(self.root / "foreign-index")}
        result, _ = self.run_tests("--workers", str(workers), environment=environment)
        observations = [json.loads((self.suite / name).read_text(encoding="utf-8")) for name in ("first", "second")]
        for first, second in zip(*observations, strict=True):
            self.assertNotEqual(first, second)
        for _, cwd, temp in observations:
            self.assertNotIn(Path(cwd), (self.root, self.suite))
            self.assertNotIn(Path(temp), (self.root, self.suite))
        self.assertFalse((self.root / "same-name.txt").exists())
        self.assertEqual(result["workers"], workers)
        self.assertEqual([case["id"] for case in result["cases"]], [
            "test_isolation.Cases.test_first", "test_isolation.Cases.test_second",
        ])

    def test_method_and_class_serial_markers_exclude_other_cases(self):
        self.module("test_exclusive", """
            import unittest

            from fixture_support import SHARED, wait_for_marker

            class Cases(unittest.TestCase):
                def check_parallel(self, own, peer):
                    (SHARED / own).touch()
                    wait_for_marker(peer)
                    (SHARED / (own + "-done")).touch()

                def test_parallel_first(self):
                    self.check_parallel("first", "second")

                def test_parallel_second(self):
                    self.check_parallel("second", "first")

                def test_serial_method(self):
                    self.assertTrue((SHARED / "first-done").is_file())
                    self.assertTrue((SHARED / "second-done").is_file())
                    (SHARED / "serial-method-done").touch()

                test_serial_method.serial_reason = "shared fixture"

            class Exclusive(unittest.TestCase):
                serial_reason = "shared resource"

                def test_serial_class(self):
                    self.assertTrue((SHARED / "serial-method-done").is_file())
            """)
        serial_reasons = {
            "test_exclusive.Cases.test_serial_method": "shared fixture",
            "test_exclusive.Exclusive.test_serial_class": "shared resource",
        }
        check_process_lifetimes = dedent(f"""
            import sys
            sys.path.insert(0, {str(run_tests.RUNNER.parent)!r})
            import run_tests

            serial = {tuple(serial_reasons)!r}
            started = []
            start = run_tests.Work.start

            def check_exclusive(work, processes, number, case=None):
                if case is not None:
                    for previous, process in started:
                        if (case.id in serial or previous in serial) and process.poll() is None:
                            raise AssertionError(f"{{case.id}} started while {{previous}} was still live")
                process = start(work, processes, number, case)
                if case is not None:
                    started.append((case.id, process))
                return process

            run_tests.Work.start = check_exclusive
            raise SystemExit(run_tests.main())
            """)
        result, _ = self.run_tests("--workers", "4", invocation=["-c", check_process_lifetimes])
        self.assertEqual(result["serial"], serial_reasons)
        self.assertEqual(result["parallel_count"], 2)
        self.assertEqual(result["serial_count"], 2)
        self.assertEqual([case["id"] for case in result["cases"]], result["selected"])

        self.module("test_invalid_serial", """
            import unittest

            from fixture_support import SHARED

            class InvalidClass(unittest.TestCase):
                serial_reason = False

                def test_invalid(self):
                    (SHARED / "invalid-class-ran").touch()

            class InvalidMethod(unittest.TestCase):
                serial_reason = "class resource"

                def test_invalid(self):
                    (SHARED / "invalid-method-ran").touch()

                test_invalid.serial_reason = ""
            """)
        invalid, _ = self.run_tests("-p", "test_invalid_serial.py", code=1)
        self.assertEqual(invalid["selected"], [])
        self.assertEqual(invalid["cases"], [])
        self.assertCountEqual(invalid["errors"], [
            f"test_invalid_serial.{name}.test_invalid needs a non-empty serial reason."
            for name in ("InvalidClass", "InvalidMethod")
        ])
        for marker in ("invalid-class-ran", "invalid-method-ran"):
            self.assertFalse((self.suite / marker).exists())

    def test_failed_and_incomplete_cases_cannot_be_hidden_by_successful_output(self):
        self.module("test_outcomes", f"""
            import os
            import subprocess
            import sys
            import unittest

            from fixture_support import wait_for_marker

            class Cases(unittest.TestCase):
                def test_error(self):
                    raise RuntimeError("distinct exception")

                def test_failure(self):
                    with self.subTest(example=2):
                        print("local case output")
                        self.fail("distinct assertion")

                def test_exit(self):
                    subprocess.Popen([sys.executable, "-B", "-c", {self.child_script()!r}],
                                     start_new_session=sys.platform == "linux")
                    wait_for_marker("ready")
                    print("OK", flush=True)
                    os._exit(0)

                def test_pass(self):
                    self.assertEqual(2 + 3, 5)

                @unittest.skip("distinct reason")
                def test_skip(self):
                    self.fail("skipped body executed")
            """)
        result, output = self.run_tests("--workers", "2", code=1)
        self.assert_child_exited()
        prefix = "test_outcomes.Cases.test_"
        outcomes = [(row["id"], row["outcome"]) for row in result["cases"]]
        self.assertCountEqual(outcomes, [(prefix + name, outcome) for name, outcome in (
            ("error", "error"), ("failure", "failure"), ("pass", "passed"), ("skip", "skipped"),
        )])
        self.assertEqual(result["missing"], [prefix + "exit"])
        self.assertTrue(any(prefix + "exit" in error for error in result["errors"]), result)
        self.assertCountEqual([(row["id"], row["kind"]) for row in result["diagnostics"]], [
            (prefix + "error", "error"), (prefix + "failure", "failure"), (prefix + "skip", "skipped"),
        ])
        for detail in ("distinct assertion", "example=2", "distinct exception", "local case output", "\nOK\n"):
            self.assertIn(detail, output)

        (self.suite / "ready").unlink()
        incomplete, output = self.run_tests("-k", "test_exit", code=1)
        self.assert_child_exited()
        self.assertEqual(incomplete["missing"], [prefix + "exit"])
        self.assertEqual(incomplete["cases"], [])
        self.assertIn("\nOK\n", output)

        skipped, _ = self.run_tests("-k", "test_skip")
        self.assertEqual([row["outcome"] for row in skipped["cases"]], ["skipped"])
        self.assertEqual(skipped["missing"], [])

    def test_discovery_error_fails_the_run_while_a_valid_case_is_reported(self):
        self.module("test_broken", "raise RuntimeError('import failed visibly')")
        self.module("test_valid", """
            import unittest

            class Cases(unittest.TestCase):
                def test_ok(self):
                    self.assertEqual(2 + 3, 5)
            """)
        result, output = self.run_tests("--workers", "2", code=1)
        self.assertEqual([row["id"] for row in result["cases"]], ["test_valid.Cases.test_ok"])
        self.assertEqual(result["missing"], [])
        self.assertTrue(any("import failed visibly" in error for error in result["errors"]), result)
        self.assertIn("import failed visibly", output)

    def test_discovery_cannot_succeed_without_a_manifest(self):
        self.module("test_discovery_exit", "import os\nos._exit(0)")
        result, _ = self.run_tests(code=1)
        self.assertEqual(result["selected"], [])
        self.assertEqual(result["cases"], [])
        self.assertTrue(result["errors"])

    def test_worker_interrupt_cancels_its_running_peer_and_descendants(self):
        self.module("test_interrupt", f"""
            import subprocess
            import sys
            import unittest

            from fixture_support import wait_for_marker

            class Cases(unittest.TestCase):
                def test_interrupt(self):
                    wait_for_marker("ready")
                    raise KeyboardInterrupt

                def test_running_peer(self):
                    subprocess.run([sys.executable, "-B", "-c", {self.child_script()!r}], check=True,
                                   start_new_session=sys.platform == "linux")
            """)
        result, output = self.run_tests("--workers", "2", code=130)
        self.assert_child_exited()
        self.assertCountEqual(result["missing"], [
            "test_interrupt.Cases.test_interrupt", "test_interrupt.Cases.test_running_peer",
        ])
        self.assertEqual(result["cases"], [])
        self.assertIn("INTERRUPTED", output)

    def test_parent_interrupt_retains_completed_failure_and_stops_running_case(self):
        self.module("test_interrupted", f"""
            import subprocess
            import sys
            import unittest

            class Cases(unittest.TestCase):
                def test_a_finished(self):
                    self.fail("completed case failure")

                def test_b_running(self):
                    subprocess.run([sys.executable, "-B", "-c", {self.child_script()!r}], check=True,
                                   start_new_session=sys.platform == "linux")

                def test_c_queued(self):
                    self.fail("queued case ran after interruption")
            """)
        interrupt = dedent(f"""
            import _thread
            from pathlib import Path
            import sys
            from threading import Thread
            import time
            sys.path.insert(0, {str(run_tests.RUNNER.parent)!r})
            import run_tests

            def interrupt_when_child_is_running():
                deadline = time.monotonic() + 12
                while time.monotonic() < deadline:
                    if Path({str(self.suite / "ready")!r}).is_file():
                        _thread.interrupt_main()
                        return
                    time.sleep(0.02)

            Thread(target=interrupt_when_child_is_running, daemon=True).start()
            raise SystemExit(run_tests.main())
            """)
        result, output = self.run_tests("--workers", "1", code=130, invocation=["-c", interrupt], assigned=True)
        self.assert_child_exited()
        self.assertEqual([row["id"] for row in result["cases"]], ["test_interrupted.Cases.test_a_finished"])
        self.assertCountEqual(result["missing"], [
            "test_interrupted.Cases.test_b_running", "test_interrupted.Cases.test_c_queued",
        ])
        self.assertIn("completed case failure", output)
        self.assertNotIn("queued case ran after interruption", output)
        _, rows = self.archive_report(output)
        self.assertEqual({row["id"].rsplit(".", 1)[-1]: row["outcome"]
                          for row in rows if row["event"] == "case_unfinished"},
                         {"test_b_running": "aborted", "test_c_queued": "never started"})
        self.assertEqual(rows[-1]["exit_code"], 130)
        self.assertEqual([row["id"].rsplit(".", 1)[-1] for row in rows if row["event"] == "case_finished"],
                         ["test_a_finished"])

    @unittest.skipUnless(os.name == "nt" or sys.platform == "linux", "Worker ownership requires a job or supervisor")
    def test_uncatchable_parent_exit_keeps_completed_running_and_pending_evidence(self):
        self.module("test_kill", """
            import time
            import unittest

            from fixture_support import SHARED, wait_for_marker

            class Cases(unittest.TestCase):
                def test_a_finished(self):
                    print("diagnostic from the finished case")
                    self.assertEqual(17, 23)

                def test_b_running(self):
                    print("native work began", flush=True)
                    (SHARED / "running").touch()
                    wait_for_marker("release")

                def test_c_pending(self):
                    (SHARED / "unexpected").touch()
            """)
        command = [sys.executable, "-B", str(run_tests.RUNNER), "-s", str(self.suite), "--workers", "1"]
        with subprocess.Popen(command, cwd=self.root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                              encoding="utf-8", errors="replace", env=self.environment(assigned=True)) as process:
            try:
                deadline = time.monotonic() + 12
                while not (self.suite / "running").is_file() and process.poll() is None:
                    self.assertLess(time.monotonic(), deadline, "The running case did not start")
                    time.sleep(0.02)
                [report] = (self.root / ARCHIVE).glob("*.md")
                before = report.read_bytes()
                self.assertIn(b"Case finished", before)
                self.assertIn(b"AssertionError: 17 != 23", before)
                self.assertIsNone(process.poll())
            finally:
                process.kill()
            output, _ = process.communicate(timeout=4)
        self.assertNotEqual(process.returncode, 0)
        recovered, rows = self.archive_report(output)
        self.assertEqual(recovered, report)
        self.assertEqual(recovered.read_bytes(), before)
        self.assertFalse((self.suite / "unexpected").exists())
        self.assertFalse(any(row["event"] == "finished" for row in rows))
        selected = {row["id"].rsplit(".", 1)[-1]: report.parent / row["artifact"]
                    for row in rows if row["event"] == "case_selected"}
        first = selected["test_a_finished"]
        active = selected["test_b_running"]
        pending = selected["test_c_pending"]
        self.assertTrue((first / "result.json").is_file())
        self.assertTrue((active / "started.json").is_file())
        self.assertFalse((active / "result.json").exists())
        self.assertIn("native work began", (active / "worker.log").read_text())
        self.assertFalse((pending / "started.json").exists())

    def child_script(self):
        self.addCleanup(self.stop_surviving_child)
        return dedent(f"""
            import json
            import os
            from pathlib import Path
            import sys
            import time
            sys.path.insert(0, {str(Path(__file__).resolve().parent)!r})
            from test_parallel_runner import process_identity

            ready = Path({str(self.suite / "ready")!r})
            pending = ready.with_suffix(".tmp")
            pending.write_text(json.dumps([os.getpid(), process_identity(os.getpid())]), encoding="utf-8")
            pending.replace(ready)
            time.sleep(30)
            """)

    def stop_surviving_child(self):
        ready = self.suite / "ready"
        if ready.is_file():
            pid, identity = json.loads(ready.read_text(encoding="utf-8"))
            if process_identity(pid) == identity:
                try:
                    os.kill(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass

    def assert_child_exited(self):
        pid, identity = json.loads((self.suite / "ready").read_text(encoding="utf-8"))
        deadline = time.monotonic() + 4
        while process_identity(pid) == identity and time.monotonic() < deadline:
            time.sleep(0.02)
        self.assertNotEqual(process_identity(pid), identity, f"case descendant {pid} still exists")


def process_identity(pid):
    if sys.platform == "linux":
        try:
            stat = Path(f"/proc/{pid}/stat").read_text(encoding="utf-8")
        except FileNotFoundError:
            return None
        # Field 22 is the process start time, after the parenthesized command name.
        return stat.rsplit(")", 1)[1].split()[19]
    import psutil

    try:
        return psutil.Process(pid).create_time()
    except psutil.NoSuchProcess:
        return None


if __name__ == "__main__":
    unittest.main()

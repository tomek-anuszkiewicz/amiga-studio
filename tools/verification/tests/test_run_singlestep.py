"""Launcher ownership and Rust completion contracts on tiny process fixtures."""
import argparse
import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import run_singlestep


class LauncherTests(unittest.TestCase):
    def options(self):
        return argparse.Namespace(full=True, sample=None, case=None, exact=False, profile="verification")

    def run_fixture(self, directory, test_code, report_code, status, completed=2):
        events = []
        def call(command, **kwargs):
            events.append(command[0])
            metadata = json.loads((directory / "run.json").read_text())
            metadata.update(status=status, selected_tests=2, completed_tests=completed)
            (directory / "run.json").write_text(json.dumps(metadata))
            if status != "running":
                timing = dict(run_id=metadata["id"], execution_seconds=0.1, finalization_seconds=0.2,
                              pipeline_seconds=0.3, report_exit_code=report_code)
                (directory / "timing.json").write_text(json.dumps(timing))
            self.assertEqual(kwargs["env"]["SINGLESTEP_CASE"], "")
            self.assertEqual(kwargs["env"]["SINGLESTEP_EXACT"], "0")
            self.assertEqual(command, ["silicon", "--quiet", "--nocapture"])
            return test_code
        with patch.object(run_singlestep.subprocess, "call", side_effect=call), \
             contextlib.redirect_stdout(io.StringIO()):
            code = run_singlestep.execute_tests("silicon", self.options(), directory)
        self.assertEqual(events, ["silicon"])
        return code, json.loads((directory / "run.json").read_text())

    def test_failure_preserves_test_exit_code_without_launching_a_reporter(self):
        with tempfile.TemporaryDirectory() as temporary:
            code, metadata = self.run_fixture(Path(temporary), 101, 1, "failed")
            self.assertEqual(code, 101)
            self.assertEqual(metadata["status"], "failed")
            self.assertEqual(metadata["completed_tests"], 2)

    def test_report_failure_cannot_turn_successful_tests_into_success(self):
        with tempfile.TemporaryDirectory() as temporary:
            code, metadata = self.run_fixture(Path(temporary), 0, 1, "failed")
            self.assertEqual(code, 1)
            self.assertEqual(metadata["status"], "failed")

    def test_missing_report_or_unfinished_groups_are_incomplete(self):
        for status, completed in (("running", 0), ("passed", 1)):
            with self.subTest(status=status), tempfile.TemporaryDirectory() as temporary:
                code, metadata = self.run_fixture(Path(temporary), 0, 0, status, completed)
                self.assertEqual(code, 1)
                self.assertEqual(metadata["status"], "incomplete")

    def test_build_failure_never_runs_tests_or_aggregates_stale_results(self):
        with tempfile.TemporaryDirectory() as temporary, \
             patch.dict(run_singlestep.os.environ, {"SINGLESTEP_RESULTS_DIR": temporary}), \
             patch.object(run_singlestep, "build_artifact", return_value=(9, None)), \
             patch.object(run_singlestep, "execute_tests") as execute:
            self.assertEqual(run_singlestep.main([]), 9)
            execute.assert_not_called()
            self.assertFalse((Path(temporary) / "run.lock").exists())

    def test_concurrent_owner_is_rejected_without_releasing_its_lock(self):
        with tempfile.TemporaryDirectory() as temporary, \
             patch.dict(run_singlestep.os.environ, {"SINGLESTEP_RESULTS_DIR": temporary}), \
             patch.object(run_singlestep, "build_artifact") as build, \
             contextlib.redirect_stderr(io.StringIO()):
            lock = Path(temporary) / "run.lock"
            lock.write_text("other process")
            self.assertEqual(run_singlestep.main([]), 1)
            build.assert_not_called()
            self.assertEqual(lock.read_text(), "other process")

    def test_disabled_runtime_checks_cannot_be_selected_as_verification(self):
        for name, kind in (("test_singlestep", "test"), ("cpu", "lib")):
            artifact = dict(reason="compiler-artifact", target=dict(name=name, kind=[kind]),
                            executable="silicon", profile=dict(opt_level="3", debug_assertions=False,
                                                              overflow_checks=False))
            process = unittest.mock.MagicMock()
            process.__enter__.return_value = process
            process.stdout = [json.dumps(artifact) + "\n"]
            process.wait.return_value = 0
            with self.subTest(name=name), patch.object(run_singlestep.subprocess, "Popen", return_value=process), \
                 self.assertRaisesRegex(RuntimeError, "debug assertions and overflow checks"):
                run_singlestep.build_artifact("verification")


if __name__ == "__main__":
    unittest.main()

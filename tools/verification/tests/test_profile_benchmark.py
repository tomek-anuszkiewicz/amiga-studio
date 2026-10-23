"""Profiling uses only the benchmark executable returned by a successful build."""
import contextlib
import io
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import MagicMock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import profile_benchmark


class ProfileBenchmarkTests(unittest.TestCase):
    def build(self, code, artifact=None):
        process = MagicMock()
        process.__enter__.return_value = process
        process.stdout = io.StringIO(json.dumps(artifact) + "\n" if artifact else "")
        process.wait.return_value = code
        return process

    def test_failed_build_does_not_launch_a_stale_executable(self):
        with patch.object(profile_benchmark.subprocess, "Popen", return_value=self.build(1)), \
             patch.object(profile_benchmark.subprocess, "call") as record:
            self.assertEqual(profile_benchmark.main(["--measure-candidates-pilot"]), 1)
            record.assert_not_called()

    def test_successful_build_profiles_its_exact_artifact(self):
        artifact = {"reason": "compiler-artifact", "target": {"name": "bench_instructions", "kind": ["bench"]},
                    "executable": "target/release/deps/bench_instructions-current.exe"}
        with patch.object(profile_benchmark.subprocess, "Popen", return_value=self.build(0, artifact)) as build, \
             patch.object(profile_benchmark.subprocess, "call", return_value=0) as record:
            self.assertEqual(profile_benchmark.main(["--measure-candidates-pilot"]), 0)
            self.assertEqual(build.call_args.args[0][:2], ["cargo", "bench"])
            self.assertIn("--no-run", build.call_args.args[0])
            self.assertEqual(record.call_args.args[0], ["samply", "record", artifact["executable"],
                                                       "--measure-candidates-pilot"])

    def test_missing_benchmark_artifact_does_not_launch_profiler(self):
        artifact = {"reason": "compiler-artifact", "target": {"name": "test_runner", "kind": ["bin"]},
                    "executable": "target/release/test_runner.exe"}
        with patch.object(profile_benchmark.subprocess, "Popen", return_value=self.build(0, artifact)), \
             patch.object(profile_benchmark.subprocess, "call") as record, \
             contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(profile_benchmark.main([]), 1)
            record.assert_not_called()


if __name__ == "__main__":
    unittest.main()

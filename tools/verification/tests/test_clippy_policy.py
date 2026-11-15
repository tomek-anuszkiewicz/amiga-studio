"""Central permissions preserve compiler-resolved Clippy architecture checks."""
import contextlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "checks"))
import check_clippy_policy as policy


class ClippyPolicyContracts(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.source = "crates/gui/src/host_video.rs"
        path = self.root / self.source
        path.parent.mkdir(parents=True)
        path.write_text("use std::sync::Mutex;\n", encoding="utf-8")
        self.entry = {"file": self.source, "lint": "clippy::disallowed_types",
                      "symbol": "std::sync::Mutex", "reason": "Host image handoff"}
        self.key = (self.source, self.entry["lint"], self.entry["symbol"])
        self.write_policy([self.entry])

    def write_policy(self, entries):
        path = self.root / policy.POLICY_FILE
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(entries), encoding="utf-8")

    def diagnostic(self, symbol="std::sync::Mutex", path=None):
        return {"reason": "compiler-message",
                "manifest_path": str(self.root / "crates/gui/Cargo.toml"),
                "message": {"code": {"code": "clippy::disallowed_types"},
                            "message": f"use of a disallowed type `{symbol}`",
                            "level": "warning", "rendered": "",
                            "spans": [{"is_primary": True, "line_start": 1,
                                       "file_name": path or self.source}]}}

    def output(self, *messages, success=True):
        return "\n".join(json.dumps(record) for record in (
            *messages, {"reason": "build-finished", "success": success}))

    def inspect(self, *messages, success=True):
        return policy.inspect_output(self.output(*messages, success=success), self.root,
                                     policy.load_permissions(self.root))

    def test_permission_matches_only_its_exact_source_lint_and_canonical_symbol(self):
        used, failures = self.inspect(self.diagnostic())
        self.assertEqual(used, {self.key})
        self.assertEqual(failures, [])
        used, failures = self.inspect(self.diagnostic("std::sync::Arc"))
        self.assertEqual(used, set())
        self.assertIn("Unapproved std::sync::Arc", failures[0])
        other = self.root / "crates/cpu/src/cpu.rs"
        other.parent.mkdir(parents=True)
        other.write_text("", encoding="utf-8")
        used, failures = self.inspect(self.diagnostic(path="crates/cpu/src/cpu.rs"))
        self.assertEqual(used, set())
        self.assertIn("crates/cpu/src/cpu.rs:1", failures[0])
        message = self.diagnostic()
        message["message"]["code"]["code"] = "clippy::disallowed_methods"
        self.assertTrue(self.inspect(message)[1])

    def test_crate_relative_absolute_and_windows_source_paths_resolve(self):
        paths = ("src/host_video.rs", str(self.root / self.source),
                 self.source.replace("/", "\\"))
        for path in paths:
            with self.subTest(path=path):
                self.assertEqual(self.inspect(self.diagnostic(path=path)), ({self.key}, []))

    def test_unknown_ambiguous_or_external_source_fails(self):
        with self.assertRaises(ValueError):
            self.inspect(self.diagnostic(path="missing.rs"))
        ambiguous = self.root / "src/host_video.rs"
        ambiguous.parent.mkdir()
        ambiguous.write_text("", encoding="utf-8")
        with self.assertRaises(ValueError):
            self.inspect(self.diagnostic(path="src/host_video.rs"))
        with tempfile.NamedTemporaryFile(suffix=".rs") as external:
            with self.assertRaises(ValueError):
                self.inspect(self.diagnostic(path=external.name))

    def test_invalid_json_wording_location_or_completion_cannot_pass(self):
        for output in ("not JSON", "[]", self.output().replace('"success": true', '"success": 1')):
            with self.subTest(output=output), self.assertRaises(ValueError):
                policy.inspect_output(output, self.root, {})
        for change in ("wording", "location"):
            message = self.diagnostic()
            if change == "wording":
                message["message"]["message"] = "Unknown future diagnostic format"
            else:
                message["message"]["spans"] = []
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.inspect(message)
        _, failures = policy.inspect_output("", self.root, {})
        self.assertIn("Missing Cargo build completion record", failures)
        self.assertTrue(self.inspect(success=False)[1])

    def test_other_compiler_errors_keep_their_failure_status(self):
        message = self.diagnostic()
        message["message"].update(code={"code": "E0308"}, level="error",
                                  message="mismatched types", rendered="compiler error\n")
        with contextlib.redirect_stderr(io.StringIO()) as stderr:
            used, failures = self.inspect(message)
        self.assertEqual(used, set())
        self.assertIn("mismatched types", failures)
        self.assertIn("compiler error", stderr.getvalue())

    def test_permissions_require_exact_paths_valid_keys_reasons_and_no_duplicates(self):
        invalid_entries = [None, {}, [self.entry, self.entry],
                           [{**self.entry, "file": "crates/gui/src/*.rs"}],
                           [{**self.entry, "file": "../outside.rs"}],
                           [{**self.entry, "file": "C:/outside.rs"}],
                           [{**self.entry, "reason": ""}],
                           [{**self.entry, "lint": "clippy::unwrap_used"}],
                           [{**self.entry, "extra": "not supported"}]]
        for entries in invalid_entries:
            self.write_policy(entries)
            with self.subTest(entries=entries), self.assertRaises(ValueError):
                policy.load_permissions(self.root)

    def run_main(self, outputs):
        results = [subprocess.CompletedProcess([], status, output, "")
                   for status, output in outputs]
        with patch.object(policy, "REPO_ROOT", self.root), \
                patch.object(policy.subprocess, "run", side_effect=results) as run, \
                contextlib.redirect_stdout(io.StringIO()), \
                contextlib.redirect_stderr(io.StringIO()) as errors:
            result = policy.main()
        return result, errors.getvalue(), run.call_args_list

    def test_native_and_wasm_results_share_one_used_permission_set(self):
        result, errors, calls = self.run_main([
            (0, self.output()), (0, self.output(self.diagnostic()))])
        self.assertEqual(result, 0, errors)
        commands = [call.args[0] for call in calls]
        self.assertIn("--all-targets", commands[0])
        self.assertIn("--lib", commands[1])
        self.assertIn("wasm32-unknown-unknown", commands[1])
        for command in commands:
            self.assertIn("--message-format=json", command)
            self.assertEqual(command.count("--force-warn"), 2)
            self.assertEqual(command[-4:], ["--force-warn", "clippy::disallowed_types",
                                           "--force-warn", "clippy::disallowed_methods"])

    def test_unused_permissions_and_failed_commands_block_the_gate(self):
        result, errors, _ = self.run_main([(0, self.output()), (0, self.output())])
        self.assertEqual(result, 1)
        self.assertIn("Unused Clippy permission", errors)
        result, errors, _ = self.run_main([
            (5, self.output(self.diagnostic())), (0, self.output())])
        self.assertEqual(result, 1)
        self.assertIn("exited with code 5", errors)


if __name__ == "__main__":
    unittest.main()

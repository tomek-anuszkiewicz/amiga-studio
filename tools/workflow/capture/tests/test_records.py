"""Exercise preservation through the capture entry points and real worktrees."""

from hashlib import sha256
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import unittest

from capture_fixtures import case_directory, fixture_git_environment


CAPTURE = Path(__file__).resolve().parents[1]
VERIFICATION = CAPTURE.parents[1] / "verification"
REPORT = "test-runtime-report.md"
TASK = "fixture-task"


class PreservationTests(unittest.TestCase):
    def setUp(self):
        self.root = case_directory(self)
        self.primary = self.root / "primary"
        self.source = self.root / "task"
        self.codex = self.root / "fixture-client"
        fixture = self.root / "fixture.json"
        if fixture.is_file():
            self.base = json.loads(fixture.read_bytes())["base"]

    def git(self, checkout, *command):
        return subprocess.check_output(["git", *command], cwd=checkout, env=fixture_git_environment())

    def write(self, path, data):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return path

    def json_file(self, path, value):
        return self.write(path, json.dumps(value).encode("utf-8"))

    def run_capture(self, script, *args, status=0, env=None):
        result = subprocess.run([sys.executable, str(CAPTURE / script), *map(str, args)],
                                cwd=self.source, env=env, capture_output=True, encoding="utf-8")
        self.assertEqual(result.returncode, status, result.stdout + result.stderr)
        if status in (0, 1):
            self.assertTrue(result.stdout, result.stderr)
            return json.loads(result.stdout)
        return result.stderr

    def timing_inputs(self):
        return json.loads((self.root / "timing-inputs.json").read_bytes())

    def preserve(self, selection, status=0, checkout=None):
        arguments = self.json_file(self.root / "selected-timing.json", selection)
        return self.run_capture("records.py", "preserve-timing", "--primary", self.primary,
                                "--source-checkout", checkout or self.source, "--task-id", TASK,
                                "--input", arguments, status=status)

    def dispose_source(self):
        self.git(self.primary, "worktree", "remove", str(self.source))
        self.assertFalse(self.source.exists())
        self.assertEqual(self.git(self.primary, "ls-files", "--", ".agent/task-archives"), b"")

    def test_timing_receipts_preserve_selected_attempts_and_report_collisions(self):
        first, later, milestone = self.timing_inputs()
        first_report = self.source / first["path"]
        pending = self.source / ".agent/tasks/plan.md"
        self.assertIn("producer digest", self.preserve({k: v for k, v in first.items() if k != "sha256"}, 2))
        self.assertIn("producer digest", self.preserve({**first, "sha256": later["sha256"]}, 2))
        self.assertIn("producer digest", self.preserve({**first, "task_id": "other-task"}, 2))
        missing = {**first, "attempt": "a" * 32, "path": first["path"].replace(first["attempt"], "a" * 32)}
        self.assertIn("Expected a regular file", self.preserve(missing, 2))
        first_receipt = Path(self.preserve(first)["receipt"])
        final_receipt = Path(self.preserve(milestone)["receipt"])
        self.assertEqual(self.preserve(first)["status"], "reused")
        for key, value in (("outcome", "failed"), ("profile", "milestone"), ("candidate", "other candidate")):
            with self.subTest(conflict=key):
                self.assertIn("Conflicting timing archive entry", self.preserve({**first, key: value}, 2))
        self.assertEqual(json.loads(first_receipt.read_bytes())["outcome"], "passed")
        self.assertEqual(first_report.read_bytes(), b"first commit report\n")
        self.assertEqual(pending.read_bytes(), b"unique input\n")
        self.assertFalse((first_receipt.parent.parent / later["attempt"]).exists())
        self.assertFalse((self.primary / ".agent/task-archives/other-task").exists())
        # This fixture disposes out-of-scope inputs only after observing they were retained.
        self.dispose_source()
        self.assertEqual((first_receipt.parent / REPORT).read_bytes(), b"first commit report\n")
        self.assertEqual((final_receipt.parent / REPORT).read_bytes(), b"milestone report\n")
        self.assertFalse((first_receipt.parent.parent.parent / "inputs").exists())

    def test_current_verifier_reports_round_trip_for_commit_and_milestone(self):
        script = """
import sys
import time
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, sys.argv[1])
import execution
import verify

profile, child_status = sys.argv[2:]
command = [sys.executable, "-c", f"raise SystemExit({int(child_status)})"]
phases = [("Fixture", [("Controlled child", command, {})])]
with (patch.object(verify, "REPO_ROOT", Path.cwd()),
      patch.object(execution, "REPO_ROOT", Path.cwd()),
      patch.object(verify, "select_phases", return_value=phases)):
    sys.exit(verify.select_and_run([profile], time.perf_counter()))
"""
        for profile, code, outcome, child_result in (("commit", 0, "passed", "executed"),
                                                     ("milestone", 1, "failed", "failed")):
            with self.subTest(profile=profile):
                result = subprocess.run([sys.executable, "-c", script, str(VERIFICATION), profile, str(code)],
                                        cwd=self.source, env=fixture_git_environment(), capture_output=True,
                                        encoding="utf-8")
                self.assertEqual(result.returncode, code, result.stdout + result.stderr)
                report_path, = re.findall(r"^\[TIMING\] Report generation:.*?; file writing: [^;]+; (.+)$",
                                          result.stdout, re.MULTILINE)
                producer_digest, = re.findall(r"^\[TIMING\] Report SHA-256: ([0-9a-f]{64})$",
                                               result.stdout, re.MULTILINE)
                report = Path(report_path)
                self.assertEqual(report.parent.parent, self.source / ".agent/tmp/verification/diagnostics")
                data = report.read_bytes()
                self.assertIn(f"Profile: `{profile}`".encode(), data)
                self.assertIn(f"| {child_result} |".encode(), data)
                self.assertEqual(producer_digest, sha256(data).hexdigest())
                selection = {"task_id": TASK, "profile": profile, "attempt": report.parent.name,
                             "candidate": self.base, "outcome": outcome,
                             "path": report.relative_to(self.source).as_posix(), "sha256": producer_digest}
                saved = self.preserve(selection)
                self.assertEqual(saved["status"], "preserved")
                self.assertEqual(saved["sha256"], producer_digest)
                receipt = Path(saved["receipt"])
                self.assertEqual(receipt, self.primary / ".agent/task-archives" / TASK / ".timing-reports"
                                 / report.parent.name / "receipt.json")
                self.assertEqual(json.loads(receipt.read_bytes()), {
                    **selection, "version": 1, "source_checkout": str(self.source),
                    "git_common_dir": str(self.primary / ".git"), "size": len(data),
                })
                self.assertEqual((receipt.parent / REPORT).read_bytes(), data)

    def test_timing_rejects_unsafe_or_mismatched_sources_without_publishing(self):
        selected = self.timing_inputs()
        report = self.source / selected["path"]
        self.assertIn("does not identify this verification attempt",
                      self.preserve({**selected, "attempt": "8" * 32}, 2))
        for path in ("../" + selected["path"], report.as_posix()):
            with self.subTest(path=path):
                self.assertIn("Unsafe archive path", self.preserve({**selected, "path": path}, 2))
        self.assertIn("differs from the selected Git repository", self.preserve(selected, 2, self.root))
        other_link = self.root / "linked-report.md"
        os.link(report, other_link)
        self.assertIn("Hard-linked input", self.preserve(selected, 2))
        other_link.unlink()
        self.write(self.source / ".gitignore", b".agent/task-archives/\n")
        self.assertIn("must be ignored in its checkout", self.preserve(selected, 2))
        self.assertFalse((self.primary / ".agent/task-archives" / TASK / ".timing-reports").exists())

    def test_primary_checkout_can_supply_an_explicitly_selected_diagnostic(self):
        selected = self.timing_inputs()
        report = self.primary / selected["path"]
        receipt = Path(self.preserve(selected, checkout=self.primary)["receipt"])
        self.assertEqual((receipt.parent / REPORT).read_bytes(), report.read_bytes())
        self.assertEqual(json.loads(receipt.read_bytes())["invocation"], selected["invocation"])

    def test_removed_collection_command_is_rejected_before_creating_archive(self):
        error = self.run_capture("records.py", "collect", "--primary", self.primary,
                                 "--task-id", TASK, "--execution-id", "removed", status=2)
        self.assertIn("invalid choice: 'collect'", error)
        self.assertFalse((self.primary / ".agent/task-archives" / TASK).exists())

    def test_existing_revision_keeps_input_versions_and_reports_after_disposal(self):
        execution = "registered"
        thread = "11111111-1111-4111-8111-111111111111"
        turn = "22222222-2222-4222-8222-222222222222"
        start, end = ("2026-11-18T10:00:00Z", "2026-11-18T10:02:00Z")
        native = self.codex / f"sessions/rollout-fixture-{thread}.jsonl"
        registration = ["--project", self.primary, "--task", TASK, "--execution", execution]
        environment = {**os.environ, "CODEX_THREAD_ID": thread}
        self.run_capture("register.py", "start", *registration, "--codex-home", self.codex,
                         "--at", start, env=environment)
        self.run_capture("register.py", "stop", *registration, "--at", end, "--turn-id", turn, env=environment)

        editable = self.source / ".agent/tasks/nested/plan.md"
        first_source = self.source / ".agent/tmp/task-inputs" / TASK / "start/nested/plan.md"
        args = ["--primary", self.primary, "--task-id", TASK, "--execution-id", execution]
        revision = "20261118T100200000000Z-12345678"
        execution_archive = self.primary / ".agent/task-archives" / TASK / execution
        archive = execution_archive / "revisions" / revision
        manifest = json.loads((archive / "manifest.json").read_bytes())
        observer = self.root / "observer.json"
        checks = json.loads(observer.read_bytes())["rows"]
        expected = {check["path"]: check for row in checks.values()
                    for check in row["checks"] if check["kind"] == "archive"}
        self.assertEqual((archive / "registration.json").read_bytes(),
                         (execution_archive / "registration.json").read_bytes())
        self.assertEqual(first_source.read_bytes(), b"starting\n")
        self.assertFalse(any(item["kind"] == "otel" for item in manifest["files"]))
        self.assertEqual((archive / f"rollouts/{thread}.jsonl").read_bytes(), native.read_bytes())
        self.assertEqual(editable.read_bytes(), b"final\n")
        self.assertEqual({item["path"] for item in manifest["files"] if "/inputs/" in item["path"]},
                         {name for name in expected if "/inputs/" in name})
        approved = archive / "artifacts/worktree/inputs/revision/nested/plan.md"
        self.assertEqual(approved.read_bytes(), b"approved\n")
        result = self.run_capture("records.py", "check", *args, "--revision", revision,
                                  "--expect", observer, status=1)
        self.assertEqual(result["integrity"], "passed")
        self.assertEqual(result["verdict"], "gaps")
        rows = json.loads((Path(result["report"]) / "result.json").read_bytes())["rows"]
        self.assertEqual(len(rows), 8)
        for row in checks:
            self.assertEqual(rows[row]["status"], "present", rows[row])
        missing = "artifacts/worktree/inputs/required-but-missing/nested/plan.md"
        details = json.loads(observer.read_bytes())
        details["rows"]["starting_inputs"]["checks"].append(
            {"kind": "archive", "path": missing, "sha256": sha256(b"missing approved content").hexdigest()})
        observer = self.json_file(self.root / "missing-input.json", details)
        missing_result = self.run_capture("records.py", "check", *args, "--revision", revision,
                                          "--expect", observer, status=1)
        report = json.loads((Path(missing_result["report"]) / "result.json").read_bytes())
        row = report["rows"]["starting_inputs"]
        self.assertEqual(row["status"], "partial")
        self.assertEqual([entry["locator"] for entry in row["checks"] if not entry["verified"]], [missing])
        self.dispose_source()
        for destination, check in expected.items():
            self.assertEqual(sha256((archive / destination).read_bytes()).hexdigest(), check["sha256"])


if __name__ == "__main__":
    unittest.main()

"""Exercise preservation through the capture entry points and real worktrees."""

from hashlib import sha256
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from capture_fixtures import fixture_git_environment, restore_capture


CAPTURE = Path(__file__).resolve().parents[1]
REPORT = "test-runtime-report.md"
TASK = "fixture-task"


class PreservationTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.primary = self.root / "primary"
        self.source = self.root / "task"
        self.codex = self.root / "fixture-client"
        ignore = (CAPTURE.parents[2] / ".gitignore").read_bytes().replace(b"\r\n", b"\n")
        restore_capture("records", self.root)
        if (self.primary / ".gitignore").read_bytes() == ignore:
            self.base = json.loads((self.root / "fixture.json").read_bytes())["base"]
        else:
            # The contract uses the current checkout's ignore rules.
            self.refresh_ignore_fixture(ignore)

    def refresh_ignore_fixture(self, ignore):
        self.write(self.primary / ".gitignore", ignore)
        self.git(self.primary, "add", ".gitignore")
        self.git(self.primary, "commit", "--allow-empty", "-qm", "Refresh the ignore rules")
        self.base = self.git(self.primary, "rev-parse", "HEAD").decode().strip()
        self.git(self.source, "merge", "--ff-only", self.base)

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

    def timing(self, profile, attempt, data, checkout=None):
        checkout = checkout or self.source
        prefix = "diagnostics" if profile == "diagnostic" else f"{profile}/verify"
        relative = Path(f".agent/tmp/verification/{prefix}/{attempt}/{REPORT}")
        report = self.write(checkout / relative, data)
        selected = {"task_id": TASK, "profile": profile, "attempt": attempt,
                    "candidate": self.base, "outcome": "passed", "path": relative.as_posix(),
                    "sha256": sha256(data).hexdigest()}
        if profile == "diagnostic":
            selected["invocation"] = "verify.py check english files AGENTS.md"
        return report, selected

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
        first_report, first = self.timing("commit", "1" * 32, b"first commit report\n")
        _, later = self.timing("commit", "2" * 32, b"later commit report\n")
        _, milestone = self.timing("milestone", "3" * 32, b"milestone report\n")
        # The task retained the first selection while the profile's latest record advanced.
        latest = self.source / ".agent/tmp/verification/commit/commit.json"
        self.json_file(latest, {"attempt": later["attempt"], "timing_report": later})
        pending = self.write(self.source / ".agent/tasks/plan.md", b"unique input\n")
        self.assertIn("producer digest", self.preserve({k: v for k, v in first.items() if k != "sha256"}, 2))
        self.assertIn("producer digest", self.preserve({**first, "sha256": later["sha256"]}, 2))
        self.assertIn("producer digest", self.preserve({**first, "task_id": "other-task"}, 2))
        missing = {**first, "attempt": "a" * 32, "path": first["path"].replace(first["attempt"], "a" * 32)}
        self.assertIn("Expected a regular file", self.preserve(missing, 2))
        first_receipt = Path(self.preserve(first)["receipt"])
        final_receipt = Path(self.preserve(milestone)["receipt"])
        self.assertEqual(self.preserve(first)["status"], "reused")
        self.assertIn("Conflicting timing archive entry", self.preserve({**first, "outcome": "failed"}, 2))
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

    def test_primary_checkout_can_supply_an_explicitly_selected_diagnostic(self):
        report, selected = self.timing("diagnostic", "4" * 32, b"diagnostic report\n", self.primary)
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
        start, middle, end = ("2026-11-18T10:00:00Z", "2026-11-18T10:01:00Z", "2026-11-18T10:02:00Z")
        rollout = [
            {"timestamp": start, "type": "session_meta", "payload": {"id": thread, "cwd": str(self.source)}},
            {"timestamp": middle, "type": "event_msg", "payload": {"type": "task_started", "turn_id": turn}},
            {"timestamp": end, "type": "event_msg", "payload": {"type": "task_complete", "turn_id": turn}},
        ]
        native = self.write(self.codex / f"sessions/rollout-fixture-{thread}.jsonl",
                            b"".join(json.dumps(row).encode("utf-8") + b"\n" for row in rollout))
        registration = ["--project", self.primary, "--task", TASK, "--execution", execution]
        environment = {**os.environ, "CODEX_THREAD_ID": thread}
        self.run_capture("register.py", "start", *registration, "--codex-home", self.codex,
                         "--at", start, env=environment)
        self.run_capture("register.py", "stop", *registration, "--at", end, "--turn-id", turn, env=environment)

        editable = self.source / ".agent/tasks/nested/plan.md"
        snapshots = self.source / ".agent/tmp/task-inputs" / TASK
        files = []
        expected = {}
        versions = []

        def select(source, destination, data):
            value = sha256(data).hexdigest()
            files.append({"kind": "artifact", "source": str(source), "path": destination, "sha256": value})
            expected[destination] = {"kind": "archive", "path": destination, "sha256": value, "size": len(data)}

        self.write(editable, b"unselected initial draft\n")
        for version, data in (("start", b"starting\n"), ("revision", b"approved\n"), ("final", b"final\n")):
            if version == "final":
                self.write(editable, b"unselected working edit after approval\n")
            self.write(editable, data)
            copy = self.write(snapshots / version / "nested/plan.md", editable.read_bytes())
            select(copy, f"artifacts/worktree/inputs/{version}/nested/plan.md", data)
            versions.append({"version": version, "path": editable.relative_to(self.source).as_posix(),
                             "sha256": sha256(data).hexdigest(), "approval": turn})
        version_map = self.json_file(snapshots / "versions.json",
                                     {"task_id": TASK, "versions": versions, "final": "final"})
        select(version_map, "artifacts/worktree/inputs/versions.json", version_map.read_bytes())
        for profile, attempt, data in (("commit", "5" * 32, b"commit duration\n"),
                                       ("milestone", "6" * 32, b"milestone duration\n")):
            report, record = self.timing(profile, attempt, data)
            destination = f"artifacts/verification/{profile}/{attempt}"
            select(report, f"{destination}/{REPORT}", data)
            selected = self.json_file(report.parent / "selection.json", record)
            select(selected, f"{destination}/selection.json", selected.read_bytes())

        args = ["--primary", self.primary, "--task-id", TASK, "--execution-id", execution]
        revision = "20261118T100200000000Z-12345678"
        execution_archive = self.primary / ".agent/task-archives" / TASK / execution
        archive = execution_archive / "revisions" / revision
        inventory = []

        def archived(path, data, kind, **metadata):
            self.write(archive / path, data)
            inventory.append({"path": path, "kind": kind, "size": len(data),
                              "sha256": sha256(data).hexdigest(), **metadata})

        # Describe an existing revision independently of the retired producer.
        registration_data = (execution_archive / "registration.json").read_bytes()
        registered = json.loads(registration_data)
        archived("registration.json", registration_data, "registration")
        native_data = native.read_bytes()
        native_lines = native_data.splitlines(keepends=True)
        archived(f"rollouts/{thread}.jsonl", native_data, "rollout", source=str(native),
                 ranges=[{"start": 0, "end": len(native_data), "first_line": 1,
                          "last_line": len(native_lines)}], whole_source=True)
        for entry in files:
            archived(entry["path"], Path(entry["source"]).read_bytes(), "artifact", source=entry["source"])
        manifest = {
            "format_version": 1, "tool_version": "records.py/1", "revision": revision,
            "previous_revision": None, "task_id": TASK, "execution_id": execution,
            "git_common_dir": registered["git_common_dir"], "root_thread_id": thread,
            "start_utc": registered["start_utc"], "end_utc": registered["end_utc"],
            "start_commit": self.base, "workflow_commit": self.base, "final_commit": self.base,
            "final_candidate": self.base, "outcome": "completed", "collection_status": "collected",
            "threads": [{"thread_id": thread, "parent_thread_id": None, "source": str(native),
                         "selection": registered["threads"][0], "turn_ids": [turn],
                         "segment": f"rollouts/{thread}.jsonl", "cli_version": None,
                         "last_event": {"timestamp": end, "source_line": len(native_lines),
                                        "byte_start": len(native_data) - len(native_lines[-1]),
                                        "byte_end": len(native_data), "type": "event_msg",
                                        "event": "task_complete", "turn_id": turn, "call_id": None}}],
            "files": inventory, "gaps": [],
        }
        self.json_file(archive / "manifest.json", manifest)
        first_source = Path(files[0]["source"])
        self.assertEqual(first_source.read_bytes(), b"starting\n")
        self.assertFalse(any(item["kind"] == "otel" for item in manifest["files"]))
        self.assertEqual((archive / f"rollouts/{thread}.jsonl").read_bytes(), native.read_bytes())
        self.assertEqual(editable.read_bytes(), b"final\n")
        self.assertEqual({item["path"] for item in manifest["files"] if "/inputs/" in item["path"]},
                         {name for name in expected if "/inputs/" in name})
        approved = archive / "artifacts/worktree/inputs/revision/nested/plan.md"
        self.assertEqual(approved.read_bytes(), b"approved\n")
        checks = {
            "starting_inputs": {"checks": [check for path, check in expected.items() if "/inputs/" in path]},
            "registration_lineage": {"checks": [{"kind": "source_range", "path": f"rollouts/{thread}.jsonl",
                                                "source": str(native), "start": 0, "end": native.stat().st_size}]},
            "usage_verification": {"checks": [check for path, check in expected.items() if "/verification/" in path]},
            "integrity_survival": {"checks": [{"kind": "commit_ref", "commit": self.base, "ref": "refs/heads/master"}]},
        }
        observer = self.json_file(self.root / "observer.json", {
            "identity": {"start_commit": self.base, "workflow_commit": self.base, "final_commit": self.base,
                         "final_candidate": self.base, "task_id": TASK, "execution_id": execution},
            "threads": [thread], "rows": checks,
        })
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

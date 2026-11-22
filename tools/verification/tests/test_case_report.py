"""Filesystem and recovery contracts for durable case evidence."""

from concurrent.futures import ThreadPoolExecutor
import contextlib
from datetime import datetime, timezone
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from uuid import UUID

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import case_report


TASK_CONTEXT = {"AMIGA_TEST_TASK_ID": "task", "AMIGA_TEST_EXECUTION_ID": "execution"}


def git_fixture(root, *args, source=None):
    environment = {key: val for key, val in os.environ.items() if not key.upper().startswith("GIT_")}
    environment.update(GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1",
                       GIT_AUTHOR_NAME="Fixture", GIT_AUTHOR_EMAIL="fixture@example.invalid",
                       GIT_COMMITTER_NAME="Fixture", GIT_COMMITTER_EMAIL="fixture@example.invalid")
    return subprocess.check_output(["git", *args], cwd=root, input=source, env=environment,
                                   stderr=subprocess.PIPE).decode().strip()


def init_repository(root):
    git_fixture(root, "init", "--quiet")
    ignore_archive(root)
    tree = git_fixture(root, "mktree", source=b"")
    revision = git_fixture(root, "commit-tree", tree, "-m", "Archive fixture")
    git_fixture(root, "update-ref", "HEAD", revision)
    return revision


def ignore_archive(root):
    exclude = root / ".git/info/exclude"
    exclude.parent.mkdir(parents=True, exist_ok=True)
    with exclude.open("a", encoding="utf-8") as stream:
        stream.write("\n/.agent/task-archives/\n")


def events(report):
    return [json.loads(line) for line in report.journal.read_text(encoding="utf-8").splitlines()]


class CaseReportTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.revision = init_repository(self.root)
        environment = {key: value for key, value in os.environ.items() if key not in case_report.CONTEXT.values()}
        environment.update(TASK_CONTEXT)
        context = patch.dict(os.environ, environment, clear=True)
        context.start()
        self.addCleanup(context.stop)

    def test_unassigned_reports_discard_all_evidence_without_git_or_warnings(self):
        for context in ({}, {"AMIGA_TEST_TASK_ID": "task"}, {"AMIGA_TEST_EXECUTION_ID": "execution"},
                        TASK_CONTEXT | {"AMIGA_TEST_TASK_ID": ""},
                        TASK_CONTEXT | {"AMIGA_TEST_EXECUTION_ID": ""},
                        TASK_CONTEXT | {"AMIGA_TEST_TASK_ID": "standalone"},
                        TASK_CONTEXT | {"AMIGA_TEST_TASK_ID": "Standalone"}):
            with self.subTest(context=context), patch.object(case_report, "git", wraps=case_report.git) as git, \
                 contextlib.redirect_stdout(io.StringIO()) as stdout, \
                 contextlib.redirect_stderr(io.StringIO()) as stderr:
                environment = context | {"AMIGA_TEST_ROUND": "2"}
                report = case_report.CaseReport.reserve("fixture", ["run"], self.root, environment)
                self.assertIsNone(report.attachment("fixture.log"))
                source = self.root / "native.log"
                self.assertIsNone(report.copy(source, "fixture.log"))
                self.assertIsNone(report.link(source))
                self.assertIsNone(report.reference())
                self.assertEqual(report.child_environment(environment), environment)
                self.assertEqual(report.children(), [])
                self.assertEqual(report.merge_children(), {"started": 0, "completed": 0})
                report.phase("execution")
                report.event("case_finished", id="fixture")
                report.record_error("no native case event available")
                report.finish(7)
                git.assert_not_called()
                self.assertFalse(report.incomplete)
                self.assertEqual(stdout.getvalue(), "")
                self.assertEqual(stderr.getvalue(), "")
                self.assertFalse((self.root / ".agent").exists())

    def test_invalid_complete_context_warns_without_writing_an_archive(self):
        for key in TASK_CONTEXT:
            with self.subTest(key=key), patch.object(case_report, "git") as git, \
                 contextlib.redirect_stderr(io.StringIO()) as stderr:
                context = TASK_CONTEXT | {key: "../invalid"}
                report = case_report.CaseReport.reserve("fixture", ["run"], self.root, context)
                git.assert_not_called()
                self.assertIsNone(report.reference())
                self.assertTrue(report.incomplete)
                self.assertIn("Invalid archive identifier", stderr.getvalue())
                self.assertFalse((self.root / ".agent").exists())

    def test_same_millisecond_boundaries_and_child_survive_worktree_removal(self):
        linked = self.root / "linked"
        git_fixture(self.root, "worktree", "add", "--detach", str(linked), self.revision)
        instant = datetime(2026, 11, 19, 14, 3, 27, 184000, timezone.utc)
        with patch.object(case_report, "utc_now", return_value=instant),              ThreadPoolExecutor(max_workers=2) as pool:
            reports = list(pool.map(lambda _: case_report.CaseReport.reserve("fixture", ["run"], linked), range(2)))
        first, second = reports
        self.assertNotEqual(first.path, second.path)
        for report in reports:
            self.assertEqual(report.path.parent,
                             self.root / ".agent/task-archives/task/execution/test-runs")
            self.assertTrue(report.path.name.startswith("2026-11-19T14-03-27.184Z_"))
            self.assertEqual(events(report)[0]["revision"], self.revision)
        child_env = first.child_environment()
        child_env["AMIGA_TEST_ROUND"] = "2"
        with patch.dict(os.environ, child_env, clear=True):
            child = case_report.CaseReport.reserve("child", ["run", "fixture"], linked)
        log = child.attachment("fixture.log")
        log.write_text("native assertion: 17 != 23", encoding="utf-8")
        child.event("case_finished", id="Case.example", seconds=0.125, artifact=child.link(log))
        child.finish(1)
        expected = {path: path.read_bytes() for path in (first.path, second.path, child.path, child.journal, log)}
        self.assertEqual(first.children(), [(child.path, child.metadata | {"event": "invocation"})])
        self.assertEqual(second.children(), [])
        self.assertEqual(linked.parent.resolve(), self.root)
        git_fixture(self.root, "worktree", "remove", str(linked))
        self.assertEqual({path: path.read_bytes() for path in expected}, expected)

    def test_filesystem_collision_preserves_previous_report(self):
        instant = datetime(2026, 11, 19, tzinfo=timezone.utc)
        id1, id2 = UUID(int=1), UUID(int=2)
        with patch.object(case_report, "utc_now", return_value=instant),              patch.object(case_report, "uuid4", side_effect=[id1, id1, id2]):
            first = case_report.CaseReport.reserve("first", ["fixture"], self.root)
            first.finish(0)
            original = first.path.read_bytes()
            second = case_report.CaseReport.reserve("second", ["fixture"], self.root)
        self.assertEqual(first.path.read_bytes(), original)
        self.assertEqual(second.id, id2.hex)

    def test_redirected_archive_parent_cannot_create_directories_outside_checkout(self):
        external = tempfile.TemporaryDirectory()
        self.addCleanup(external.cleanup)
        target = Path(external.name)
        redirect = self.root / ".agent/task-archives"
        redirect.parent.mkdir()
        if os.name == "nt":
            subprocess.run(["cmd", "/d", "/c", "mklink", "/j", str(redirect), str(target)],
                           check=True, capture_output=True)
            self.addCleanup(redirect.rmdir)
        else:
            redirect.symlink_to(target, target_is_directory=True)
        with contextlib.redirect_stderr(io.StringIO()) as stderr:
            report = case_report.CaseReport.reserve("fixture", ["run"], self.root)
        self.assertIsNone(report.path)
        self.assertTrue(report.incomplete)
        self.assertIn("evidence is incomplete", stderr.getvalue())
        self.assertEqual(list(target.iterdir()), [])

    def test_write_error_preserves_the_original_gate_result(self):
        report = case_report.CaseReport.reserve("fixture", ["fixture"], self.root)
        report.event("case_finished", id="completed", seconds=0.25)
        open_file = Path.open

        def obstruct(path, *args, **kwargs):
            if path == report.path and args[:1] == ("a",):
                raise PermissionError("archive fixture denied an append")
            return open_file(path, *args, **kwargs)

        def run(archive):
            archive.event("case_finished", id="recoverable", seconds=0.75)
            return 0

        with patch.object(case_report.CaseReport, "reserve", return_value=report),              patch.object(Path, "open", obstruct), contextlib.redirect_stderr(io.StringIO()) as stderr:
            self.assertEqual(case_report.run_reported("fixture", ["fixture"], run), 0)
        self.assertIn("evidence is incomplete", stderr.getvalue())
        self.assertIn("completed", report.path.read_text(encoding="utf-8"))
        self.assertEqual(events(report)[-2]["id"], "recoverable")
        self.assertEqual(events(report)[-1]["evidence"], "incomplete")
        self.assertEqual(events(report)[-1]["exit_code"], 0)

    def test_failed_reservation_is_visible_and_does_not_replace_native_exit(self):
        archive = self.root / ".agent"
        archive.touch()
        with contextlib.redirect_stderr(io.StringIO()) as stderr:
            self.assertEqual(case_report.run_reported("fixture", ["fixture"], lambda _: 7, self.root), 7)
        self.assertIn("evidence is incomplete", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()

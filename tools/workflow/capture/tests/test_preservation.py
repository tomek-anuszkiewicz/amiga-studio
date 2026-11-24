"""Ordinary whole-file preservation in isolated Git worktrees."""

from argparse import Namespace
from contextlib import redirect_stdout
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

from capture_fixtures import case_directory

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import records
import preservation


class WholeFilePreservationTests(unittest.TestCase):
    def setUp(self):
        self.folder = case_directory(self)
        self.primary = self.folder / "primary"
        self.task = self.folder / "task"
        self.archive = self.primary / ".agent/task-archives/trial/execution"
        self.selection = self.task / ".agent/tmp/selected.json"

    def git(self, *arguments):
        return subprocess.check_output(["git", *arguments], cwd=self.primary)

    def write(self, path, data):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return path

    def item(self, name, source, path, complete=True):
        return {"id": name, "source": str(source), "path": path, "complete": complete}

    def run_selection(self, items=None, primary=None):
        if items is not None:
            self.write(self.selection, json.dumps({"files": items}).encode())
        args = Namespace(primary=primary or self.primary, task_id="trial", execution_id="execution",
                         input=self.selection, publication=None)
        output = io.StringIO()
        with redirect_stdout(output):
            self.assertEqual(records.preserve(args), 0)
        return output.getvalue()

    def test_only_selected_files_are_opened_and_archive_residents_are_reused(self):
        main = self.folder / "main.jsonl"
        task_log = self.task / ".agent/tmp/output.txt"
        resident = self.archive / "sessions/agent.jsonl"
        telemetry = self.archive / "otel/events.jsonl"
        decoy = self.folder / "decoy/sessions/other.jsonl"
        native_open = Path.open
        opened = []

        def observe(path, *args, **kwargs):
            if path != self.selection:
                opened.append(path)
            return native_open(path, *args, **kwargs)

        with (patch.object(Path, "open", observe),
              patch.object(preservation.shutil, "copyfileobj", wraps=preservation.shutil.copyfileobj) as copies,
              patch.object(os, "scandir", side_effect=AssertionError("Unexpected enumeration"))):
            output = self.run_selection()
            self.assertEqual(copies.call_count, 2)
            self.assertEqual(opened, [main, task_log])
            self.assertIn("[RETAINED] worker: sessions/agent.jsonl", output)
            opened.clear()
            destinations = [("main", "sessions/main.jsonl"), ("worker", "sessions/agent.jsonl"),
                            ("log", "outputs/execution.txt")]
            self.run_selection([self.item(name, self.archive / path, path) for name, path in destinations])
            self.assertEqual(copies.call_count, 2)
            self.assertEqual(opened, [])

        self.assertEqual((self.archive / "sessions/main.jsonl").read_bytes(), main.read_bytes())
        self.assertEqual((self.archive / "outputs/execution.txt").read_bytes(), task_log.read_bytes())
        self.assertEqual(resident.read_bytes(), b"already archived\r\n\x80")
        self.assertEqual(telemetry.read_bytes(), b"existing telemetry")
        self.assertEqual(decoy.read_bytes(), b"must remain unobserved")
        self.assertEqual({path.relative_to(self.archive).as_posix() for path in self.archive.rglob("*") if path.is_file()},
                         {"sessions/main.jsonl", "sessions/agent.jsonl", "outputs/execution.txt",
                          "otel/events.jsonl"})
        self.assertFalse(self.git("ls-files", "--", ".agent"))

    def test_copy_never_reads_an_existing_registration(self):
        registration = self.archive / "registration.json"
        log = self.folder / "session.jsonl"
        native_open = Path.open

        def open_without_registration(path, *args, **kwargs):
            if path == registration:
                raise AssertionError("Attempted to open the registration")
            return native_open(path, *args, **kwargs)

        with patch.object(Path, "open", open_without_registration):
            result = self.run_selection()
        self.assertIn("[COPIED] session: sessions/main.jsonl", result)
        self.assertEqual(registration.read_bytes(), b"unreadable registration")
        self.assertEqual((self.archive / "sessions/main.jsonl").read_bytes(), log.read_bytes())

    def test_collisions_keep_distinct_sources_without_reading_occupied_entries(self):
        choices = json.loads(self.selection.read_bytes())["files"]
        opened = []
        native_open = Path.open

        def observe(path, *args, **kwargs):
            if path != self.selection:
                opened.append(path)
            return native_open(path, *args, **kwargs)

        with patch.object(Path, "open", observe):
            result = self.run_selection()
        self.assertCountEqual(opened, [Path(item["source"]) for item in choices])
        destinations = [self.archive / line.partition(": ")[2] for line in result.splitlines()]
        self.assertEqual(len(set(destinations)), 2)
        self.assertNotIn(self.archive / "same.log", destinations)
        self.assertNotIn(self.archive / "same-2.log", destinations)
        self.assertEqual([path.read_bytes() for path in destinations], [b"original", b"second source"])
        self.assertEqual((self.archive / "same.log").read_bytes(), b"original")
        self.assertEqual((self.archive / "same-2.log").read_bytes(), b"occupied fallback")

    def test_failed_copy_keeps_previous_success_and_retry_copies_only_missing_file(self):
        obstruction = self.archive / "blocked"
        second = self.folder / "second.json"
        choices = json.loads(self.selection.read_bytes())["files"]
        with self.assertRaisesRegex(ValueError, "Cannot preserve .*blocked/second.json"):
            self.run_selection()
        self.assertEqual((self.archive / "first.json").read_bytes(), b"complete first")
        self.assertEqual(obstruction.read_bytes(), b"keep obstruction")
        self.assertFalse(list(self.archive.rglob(".pending-*")))
        obstruction.unlink()
        choices[0]["source"] = str(self.archive / "first.json")
        result = self.run_selection(choices)
        self.assertIn("[RETAINED] first", result)
        self.assertIn("[COPIED] second", result)
        self.assertEqual({path.name for path in self.archive.rglob("*.json")},
                         {"first.json", "second.json"})
        self.assertEqual((self.archive / "blocked/second.json").read_bytes(), second.read_bytes())

    def test_actual_live_writers_are_never_read_before_completion(self):
        script = (
            "from pathlib import Path; import sys; "
            "output = Path(sys.argv[1]).open('wb'); output.write(b'partial'); output.flush(); "
            "print('ready', flush=True); sys.stdin.readline(); output.write(b' final tail'); output.close()"
        )
        for index, parent in enumerate((self.folder, self.task / ".agent/tmp", self.archive)):
            with self.subTest(parent=parent):
                source = parent / f"live-{index}.jsonl"
                producer = subprocess.Popen([sys.executable, "-c", script, str(source)], text=True,
                                            stdin=subprocess.PIPE, stdout=subprocess.PIPE)
                try:
                    self.assertEqual(producer.stdout.readline().strip(), "ready")
                    name = f"live-{index}"
                    choice = self.item(name, source, f"selected/{name}.jsonl", complete=False)
                    with patch.object(preservation.shutil, "copyfileobj",
                                      side_effect=AssertionError("Read from an active producer")):
                        if parent == self.archive:
                            self.assertIn(f"[RETAINED] {name}", self.run_selection([choice]))
                        else:
                            with self.assertRaisesRegex(ValueError, "producer is still active"):
                                self.run_selection([choice])
                            self.assertFalse((self.archive / choice["path"]).exists())
                    producer.communicate("finish\n", timeout=10)
                    self.assertEqual(producer.returncode, 0)
                    choice["complete"] = True
                    self.run_selection([choice])
                    target = source if parent == self.archive else self.archive / choice["path"]
                    self.assertEqual(target.read_bytes(), b"partial final tail")
                finally:
                    if producer.poll() is None:
                        producer.kill()
                        producer.communicate(timeout=10)

    def test_selected_file_in_another_execution_is_refused(self):
        source = self.archive.parent / "other/output.txt"
        with self.assertRaisesRegex(ValueError, "belongs to another private execution"):
            self.run_selection()
        self.assertFalse((self.archive / "output.txt").exists())
        self.assertEqual(source.read_bytes(), b"keep other execution")

    def test_wrong_root_unignored_and_force_tracked_archive_are_refused(self):
        with self.assertRaisesRegex(ValueError, "surviving primary checkout"):
            self.run_selection(primary=self.task)
        self.write(self.primary / ".gitignore", b"")
        with self.assertRaisesRegex(ValueError, "must be ignored"):
            self.run_selection()
        self.write(self.primary / ".gitignore", b".agent/\n")
        tracked = self.write(self.archive / "tracked.txt", b"keep staged bytes")
        self.git("add", "-f", str(tracked))
        index = (self.primary / ".git/index").read_bytes()
        with self.assertRaisesRegex(ValueError, "outside Git"):
            self.run_selection()
        self.assertFalse((self.archive / "source.txt").exists())
        self.assertEqual((self.primary / ".git/index").read_bytes(), index)

    def test_redirected_execution_leaf_and_traversal_are_refused(self):
        sentinel = self.folder / "outside/sentinel.txt"
        other = self.archive.parent / "other/sentinel.txt"
        selected = json.loads(self.selection.read_bytes())["files"]
        try:
            self.archive.symlink_to(sentinel.parent, target_is_directory=True)
        except OSError as error:
            if os.name != "nt":
                self.skipTest(f"Native directory link unavailable: {error}")
            result = subprocess.run(["cmd", "/c", "mklink", "/J", str(self.archive), str(sentinel.parent)],
                                    capture_output=True)
            if result.returncode:
                self.skipTest("Native directory links unavailable in this fixture")
        with self.assertRaisesRegex(ValueError, "linked path"):
            self.run_selection()
        if self.archive.is_symlink():
            self.archive.unlink()
        else:
            self.archive.rmdir()
        self.archive.mkdir()
        leaf = self.archive / "sentinel.txt"
        try:
            leaf.symlink_to(sentinel)
        except OSError:
            # A Windows directory junction at the selected leaf exercises the same guard.
            result = subprocess.run(["cmd", "/c", "mklink", "/J", str(leaf), str(sentinel.parent)], capture_output=True)
            if result.returncode:
                self.skipTest("Native leaf links unavailable in this fixture")
        with self.assertRaisesRegex(ValueError, "linked path"):
            self.run_selection()
        selected[0]["path"] = "../other/sentinel.txt"
        with self.assertRaisesRegex(ValueError, "Unsafe archive path"):
            self.run_selection(selected)
        self.assertEqual(sentinel.read_bytes(), b"do not change")
        self.assertEqual(other.read_bytes(), b"other execution")

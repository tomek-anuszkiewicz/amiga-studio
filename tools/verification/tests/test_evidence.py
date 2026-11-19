"""Exercise batch continuation against real Git fixtures and child processes."""
import contextlib
import errno
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from git_fixtures import restore_repository

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import evidence
import execution
import repository
import verify
import commands
import psutil
from batch_lock import VerificationLock
from impact import RustScope
from timing_report import measurement


class ContinuationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.configure(Path(self.temporary.name).resolve())
        restore_repository("evidence", self.root)
        self.head = (self.root / ".git/refs/heads/main").read_text().strip()

    def configure(self, root):
        self.root = root
        self.words = "tools/verification/checks/english_words/programming.txt"
        self.directory = self.root / ".agent/tmp/verification/commit"
        self.phases = [
            ("Preflight", [("Strict English Policy", self.child("english"), {})]),
            ("Coverage", [("Fixture coverage", self.child("coverage"), {})]),
            ("Verification", [("Verification CLI contracts", self.child("contracts"), {})]),
        ]
        self.condition = "environment-one"
        self.notice = self.enterContext(patch.object(verify, "stable_update_notice", return_value=
            "[INFO] Stable is up to date (latest release: 1.100.0).\n"))

    def restore_successful_attempt(self, state="evidence-passed"):
        restore_repository(state, self.root)
        record = self.record()
        record["checkout"] = evidence.digest(str(self.root))
        inputs = json.loads((self.directory / record["inputs"]).read_text(encoding="utf-8"))
        for name, group in record["groups"].items():
            for _, command, _ in group["commands"]:
                command[0] = sys.executable
            group["identity"] = evidence.group_identity(name, group["commands"], inputs, group["conditions"])
        (self.directory / "commit.json").write_text(json.dumps(record), encoding="utf-8")

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8", newline="\n")

    @contextlib.contextmanager
    def added_input(self, name, text):
        path = self.root / name
        before = path.read_bytes() if path.exists() else None
        self.write(name, text)
        try:
            yield
        finally:
            if before is None:
                path.unlink()
            else:
                path.write_bytes(before)

    def assert_captured_input(self, before, name, content):
        current = evidence.capture_inputs(self.root)
        expected = hashlib.sha256(content.encode()).hexdigest()
        self.assertNotEqual(before.get(name), expected)
        self.assertEqual(current, {**before, name: expected})
        return current

    def assert_identity_change(self, name, before, current, changes_identity):
        command = [(name, ["fixture"], {})]
        original = evidence.group_identity(name, command, before, self.condition)
        updated = evidence.group_identity(name, command, current, self.condition)
        assertion = self.assertNotEqual if changes_identity else self.assertEqual
        assertion(updated, original, name)

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root).decode().strip()

    def commit(self):
        tree = self.git("write-tree")
        args = ["-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit-tree", tree, "-m", "fixture"]
        result = subprocess.run(["git", "rev-parse", "--verify", "HEAD"], cwd=self.root, capture_output=True)
        parent = result.stdout.decode().strip() if result.returncode == 0 else None
        if parent:
            args += ["-p", parent]
        revision = self.git(*args)
        self.git("update-ref", "HEAD", revision)
        return revision

    def child(self, kind):
        code = "from pathlib import Path; import sys; p=Path('.agent/tmp/events'); p.parent.mkdir(parents=True,exist_ok=True); "
        code += f"p.open('a').write('{kind}\\n'); "
        if kind == "english":
            code += f"sys.exit(1 if 'invalid' in Path('{self.words}').read_text() else 0)"
        elif kind == "coverage":
            code += "d=Path('.agent/tmp/coverage'); d.mkdir(exist_ok=True); "
            code += "[(d/n).write_text('{}') for n in ('coverage.json','summary.json','gaps.md')]"
        return [sys.executable, "-c", code]

    def run_profile(self, extra=()):
        return self.run_main(["commit", *extra])

    def run_main(self, args, *, select_phases=None):
        with patch.object(verify, "REPO_ROOT", self.root), patch.object(execution, "REPO_ROOT", self.root), \
             patch.object(verify, "select_phases", return_value=self.phases, side_effect=select_phases), \
             patch.object(verify, "execution_conditions", side_effect=lambda root: self.condition), \
             contextlib.redirect_stdout(io.StringIO()) as output:
            result = verify.main(args)
        self.output = output.getvalue()
        return result

    def events(self):
        path = self.root / ".agent/tmp/events"
        return path.read_text().splitlines() if path.exists() else []

    def record(self):
        return json.loads((self.directory / "commit.json").read_text())

    def load_batch(self):
        batch = evidence.BatchEvidence(self.root, self.directory, "commit")
        with VerificationLock(self.root):
            batch.load()
        return batch

    def test_vocabulary_correction_reuses_coverage_without_resetting_reports(self):
        self.restore_successful_attempt()
        reports = {p.name: p.read_bytes() for p in (self.root / ".agent/tmp/coverage").iterdir()}
        self.write(self.words, "accepted\nmore\n")
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), ["english", "coverage", "contracts", "english"])
        self.assertEqual(self.record()["results"]["Coverage"]["status"], "reused")
        self.assertEqual(reports, {p.name: p.read_bytes() for p in (self.root / ".agent/tmp/coverage").iterdir()})


    def test_each_attempt_preserves_its_report_without_changing_candidate_or_reuse(self):
        before = repository.snapshot(self.root)
        inputs = evidence.capture_inputs(self.root)
        status = self.git("status", "--porcelain=v1")
        index = self.git("write-tree")
        candidate = repository.candidate_tree(self.root)
        self.assertEqual(self.run_profile(), 0, self.output)
        first = self.record()
        timing = first["timing_report"]
        report = self.directory / timing["path"]
        original = report.read_bytes()
        self.assertEqual(timing["path"], f"verify/{first['attempt']}/test-runtime-report.md")
        self.assertEqual(timing["sha256"], evidence.file_digest(report))
        self.assertIn(str(report), self.output)
        self.assertTrue(all(timing["path"] not in item["artifacts"] for item in first["groups"].values()))
        events = self.events()
        # Older completion evidence without an archival locator still supports reuse.
        first["timing_report"].pop("path")
        first["timing_report"].pop("sha256")
        (self.directory / "commit.json").write_text(json.dumps(first), encoding="utf-8")
        self.assertEqual(self.run_profile(), 0, self.output)
        current = self.record()
        self.assertEqual(self.events(), events)
        self.assertTrue(all(item["status"] == "reused" for item in current["results"].values()))
        self.assertNotEqual(current["timing_report"]["path"], f"verify/{first['attempt']}/test-runtime-report.md")
        self.assertEqual(report.read_bytes(), original)
        latest = self.directory / current["timing_report"]["path"]
        self.assertEqual(evidence.file_digest(latest), current["timing_report"]["sha256"])
        self.assertIn("reused", latest.read_text(encoding="utf-8"))
        self.assertEqual(repository.snapshot(self.root), before)
        self.assertEqual(evidence.capture_inputs(self.root), inputs)
        self.assertEqual(self.git("status", "--porcelain=v1"), status)
        self.assertEqual(self.git("write-tree"), index)
        self.assertEqual(repository.candidate_tree(self.root), candidate)
        self.assertEqual(current["tree"], candidate)
        self.assertEqual(current["scope"], first["scope"])
        latest.unlink()
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), events)

    def test_authored_retired_report_created_during_checks_rejects_source(self):
        child = self.phases[1][1][0][1]
        child[-1] += "; p=Path('tests/test-runtime-report.md'); p.parent.mkdir(); p.write_text('data')"
        self.assertEqual(self.run_profile(), 1, self.output)
        self.assertIn("Source or index changed", self.output)
        record = self.record()
        self.assertIn(record["attempt"], record["timing_report"]["path"])
        rows = {row["title"]: row for row in record["timing_report"]["rows"]
                if row["status"] != "skipped"}
        self.assertEqual(rows["Fixture coverage"]["status"], "executed")
        self.assertEqual(rows["Verification CLI contracts"]["status"], "missing")
        self.assertEqual(rows["Verification CLI contracts"]["commands"], [self.phases[-1][1][0][1]])
        self.assertEqual(rows["Complete the attempt"]["status"], "failed")

    def test_neighbors_of_retired_report_are_captured_as_authored_inputs(self):
        before = repository.snapshot(self.root)
        inputs = evidence.capture_inputs(self.root)
        for path in ("tests/neighbor.md", "tests/results/real.txt"):
            with self.subTest(path=path), self.added_input(path, "Authored input.\n"):
                self.assertNotEqual(repository.snapshot(self.root), before)
                self.assert_captured_input(inputs, path, "Authored input.\n")

    def test_report_write_failure_preserves_successful_test_evidence(self):
        with patch.object(verify, "publish", side_effect=PermissionError("Report output locked")):
            self.assertEqual(self.run_profile(), 1, self.output)
        self.assertEqual(self.record()["outcome"], "failed")
        failed = self.record()
        self.assertNotIn("path", failed["timing_report"])
        self.assertNotIn("sha256", failed["timing_report"])
        self.assertTrue(all(item["status"] == "passed" for item in self.record()["groups"].values()))
        events = self.events()
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), events)
        self.assertNotEqual(self.record()["attempt"], failed["attempt"])

    def test_input_capture_abort_keeps_the_prior_report_and_records_pending_commands(self):
        self.assertEqual(self.run_profile(), 0, self.output)
        first = self.record()
        first_report = self.directory / first["timing_report"]["path"]
        for error, exit_code, outcome in (
                (ValueError("Input became unreadable"), 1, "failed"),
                (KeyboardInterrupt(), 130, "interrupted")):
            with self.subTest(outcome=outcome), patch.object(verify, "capture_inputs", side_effect=error):
                self.assertEqual(self.run_profile(), exit_code, self.output)
                attempt = self.record()
                self.assertNotEqual(attempt["attempt"], first["attempt"])
                self.assertEqual(attempt["outcome"], outcome)
                timing = attempt["timing_report"]
                self.assertIn(attempt["attempt"], timing["path"])
                report = self.directory / timing["path"]
                self.assertEqual(timing["sha256"], evidence.file_digest(report))
                self.assertEqual(first["timing_report"]["sha256"], evidence.file_digest(first_report))
                self.assertEqual(
                    [(row["phase"], row["title"], row["status"], row["commands"])
                     for row in timing["rows"] if row["status"] != "skipped"],
                    [("Preflight", "Strict English Policy", "missing", [self.phases[0][1][0][1]]),
                     ("Coverage", "Fixture coverage", "missing", [self.phases[1][1][0][1]]),
                     ("Verification", "Verification CLI contracts", "missing", [self.phases[2][1][0][1]]),
                     ("Verification", "Complete the attempt", outcome, [])])
                report_text = report.read_text(encoding="utf-8")
                executable = Path(sys.executable).name
                self.assertIn(f"Coverage: Fixture coverage | {executable} -c &lt;inline command&gt; | missing |",
                              report_text)
                self.assertIn(f"Complete the attempt |  | {outcome} |", report_text)
        events = self.events()
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), events)

    def test_owned_verifier_change_executes_narrow_contracts(self):
        self.restore_successful_attempt()
        self.write("tools/verification/commands.py", "# Changed selection\n")
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.record()["results"]["Verification CLI contracts"]["status"], "executed")
        self.assertEqual(self.events().count("contracts"), 2)

    def test_verification_contract_identity_tracks_only_owned_inputs(self):
        before = evidence.capture_inputs(self.root)
        for name, changes_identity in (
                ("crates/example/src/example.rs", False),
                (self.words, False),
                ("tools/verification/commands.py", True),
                ("tools/graphify_query.py", True),
        ):
            with self.subTest(name=name), self.added_input(name, "Changed input.\n"):
                current = self.assert_captured_input(before, name, "Changed input.\n")
                self.assert_identity_change("Verification CLI contracts", before, current, changes_identity)

    def test_root_ignore_rule_change_invalidates_verification_contract_evidence(self):
        self.restore_successful_attempt()
        previous = self.record()
        self.write(".gitignore", ".agent/\nignored-fixture/\n")
        self.assertEqual(self.run_profile(), 0, self.output)
        current = self.record()
        contract = "Verification CLI contracts"
        self.assertEqual(current["results"][contract]["status"], "executed")
        self.assertNotEqual(previous["groups"][contract]["identity"], current["groups"][contract]["identity"])
        self.assertEqual(self.events().count("contracts"), 2)

    def test_batch_wall_time_includes_initial_selection(self):
        clock = [0.0]

        def collect(*args):
            clock[0] = 5.0
            return {"source.rs"}

        self.phases = [("Verification", [("Fixture", ["fixture"], {})])]
        with patch.object(verify.time, "perf_counter", side_effect=lambda: clock[0]), \
             patch.object(verify, "snapshot", return_value="source"), \
             patch.object(verify, "collect_changes", side_effect=collect), \
             patch.object(verify, "capture_inputs", return_value={}), \
             patch.object(verify, "candidate_tree", return_value="tree"), \
             patch.object(verify, "execute_command", return_value=(True, "fixture passed", 0)):
            self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.record()["timing_report"]["wall_seconds_before_report"], 5.0)

    def include_diary_groups(self):
        for name in sorted(evidence.DIARY_INDEPENDENT_GROUPS - {"Coverage", "Verification CLI contracts"}):
            self.phases[-1][1].append((name, self.child(name), {}))
        self.phases[-1][1].append(("Unknown diary consumer", self.child("unknown"), {}))

    def test_diary_edit_preserves_known_groups_but_updates_final_candidate(self):
        self.include_diary_groups()
        self.restore_successful_attempt("evidence-diary")
        representative = ("Coverage", "Architecture Rules")
        retained = {"Architecture Rules", "Unknown diary consumer"}
        self.phases[-1][1][:] = [item for item in self.phases[-1][1] if item[0] in retained]
        previous = self.record()
        count = len(self.events())
        diary = "Separate entries for the commit partition."
        self.write("DIARY.md", diary + "\n")
        self.assertEqual(self.run_profile(), 0, self.output)
        current = self.record()
        self.assertEqual(self.events()[count:], ["english", "unknown"])
        self.assertEqual(current["source"], repository.snapshot(self.root))
        self.assertNotEqual(current["source"], previous["source"])
        self.assertNotEqual(current["tree"], previous["tree"])
        self.assertEqual(self.git("show", f"{current['tree']}:DIARY.md"), diary)
        self.assertEqual(current["batch"], previous["batch"])
        for name in representative:
            self.assertEqual(current["results"][name]["status"], "reused", name)
            self.assertEqual(current["groups"][name]["artifacts"], previous["groups"][name]["artifacts"])

    def test_diary_is_excluded_only_by_known_independent_groups(self):
        before = {"source.rs": "source", "DIARY.md": "old diary"}
        current = {**before, "DIARY.md": "new diary"}
        independent_groups = (
            "Obsidian Frontmatter", "Formatting", "Architecture Rules", "Hardware Quality",
            "Code Quality", "Clippy Invariants", "WebAssembly build", "Coverage",
            "All standard harness tests", "Full SingleStepTests", "RAG MCP contracts",
            "Artifact report contracts", "Verification CLI contracts", "Bootstrap contracts",
            "Conversion contracts", "HTML conversion contracts", "PDF conversion contracts",
            "Book review contracts",
        )
        for name in independent_groups:
            with self.subTest(name=name):
                self.assert_identity_change(name, before, current, False)
        for name in ("Strict English Policy", "Unknown diary consumer"):
            with self.subTest(name=name):
                self.assert_identity_change(name, before, current, True)

    def test_diary_change_during_execution_still_rejects_candidate(self):
        self.write("DIARY.md", "Before.\n")
        self.phases[1][1][0][1][-1] += "; Path('DIARY.md').write_text('Changed during tests.')"
        self.assertEqual(self.run_profile(), 1)
        self.assertIn("Source or index changed", self.output)

    def test_interrupt_preserves_completed_groups_for_same_directory_retry(self):
        execute = verify.execute_command
        second = self.child("second coverage command")
        self.phases[1][1].append(("Second coverage command", second, {}))

        def interrupt(command, *args, **kwargs):
            if command == second:
                raise KeyboardInterrupt
            return execute(command, *args, **kwargs)
        with patch.object(verify, "execute_command", side_effect=interrupt):
            self.assertEqual(self.run_profile(), 130)
        record = self.record()
        self.assertEqual(record["outcome"], "interrupted")
        self.assertEqual(record["groups"]["Strict English Policy"]["status"], "passed")
        rows = {row["title"]: row for row in record["timing_report"]["rows"] if row["status"] != "skipped"}
        self.assertEqual(rows["Strict English Policy"]["status"], "executed")
        self.assertEqual(rows["Fixture coverage"]["status"], "executed")
        self.assertEqual(rows["Second coverage command"]["status"], "missing")
        self.assertEqual(rows["Second coverage command"]["commands"], [second])
        self.assertEqual(rows["Verification CLI contracts"]["status"], "missing")
        self.assertEqual(rows["Complete the attempt"]["status"], "interrupted")
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), ["english", "coverage", "coverage", "second coverage command", "contracts"])
        self.assertEqual(self.record()["results"]["Strict English Policy"]["status"], "reused")

    def invocation(self, args, phases=None):
        return (f"import sys; sys.path.insert(0, {str(Path(verify.__file__).parent)!r})\n"
                "from pathlib import Path\nimport verify, execution\n"
                f"verify.REPO_ROOT = execution.REPO_ROOT = Path({str(self.root)!r})\n"
                f"verify.select_phases = lambda *args: {self.phases if phases is None else phases!r}\n"
                f"verify.execution_conditions = lambda root: {self.condition!r}\n"
                "verify.stable_update_notice = lambda root: '[INFO] Stable fixture\\n'\n"
                f"sys.exit(verify.main({args!r}))\n")

    def wait_for(self, path, process, log):
        deadline = time.monotonic() + 10
        while not path.exists() and process.poll() is None and time.monotonic() < deadline:
            time.sleep(0.02)
        if not path.exists():
            status = process.poll()
            output = log.read_text(encoding="utf-8", errors="replace")
            self.fail(f"Fixture did not become ready: {path.name}; exit status {status}\n{output}")

    @contextlib.contextmanager
    def holder(self, root=None):
        root = self.root if root is None else root
        ready = root / ".agent/tmp/holder-ready"
        log = ready.with_suffix(".log")
        log.parent.mkdir(parents=True, exist_ok=True)
        code = (f"import sys; sys.path.insert(0, {str(Path(verify.__file__).parent)!r})\n"
                "from pathlib import Path\nfrom batch_lock import VerificationLock\n"
                f"with VerificationLock(Path({str(root)!r})):\n"
                f"    Path({str(ready)!r}).touch()\n"
                "    sys.stdin.read()\n")
        with log.open("wb") as output, subprocess.Popen(
                [sys.executable, "-u", "-c", code], stdin=subprocess.PIPE,
                stdout=output, stderr=subprocess.STDOUT) as process:
            try:
                self.wait_for(ready, process, log)
                yield process
            finally:
                process.stdin.close()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)
                ready.unlink(missing_ok=True)

    def test_active_owner_blocks_all_profiles_before_selection_or_evidence_writes(self):
        self.restore_successful_attempt()
        previous = {p.relative_to(self.directory): p.read_bytes()
                    for p in self.directory.rglob("*") if p.is_file()}
        marker = self.root / ".agent/tmp/selected"
        failure = "[FAIL] Verification is already running in this repository working directory."
        with self.holder():
            code = self.invocation(["commit"])
            code = code.replace("verify.select_phases = lambda *args:",
                                f"verify.select_phases = lambda *args: Path({str(marker)!r}).touch() or")
            code = code.replace("sys.exit(verify.main(",
                                f"verify.snapshot = lambda *args: Path({str(marker)!r}).touch()\nsys.exit(verify.main(")
            result = subprocess.run([sys.executable, "-c", code], cwd=self.root,
                                    capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertEqual(result.stdout.strip(), failure)
            self.assertFalse(marker.exists())
            with patch.object(verify, "snapshot") as snapshot, \
                 patch.object(verify, "execute_command") as execute:
                for args in (["commit"], ["milestone", "--results-dir", ".agent/tmp/other"],
                             ["test", "python"], ["check", "formatting"],
                             ["runner", "bench", "inventory", "--results-dir", ".agent/tmp/focused"]):
                    with self.subTest(args=args):
                        self.assertEqual(self.run_main(args, select_phases=lambda *args: marker.touch()), 1,
                                         self.output)
                        self.assertEqual(self.output.strip(), failure)
                snapshot.assert_not_called()
                execute.assert_not_called()
            self.assertFalse(marker.exists())
            self.assertFalse((self.root / ".agent/tmp/other").exists())
            self.assertFalse((self.root / ".agent/tmp/focused").exists())
            self.assertEqual(previous, {p.relative_to(self.directory): p.read_bytes()
                                       for p in self.directory.rglob("*") if p.is_file()})

    def test_independent_roots_can_hold_locks_simultaneously(self):
        with tempfile.TemporaryDirectory() as other, self.holder(), self.holder(Path(other)):
            for root in (self.root, Path(other)):
                with self.assertRaises(BlockingIOError):
                    with VerificationLock(root):
                        self.fail("A live holder must exclude this process")

    def test_help_and_plan_do_not_touch_lock_or_execute_while_held(self):
        path = self.root / ".agent/tmp/verification.lock"
        path.parent.mkdir(parents=True)
        contents = b"Help and plans must leave these bytes unchanged."
        path.write_bytes(contents)
        variants = ([], ["--help"], ["commit", "--plan"],
                    ["check", "formatting", "--plan", "--results-dir", ".agent/tmp/focused"])
        def check_variant(args):
            if args == ["--help"]:
                with self.assertRaises(SystemExit) as result:
                    self.run_main(args)
                self.assertEqual(result.exception.code, 0)
            else:
                self.assertEqual(self.run_main(args), 0, self.output)

        with patch.object(verify, "execute_command") as execute:
            with self.holder():
                before = path.stat().st_size, path.stat().st_mtime_ns
                for args in variants:
                    with self.subTest(held=True, args=args):
                        check_variant(args)
                        self.assertEqual((path.stat().st_size, path.stat().st_mtime_ns), before)
            self.assertEqual(path.read_bytes(), contents)
            path.unlink()
            for args in variants:
                with self.subTest(held=False, args=args):
                    check_variant(args)
                    self.assertFalse(path.exists())
            execute.assert_not_called()
        self.notice.assert_not_called()
        self.assertFalse(self.directory.exists())
        self.assertFalse((self.root / ".agent/tmp/focused").exists())

    def test_main_releases_persistent_lock_after_success_failure_and_empty_selection(self):
        path = self.root / ".agent/tmp/verification.lock"
        contents = b"Arbitrary contents are not ownership metadata."
        path.parent.mkdir(parents=True)
        path.write_bytes(contents)
        self.phases = [("Verification", [("Fixture", ["fixture"], {})])]
        for passed in (True, False):
            with self.subTest(passed=passed), \
                 patch.object(verify, "execute_command", return_value=(passed, "Fixture result", 0)) as execute:
                self.assertEqual(self.run_main(["check", "formatting"]), int(not passed), self.output)
                execute.assert_called_once()
                with VerificationLock(self.root) as lock:
                    self.assertFalse(os.get_inheritable(lock.handle.fileno()))
                self.assertEqual(path.read_bytes(), contents)
        self.phases = []
        with patch.object(verify, "execute_command") as execute:
            self.assertEqual(self.run_main(["check", "formatting"]), 0, self.output)
            execute.assert_not_called()
        with VerificationLock(self.root):
            self.assertTrue(path.is_file())
        self.assertEqual(path.read_bytes(), contents)

    def test_lock_io_error_is_not_reported_as_contention_and_closes_handle(self):
        backend = "msvcrt.locking" if os.name == "nt" else "fcntl.flock"
        lock = VerificationLock(self.root)
        with patch(backend, side_effect=OSError(errno.EIO, "Fixture I/O error")):
            with self.assertRaises(OSError) as error:
                with lock:
                    self.fail("Lock acquisition must fail")
            self.assertNotIsInstance(error.exception, BlockingIOError)
            self.assertIsNone(lock.handle)
            with patch.object(verify, "REPO_ROOT", self.root), \
                 patch.object(verify, "snapshot") as snapshot, \
                 contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(verify.main(["commit"]), 1)
            snapshot.assert_not_called()
            self.assertIn("Verification I/O error", output.getvalue())
            self.assertNotIn("already running", output.getvalue())
        with VerificationLock(self.root):
            pass

    def test_killed_verifier_reuses_success_while_unfinished_child_does_not_retain_lock(self):
        ready = self.root / ".agent/tmp/ready"
        release = self.root / ".agent/tmp/release"
        log = self.root / ".agent/tmp/killed-verifier.log"
        log.parent.mkdir(parents=True, exist_ok=True)
        self.phases[1][1][0][1][-1] += (
            "\nimport os,time\n"
            "if not Path('.agent/tmp/ready').exists():\n"
            "    ready = Path('.agent/tmp/ready'); pending = ready.with_suffix('.pending')\n"
            "    pending.write_text(str(os.getpid())); pending.replace(ready)\n"
            "    deadline = time.monotonic() + 20\n"
            "    while not Path('.agent/tmp/release').exists() and time.monotonic() < deadline: time.sleep(0.02)\n")
        with log.open("wb") as output, subprocess.Popen(
                [sys.executable, "-u", "-c", self.invocation(["commit"])], cwd=self.root,
                stdout=output, stderr=subprocess.STDOUT) as process:
            survivor = None
            try:
                self.wait_for(ready, process, log)
                survivor = psutil.Process(int(ready.read_text()))
                process.kill()
                process.wait(timeout=5)
                self.assertTrue(survivor.is_running())
                self.assertTrue((self.root / ".agent/tmp/verification.lock").is_file())
                self.assertEqual(self.run_profile(), 0, self.output)
                self.assertEqual(self.events(), ["english", "coverage", "coverage", "contracts"])
                self.assertEqual(self.record()["results"]["Strict English Policy"]["status"], "reused")
                self.assertEqual(self.record()["results"]["Coverage"]["status"], "executed")
            finally:
                release.touch()
                if process.poll() is None:
                    process.kill()
                process.wait(timeout=5)
                if survivor is not None:
                    try:
                        survivor.wait(timeout=10)
                    except psutil.TimeoutExpired:
                        survivor.kill()
                        survivor.wait(timeout=5)

    def test_early_failure_defers_coverage_and_remains_failure_on_retry(self):
        self.phases = [
            ("Preflight", [("Strict English Policy", ["english"], {})]),
            ("Build checks", [("Clippy Invariants", ["clippy"], {})]),
            ("Coverage", [("Fixture coverage", ["coverage"], {})]),
            ("Verification", [("Verification CLI contracts", ["contracts"], {})]),
        ]

        def execute(command, *args, **kwargs):
            name, = command
            if name == "coverage":
                for report in ("coverage.json", "summary.json", "gaps.md"):
                    self.write(f".agent/tmp/coverage/{report}", "{}")
            passed = name != "english" or (self.root / self.words).read_text().strip() == "accepted"
            return passed, "Fixture result", 0

        self.write(self.words, "invalid\n")
        with patch.object(verify, "execute_command", side_effect=execute) as executor:
            for attempt in range(2):
                with self.subTest(attempt=attempt):
                    self.assertEqual(self.run_profile(), 1, self.output)
                    self.assertEqual([call.args[0] for call in executor.call_args_list], [["english"]])
                    self.assertEqual(self.record()["groups"]["Strict English Policy"]["status"], "failed")
                    self.assertEqual(self.record()["results"]["Coverage"]["status"], "missing")
                    executor.reset_mock()
            batch = self.record()["batch"]
            self.write(self.words, "accepted\n")
            self.assertEqual(self.run_profile(), 0, self.output)
            self.assertEqual([call.args[0] for call in executor.call_args_list],
                             [["english"], ["clippy"], ["coverage"], ["contracts"]])
            self.assertEqual(self.record()["batch"], batch)

    def test_build_gate_is_separate_and_blocks_coverage_on_failure(self):
        _, args = verify.parse_args(["commit", "--plan"])
        with patch.object(commands, "select_rust_scope", return_value=RustScope(("config",), "fixture")):
            selected = commands.select_phases(args, {"crates/config/src/config.rs"})
        self.assertEqual([phase for phase, _ in selected][:3], ["Preflight", "Build checks", "Coverage"])
        self.assertEqual([name for name, _, _ in selected[1][1]],
                         ["Clippy Invariants", "WebAssembly build"])
        preflight = self.phases[0][1][0][1]
        clippy = selected[1][1][0][1]
        self.phases.insert(1, selected[1])
        with patch.object(verify, "execute_command", side_effect=[(True, "Passed", 0), (False, "Failed", 0)]) as execute:
            self.assertEqual(self.run_profile(), 1, self.output)
        self.assertEqual([call.args[0] for call in execute.call_args_list], [preflight, clippy])
        self.assertEqual({name: result["status"] for name, result in self.record()["results"].items()},
                         {"Strict English Policy": "executed", "Clippy Invariants": "failed",
                          "WebAssembly build": "missing", "Coverage": "missing",
                          "Verification CLI contracts": "missing"})

    def test_coverage_bundle_stops_at_every_failed_step_before_summary(self):
        names = ["prepare", "reset", "tests", "export", "summary"]
        items = [(name, [name], {}) for name in names]
        _, args = verify.parse_args(["commit"])
        for index, failing in enumerate(names[:-1]):
            with self.subTest(failing=failing), contextlib.redirect_stdout(io.StringIO()), \
                 patch.object(verify, "execute_command", side_effect=[
                     *[(True, "Passed", 1)] * index, (False, "Failed", 1),
                 ]) as execute:
                rows = [measurement("Coverage", "Coverage", name, "missing", commands=[[name]])
                        for name in names]
                self.assertEqual(verify.execute_group("Coverage", "Coverage", items, args, self.directory, rows),
                                 (False, index + 1))
                self.assertEqual([call.args[0] for call in execute.call_args_list],
                                 [[name] for name in names[:index + 1]])
                self.assertEqual([row["status"] for row in rows],
                                 ["executed"] * index + ["failed"] + ["missing"] * (len(names) - index - 1))

    def test_failed_serial_coverage_keeps_other_commit_checks(self):
        failed = self.child("tests")
        failed[-1] += "sys.exit(1)"
        self.phases = [
            ("Coverage", [("Prepare", self.child("prepare"), {}), ("Tests", failed, {}),
                          ("Export", self.child("export"), {})]),
            ("Verification", [("Remaining gate", self.child("gate"), {})]),
        ]
        self.assertEqual(self.run_profile(), 1, self.output)
        self.assertEqual(self.events(), ["prepare", "tests", "gate"])
        self.assertEqual({name: group["status"] for name, group in self.record()["groups"].items()},
                         {"Coverage": "failed", "Remaining gate": "passed"})

    def test_failed_web_build_blocks_commit_and_cannot_be_reused_as_success(self):
        command = self.child("web")
        command[-1] += "sys.exit(1)"
        self.phases.insert(1, ("Build checks", [("WebAssembly build", command, {})]))
        self.assertEqual(self.run_profile(), 1, self.output)
        self.assertEqual(self.events(), ["english", "web"])
        self.assertEqual(self.record()["groups"]["WebAssembly build"]["status"], "failed")
        execute = verify.execute_command
        attempted = []
        outcomes = iter((False, True))

        def web_retry(argv, *args, **kwargs):
            if argv == command:
                attempted.append(argv)
                return next(outcomes), "Fixture web result", 0
            return execute(argv, *args, **kwargs)

        with patch.object(verify, "execute_command", side_effect=web_retry):
            self.assertEqual(self.run_profile(), 1, self.output)
            self.assertEqual(attempted, [command])
            self.assertEqual(self.events(), ["english", "web"])
            self.assertEqual({name: result["status"] for name, result in self.record()["results"].items()},
                             {"Strict English Policy": "reused", "WebAssembly build": "failed",
                              "Coverage": "missing", "Verification CLI contracts": "missing"})
            self.assertEqual(self.run_profile(), 0, self.output)
            self.assertEqual(attempted, [command, command])
        self.assertEqual(self.events(), ["english", "web", "coverage", "contracts"])
        self.assertEqual({name: result["status"] for name, result in self.record()["results"].items()},
                         {"Strict English Policy": "reused", "WebAssembly build": "executed",
                          "Coverage": "executed", "Verification CLI contracts": "executed"})

    def test_commit_captures_source_before_change_selection(self):
        events = []
        def fingerprint(root):
            events.append("snapshot")
            return "before" if len(events) == 1 else "after"
        def changes(root):
            events.append("changes")
            return {"source.rs"}
        with patch.object(verify, "snapshot", side_effect=fingerprint), \
             patch.object(verify, "collect_changes", side_effect=changes):
            self.assertEqual(self.run_profile(), 1)
        self.assertEqual(events, ["snapshot", "changes", "snapshot"])
        self.assertIn("Source or index changed", self.output)
        self.assertEqual(self.events(), [])

    def test_removed_completion_options_are_rejected(self):
        for option, value in (("--base", "HEAD"), ("--resume", ".agent/tmp/batch"),
                              ("--editorial", "DIARY.md=review")):
            with self.subTest(option=option), contextlib.redirect_stderr(io.StringIO()), \
                 self.assertRaises(SystemExit) as error:
                verify.parse_args(["commit", option, value])
            self.assertEqual(error.exception.code, 2)

    def test_tracked_and_untracked_changes_invalidate_execution_on_successive_replays(self):
        self.restore_successful_attempt()
        for name in ("source.rs", "unknown.input"):
            with self.subTest(name=name):
                before = self.events()
                previous = self.record()
                inputs = evidence.capture_inputs(self.root)
                self.write(name, "New input.\n")
                self.assert_captured_input(inputs, name, "New input.\n")
                self.assertNotEqual(repository.snapshot(self.root), previous["source"])
                self.assertEqual(self.run_profile(), 0, self.output)
                current = self.record()
                self.assertEqual(current["batch"], previous["batch"])
                self.assertNotEqual(current["source"], previous["source"])
                self.assertEqual(self.events()[len(before):], ["english", "coverage"])
                self.assertEqual(current["results"]["Coverage"]["status"], "executed")
                self.assertEqual(current["results"]["Verification CLI contracts"]["status"], "reused")

    def test_capture_invalidates_coverage_for_tests_manifests_checkers_and_design(self):
        before = evidence.capture_inputs(self.root)
        for name in (
                "tests/assertions.rs", "tools/verification/checks/check_architecture.py",
                "Cargo.toml", ".config/nextest.toml", "Obsidian/Amiga/Design/Contract.md",
        ):
            with self.subTest(name=name), self.added_input(name, "New input.\n"):
                current = self.assert_captured_input(before, name, "New input.\n")
                self.assert_identity_change("Coverage", before, current, True)
                self.assert_identity_change("Verification CLI contracts", before, current,
                                            name == "tools/verification/checks/check_architecture.py")

    def test_changed_execution_conditions_invalidate_all_groups(self):
        self.restore_successful_attempt()
        self.condition = "environment-two"
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), ["english", "coverage", "contracts"] * 2)

    def test_tool_call_identifier_is_not_an_execution_condition(self):
        version = subprocess.CompletedProcess([], 0, stdout=b"version", stderr=b"")
        with patch.dict(evidence.os.environ, {"CODEX_TOOL_CALL_ID": "first", "RUSTFLAGS": ""}), \
             patch.object(evidence.subprocess, "run", return_value=version), \
             patch.object(evidence.platform, "platform", return_value="fixture-platform"), \
             patch.object(evidence.importlib.metadata, "distributions", return_value=[]), \
             patch.object(evidence, "file_digest", return_value="content"):
            original = evidence.execution_conditions(self.root)
            evidence.os.environ["CODEX_TOOL_CALL_ID"] = "second"
            self.assertEqual(evidence.execution_conditions(self.root), original)
            evidence.os.environ["RUSTFLAGS"] = "-C opt-level=1"
            self.assertNotEqual(evidence.execution_conditions(self.root), original)

    def test_runner_and_browser_tool_versions_invalidate_execution_conditions(self):
        changed = {}

        def version(command, **kwargs):
            return subprocess.CompletedProcess(command, 0, stdout=changed.get(tuple(command), b"version"),
                                               stderr=b"")

        with patch.object(evidence.subprocess, "run", side_effect=version), \
             patch.object(evidence.platform, "platform", return_value="fixture-platform"), \
             patch.object(evidence.importlib.metadata, "distributions", return_value=[]), \
             patch.object(evidence, "file_digest", return_value="content"):
            original = evidence.execution_conditions(self.root)
            shell = "powershell" if evidence.os.name == "nt" else "pwsh"
            for command in (("cargo", "nextest", "--version"),
                            ("cargo", "+nightly", "--version"), ("rustc", "+nightly", "-vV"),
                            ("trunk", "--version"),
                            (shell, "-NoProfile", "-Command", "$PSVersionTable.PSVersion.ToString()")):
                with self.subTest(command=command):
                    changed[command] = b"new version"
                    self.assertNotEqual(evidence.execution_conditions(self.root), original)
                    changed.clear()

    def test_coverage_reuse_requires_every_retained_artifact(self):
        self.restore_successful_attempt()
        batch = self.load_batch()
        group = batch.record["groups"]["Coverage"]
        identity = group["identity"]
        self.assertIsNone(batch.reason("Coverage", identity))
        artifacts = {Path(relative).name: relative for relative in group["artifacts"]}
        for name in ("coverage.json", "summary.json", "gaps.md"):
            with self.subTest(name=name):
                relative = artifacts[name]
                path = self.directory / relative
                expected = b"{}"
                self.assertEqual(path.read_bytes(), expected)
                self.assertEqual(group["artifacts"][relative], hashlib.sha256(expected).hexdigest())
                path.unlink()
                try:
                    self.assertEqual(batch.reason("Coverage", identity), "missing or corrupt evidence artifact")
                finally:
                    path.write_bytes(expected)
        summary = self.directory / artifacts["summary.json"]
        summary.write_text("corrupt", encoding="utf-8")
        self.assertEqual(batch.reason("Coverage", identity), "missing or corrupt evidence artifact")

        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), ["english", "coverage", "contracts", "coverage"])
        current = self.record()
        self.assertEqual(current["results"]["Coverage"]["status"], "executed")
        self.assertEqual(current["groups"]["Coverage"]["status"], "passed")
        refreshed = {Path(relative).name: relative for relative in current["groups"]["Coverage"]["artifacts"]}
        for name in ("coverage.json", "summary.json", "gaps.md"):
            self.assertEqual((self.directory / refreshed[name]).read_bytes(), b"{}", name)

    def test_interrupted_group_and_new_required_group_both_execute(self):
        self.restore_successful_attempt()
        record = self.record()
        record["groups"]["Coverage"]["status"] = "running"
        (self.directory / "commit.json").write_text(json.dumps(record), encoding="utf-8")
        self.phases[-1][1].append(("New required suite", self.child("new"), {}))
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), ["english", "coverage", "contracts", "coverage", "new"])
        self.assertEqual({name: result["status"] for name, result in self.record()["results"].items()},
                         {"Strict English Policy": "reused", "Coverage": "executed",
                          "Verification CLI contracts": "reused", "New required suite": "executed"})

    def test_successful_group_rejects_different_command_or_overrides(self):
        self.restore_successful_attempt()
        batch = self.load_batch()
        name = "Verification CLI contracts"
        group = batch.record["groups"][name]
        self.assertEqual(group["status"], "passed")
        ((title, argv, overrides),) = group["commands"]
        inputs = json.loads((self.directory / group["inputs"]).read_text(encoding="utf-8"))
        current = evidence.group_identity(name, group["commands"], inputs, group["conditions"])
        self.assertIsNone(batch.reason(name, current))
        variants = {"argv": (argv + ["extra-argument"], overrides),
                    "overrides": (argv, {**overrides, "FIXTURE_SETTING": "changed"})}
        for field, (new_argv, new_overrides) in variants.items():
            with self.subTest(field=field):
                identity = evidence.group_identity(name, [(title, new_argv, new_overrides)],
                                                   inputs, group["conditions"])
                self.assertEqual(batch.reason(name, identity),
                                 "inputs, command, selection or execution conditions changed")

    def test_source_change_inside_child_rejects_attempt(self):
        self.phases[1][1][0][1][-1] += "; Path('source.rs').write_text('raced')"
        self.assertEqual(self.run_profile(), 1)
        self.assertIn("Source or index changed", self.output)
        self.assertNotIn("contracts", self.events())
        self.assertFalse(any(item["status"] == "passed" for item in self.record()["groups"].values()))

    def test_identical_fixture_rewrite_preserves_content_identity(self):
        self.phases[1][1][0][1][-1] += "; p=Path('source.rs'); p.write_bytes(p.read_bytes())"
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events().count("coverage"), 1)

    def test_document_is_a_conservative_input_even_without_rust_edit(self):
        self.restore_successful_attempt()
        name = "docs/included.md"
        self.write(name, "Build or test input.\n")
        before = evidence.capture_inputs(self.root)
        self.write(name, "Changed input.\n")
        current = self.assert_captured_input(before, name, "Changed input.\n")
        self.assert_identity_change("Coverage", before, current, True)
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.record()["results"]["Coverage"]["status"], "executed")
        self.assertEqual(self.events().count("coverage"), 2)

    def test_absent_and_invalid_records_leave_a_fresh_batch(self):
        self.restore_successful_attempt()
        previous = self.record()
        path = self.directory / "commit.json"
        wrong_groups = json.dumps({**previous, "groups": []})
        self.assertEqual(self.load_batch().record["batch"], previous["batch"])
        for label, value in (("absent", None), ("invalid JSON", "broken"),
                             ("wrong envelope type", "[]"), ("wrong groups type", wrong_groups)):
            with self.subTest(label=label):
                path.unlink() if value is None else path.write_text(value, encoding="utf-8")
                batch = evidence.BatchEvidence(self.root, self.directory, "commit")
                fresh = batch.record["batch"]
                with VerificationLock(self.root), contextlib.redirect_stdout(io.StringIO()):
                    batch.load()
                self.assertNotEqual(fresh, previous["batch"])
                self.assertEqual(batch.record["batch"], fresh)
                self.assertEqual(batch.record["groups"], {})
                self.assertEqual(batch.reason("Coverage", previous["groups"]["Coverage"]["identity"]),
                                 "missing successful evidence")

    def test_corrupt_persisted_record_recovers_with_fresh_execution(self):
        self.restore_successful_attempt()
        before = self.events()
        previous = self.record()
        (self.directory / "commit.json").write_text("broken", encoding="utf-8")
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events()[len(before):], ["english", "coverage", "contracts"])
        record = self.record()
        self.assertNotEqual(record["batch"], previous["batch"])
        self.assertEqual({name: result["status"] for name, result in record["results"].items()},
                         {"Strict English Policy": "executed", "Coverage": "executed",
                          "Verification CLI contracts": "executed"})

    def test_new_head_starts_fresh_even_when_input_contents_are_unchanged(self):
        self.restore_successful_attempt()
        previous_batch = self.record()["batch"]
        new_head = self.commit()
        self.assertNotEqual(new_head, self.head)
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), ["english", "coverage", "contracts"] * 2)
        self.assertEqual(self.record()["head"], new_head)
        self.assertNotEqual(self.record()["batch"], previous_batch)

    def test_repeated_custom_results_directory_reuses_evidence(self):
        self.directory = self.root / ".agent/tmp/custom"
        options = ("--results-dir", ".agent/tmp/custom")
        self.assertEqual(self.run_profile(extra=options), 0, self.output)
        self.assertEqual(self.run_profile(extra=options), 0, self.output)
        self.assertEqual(self.events(), ["english", "coverage", "contracts"])
        self.assertEqual(self.record()["results"]["Coverage"]["status"], "reused")
        self.assertFalse((self.root / ".agent/tmp/verification").exists())

    def test_completion_plan_writes_no_evidence_or_logs(self):
        self.assertEqual(self.run_profile(extra=("--plan",)), 0, self.output)
        self.assertFalse((self.root / ".agent").exists())
        self.notice.assert_not_called()

    def test_stable_notice_runs_once_per_attempt_even_with_reused_tests(self):
        self.assertEqual(self.run_profile(), 0, self.output)
        previous = self.record()
        events = self.events()
        update = "[INFO] Stable update available: 1.99.0 -> 1.100.0. Run: rustup update stable\n"
        self.notice.return_value = update
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.notice.call_count, 2)
        self.assertEqual(self.events(), events)
        current = self.record()
        self.assertEqual(current["groups"], previous["groups"])
        self.assertEqual(current["conditions"], previous["conditions"])
        self.assertTrue(all(item["status"] == "reused" for item in current["results"].values()))
        log = self.directory / "verify" / current["attempt"] / "stable-update.log"
        self.assertEqual(log.read_text(encoding="utf-8"), update)
        self.assertIn(update.strip(), self.output)

    def test_stable_notice_cannot_hide_required_gate_failure(self):
        self.phases = [("Preflight", [("Strict English Policy", ["english"], {})])]
        for notice in ("[INFO] Stable update available. Run: rustup update stable\n",
                       "[INFO] Could not check Stable updates: timed out\n"):
            with self.subTest(notice=notice), \
                 patch.object(verify, "execute_command", return_value=(False, "Failed gate", 0)) as execute:
                self.notice.return_value = notice
                self.assertEqual(self.run_profile(), 1, self.output)
                execute.assert_called_once()
                record = self.record()
                self.assertEqual(record["outcome"], "failed")
                self.assertEqual(record["results"]["Strict English Policy"]["status"], "failed")
                self.assertIn(notice.strip(), self.output)

    def test_unavailable_stable_notice_does_not_block_required_tests(self):
        self.notice.return_value = "[INFO] Could not check Stable updates: timed out\n"
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), ["english", "coverage", "contracts"])

    def test_singlestep_reports_are_retained_and_corruption_requires_execution(self):
        command = self.child("singlestep")
        command[-1] += "d=Path('.agent/tmp/verification/commit/singlestep'); d.mkdir(exist_ok=True); "
        command[-1] += "[(d/n).write_text('{}') for n in ('run.json','timing.json','summary.json','execution.log')];"
        command.append("tools/verification/run_singlestep.py")
        self.phases[-1][1].append(("Full SingleStepTests", command, {}))
        self.assertEqual(self.run_profile(), 0, self.output)
        record = self.record()["groups"]["Full SingleStepTests"]
        relative = next(name for name in record["artifacts"] if name.endswith("timing.json"))
        (self.directory / relative).write_text("broken")
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events().count("singlestep"), 2)

    def test_scope_excludes_committed_source_and_retains_all_pending_changes(self):
        self.git("add", "source.rs")
        self.commit()
        self.write(self.words, "more\n")
        self.write("pending.rs", "pending input\n")
        self.assertEqual(repository.collect_changes(self.root), {self.words, "pending.rs"})

    def barrier_child(self, kind, own, peer):
        command = self.child(kind)
        command[-1] += (
            f"\nimport time; Path('.agent/tmp/{own}').touch(); deadline=time.monotonic()+3\n"
            f"while not Path('.agent/tmp/{peer}').exists():\n"
            " if time.monotonic()>deadline: raise RuntimeError('Independent command did not overlap')\n"
            " time.sleep(0.01)\n"
        )
        return command

    def test_parallel_preflight_finishes_before_builds_and_preserves_blockers(self):
        for failing in (False, True):
            with self.subTest(failing=failing):
                left = self.barrier_child("left", f"left-{failing}", f"right-{failing}")
                right = self.barrier_child("right", f"right-{failing}", f"left-{failing}")
                left_done = f".agent/tmp/left-{failing}.done"
                right_done = f".agent/tmp/right-{failing}.done"
                left[-1] += f"Path({left_done!r}).touch()\n"
                right[-1] += f"time.sleep(0.25); Path({right_done!r}).touch()\n"
                if failing:
                    left[-1] += "sys.exit(1)\n"
                build = self.child("build")
                build[-1] += f"assert Path({left_done!r}).is_file() and Path({right_done!r}).is_file()"
                self.phases = [
                    ("Preflight", [("Left gate", left, {}), ("Right gate", right, {})]),
                    ("Build checks", [("Build gate", build, {})]),
                ]
                before = len(self.events())
                directory = f".agent/tmp/parallel-{failing}"
                self.directory = self.root / directory
                self.assertEqual(self.run_profile(("--results-dir", directory)), int(failing), self.output)
                events = self.events()[before:]
                self.assertCountEqual(events[:2], ["left", "right"])
                self.assertEqual(events[2:], [] if failing else ["build"])
                self.assertTrue((self.root / left_done).is_file())
                self.assertTrue((self.root / right_done).is_file())
                self.assertEqual({name: result["status"] for name, result in self.record()["results"].items()},
                                 {"Left gate": "failed" if failing else "executed", "Right gate": "executed",
                                  "Build gate": "missing" if failing else "executed"})

    def test_parallel_coverage_and_contract_failure_reuse_only_completed_groups(self):
        coverage = self.barrier_child("coverage", "coverage-ready", "contracts-ready")
        contracts = self.barrier_child("contracts", "contracts-ready", "coverage-ready")
        contracts[-1] += "sys.exit(1)\n"
        self.phases = [("Coverage", [("Fixture coverage", coverage, {})]),
                       ("Verification", [("Verification CLI contracts", contracts, {})])]
        with patch.object(verify, "independent_contract", side_effect=lambda name, items: name == "Verification CLI contracts"):
            self.assertEqual(self.run_profile(), 1, self.output)
            self.assertEqual(self.record()["groups"]["Coverage"]["status"], "passed")
            self.assertEqual(self.record()["groups"]["Verification CLI contracts"]["status"], "failed")
            rows = self.record()["timing_report"]["rows"]
            self.assertTrue(any(row["group"] == "Coverage" and row["status"] == "executed" for row in rows))
            self.assertTrue(any(row["group"] == "Verification CLI contracts" and row["status"] == "failed"
                                for row in rows))
            before = self.events().count("coverage")
            contracts[-1] = contracts[-1].replace("sys.exit(1)\n", "")
            self.assertEqual(self.run_profile(), 0, self.output)
            self.assertEqual(self.events().count("coverage"), before)
            self.assertEqual(self.record()["results"]["Coverage"]["status"], "reused")
            rows = self.record()["timing_report"]["rows"]
            self.assertTrue(any(row["group"] == "Coverage" and row["status"] == "reused"
                                and row["seconds"] is None for row in rows))
            self.assertTrue(any(row["group"] == "Verification CLI contracts" and row["status"] == "executed"
                                for row in rows))

    def test_parallel_coverage_failure_stops_export_and_preserves_contract_outcome(self):
        failed = self.child("tests")
        failed[-1] += "sys.exit(1)"
        self.phases = [("Coverage", [("Prepare", self.child("prepare"), {}),
                                     ("Tests", failed, {}), ("Export", self.child("export"), {})]),
                       ("Verification", [("Verification CLI contracts", self.child("contracts"), {})])]
        with patch.object(verify, "independent_contract", side_effect=lambda name, items: name == "Verification CLI contracts"):
            self.assertEqual(self.run_profile(), 1, self.output)
        self.assertCountEqual(self.events(), ["prepare", "tests", "contracts"])
        self.assertEqual(self.record()["groups"]["Coverage"]["status"], "failed")
        self.assertEqual(self.record()["groups"]["Verification CLI contracts"]["status"], "passed")
        rows = self.record()["timing_report"]["rows"]
        self.assertTrue(any(row["title"] == "Tests" and row["status"] == "failed" for row in rows))
        self.assertTrue(any(row["title"] == "Export" and row["status"] == "missing" for row in rows))

    def test_parallel_source_edit_invalidates_the_combined_attempt(self):
        edit = self.barrier_child("edit", "edit-ready", "peer-ready")
        edit[-1] += "Path('source.rs').write_text('changed during parallel work')\n"
        peer = self.barrier_child("peer", "peer-ready", "edit-ready")
        self.phases = [("Preflight", [("Editing gate", edit, {}), ("Peer gate", peer, {})]),
                       ("Coverage", [("Fixture coverage", self.child("coverage"), {})])]
        self.assertEqual(self.run_profile(), 1, self.output)
        self.assertIn("Source or index changed", self.output)
        self.assertNotIn("coverage", self.events())
        self.assertTrue(all(group["status"] != "passed" for group in self.record()["groups"].values()))

    def test_parallel_interrupt_never_marks_unfinished_coverage_passed(self):
        coverage = self.child("coverage")
        self.phases = [("Coverage", [("Fixture coverage", coverage, {})]),
                       ("Verification", [("Verification CLI contracts", self.child("contracts"), {})])]
        execute = verify.execute_command

        def interrupt(command, *args, **kwargs):
            if command == coverage:
                raise KeyboardInterrupt
            return execute(command, *args, **kwargs)

        with patch.object(verify, "independent_contract", side_effect=lambda name, items: name == "Verification CLI contracts"):
            with patch.object(verify, "execute_command", side_effect=interrupt):
                self.assertEqual(self.run_profile(), 130, self.output)
            self.assertEqual(self.record()["outcome"], "interrupted")
            self.assertNotEqual(self.record()["groups"]["Coverage"]["status"], "passed")
            self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events().count("coverage"), 1)

    def test_scope_catalog_change_is_distinct_from_exact_file_words(self):
        name = "tools/verification/checks/check_english.json"
        baseline = {"extensions": [".rs"], "file_words": {}}
        exact_words = {**baseline, "file_words": {"source.rs": ["literal"]}}
        scan_extensions = {**baseline, "extensions": [".py"]}
        variants = []
        for settings in (baseline, exact_words, scan_extensions):
            self.write(name, json.dumps(settings))
            variants.append(evidence.capture_inputs(self.root))
        before, words, extensions = variants
        key = "@english-scan-scope"
        self.assertEqual(before[key], words[key])
        self.assertNotEqual(before[key], extensions[key])
        self.assert_identity_change("Coverage", before, words, False)
        self.assert_identity_change("Coverage", before, extensions, True)
        for current in (words, extensions):
            self.assert_identity_change("Strict English Policy", before, current, True)

        self.write(name, json.dumps(baseline))
        self.assertEqual(self.run_profile(), 0, self.output)
        previous = len(self.events())
        self.write(name, json.dumps(scan_extensions))
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events()[previous:previous + 2], ["english", "coverage"])
        for group in ("Strict English Policy", "Coverage"):
            self.assertEqual(self.record()["results"][group]["status"], "executed")


if __name__ == "__main__":
    unittest.main()

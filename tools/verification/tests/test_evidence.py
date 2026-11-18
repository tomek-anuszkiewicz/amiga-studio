"""Exercise batch continuation against real Git fixtures and child processes."""
import contextlib
import errno
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

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import evidence
import execution
import repository
import verify
import commands
import psutil
from batch_lock import VerificationLock
from impact import RustScope


class ContinuationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.write(".gitignore", ".agent/\n")
        self.write("source.rs", "fn main() {}\n")
        self.words = "tools/verification/checks/english_words/programming.txt"
        self.write(self.words, "accepted\n")
        self.git("init", "-q")
        self.git("add", ".")
        self.head = self.commit()
        self.write("source.rs", "fn main() { /* changed */ }\n")
        self.directory = self.root / ".agent/tmp/verification/commit"
        self.phases = [
            ("Preflight", [("Strict English Policy", self.child("english"), {})]),
            ("Coverage", [("Fixture coverage", self.child("coverage"), {})]),
            ("Verification", [("Verification CLI contracts", self.child("contracts"), {})]),
        ]
        self.condition = "environment-one"
        self.notice = self.enterContext(patch.object(verify, "stable_update_notice", return_value=
            "[INFO] Stable is up to date (latest release: 1.100.0).\n"))

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

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
        args = ["commit", *extra]
        with patch.object(verify, "REPO_ROOT", self.root), patch.object(execution, "REPO_ROOT", self.root), \
             patch.object(verify, "select_phases", return_value=self.phases), \
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

    def test_vocabulary_correction_reuses_coverage_without_resetting_reports(self):
        self.assertEqual(self.run_profile(), 0, self.output)
        reports = {p.name: p.read_bytes() for p in (self.root / ".agent/tmp/coverage").iterdir()}
        self.write(self.words, "accepted\nmore\n")
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), ["english", "coverage", "contracts", "english"])
        self.assertEqual(self.record()["results"]["Coverage"]["status"], "reused")
        self.assertEqual(reports, {p.name: p.read_bytes() for p in (self.root / ".agent/tmp/coverage").iterdir()})

    def test_unchanged_answer_runs_no_children(self):
        self.assertEqual(self.run_profile(), 0, self.output)
        events = self.events()
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), events)

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
        self.write("source.rs", "fn changed_again() {}\n")
        self.assertNotEqual(repository.snapshot(self.root), before)
        self.assertNotEqual(evidence.capture_inputs(self.root), inputs)
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events()[len(events):], ["english", "coverage"])

    def test_authored_retired_report_and_neighbor_created_during_checks_reject_source(self):
        child = self.phases[1][1][0][1]
        original = child[-1]
        for path in ("tests/test-runtime-report.md", "tests/neighbor.md", "tests/results/real.txt"):
            with self.subTest(path=path):
                child[-1] = original + (
                    f"; p=Path({path!r}); p.parent.mkdir(parents=True, exist_ok=True); p.write_text('data')")
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
                (self.root / path).unlink()

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

    def test_verification_contract_identity_tracks_tooling_but_not_emulator_or_vocabulary(self):
        self.assertEqual(self.run_profile(), 0, self.output)
        self.write("crates/example/src/example.rs", "pub fn guest() {}")
        self.write(self.words, "accepted\nmore\n")
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events().count("contracts"), 1)
        self.write("tools/verification/commands.py", "# Changed selection\n")
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events().count("contracts"), 2)
        self.write("tools/graphify_query.py", "# Changed graph query\n")
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events().count("contracts"), 3)

    def test_root_ignore_rule_change_invalidates_verification_contract_evidence(self):
        self.assertEqual(self.run_profile(), 0, self.output)
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
        selection_seen = [False]
        collect = verify.collect_changes

        def select(*args):
            if not selection_seen[0]:
                clock[0] += 5.0
                selection_seen[0] = True
            return collect(*args)

        with patch.object(verify.time, "perf_counter", side_effect=lambda: clock[0]), \
             patch.object(verify, "collect_changes", side_effect=select):
            self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.record()["timing_report"]["wall_seconds_before_report"], 5.0)

    def test_diary_edit_preserves_known_groups_but_updates_final_candidate(self):
        self.write("DIARY.md", "First entry.\n")
        for name in sorted(evidence.DIARY_INDEPENDENT_GROUPS - {"Coverage", "Verification CLI contracts"}):
            self.phases[-1][1].append((name, self.child(name), {}))
        self.phases[-1][1].append(("Unknown diary consumer", self.child("unknown"), {}))
        self.assertEqual(self.run_profile(), 0, self.output)
        previous = self.record()
        count = len(self.events())
        self.write("DIARY.md", "Separate entries for the commit partition.\n")
        self.assertEqual(self.run_profile(), 0, self.output)
        current = self.record()
        self.assertEqual(self.events()[count:], ["english", "unknown"])
        self.assertNotEqual(current["source"], previous["source"])
        self.assertEqual(current["batch"], previous["batch"])
        for name in evidence.DIARY_INDEPENDENT_GROUPS:
            self.assertEqual(current["results"][name]["status"], "reused", name)
            self.assertEqual(current["groups"][name]["artifacts"], previous["groups"][name]["artifacts"])

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

    def wait_for(self, path, process):
        deadline = time.monotonic() + 10
        while not path.exists() and process.poll() is None and time.monotonic() < deadline:
            time.sleep(0.02)
        self.assertTrue(path.exists(), f"Fixture did not become ready: {path.name}")

    @contextlib.contextmanager
    def holder(self, root=None):
        root = self.root if root is None else root
        ready = root / ".agent/tmp/holder-ready"
        code = (f"import sys; sys.path.insert(0, {str(Path(verify.__file__).parent)!r})\n"
                "from pathlib import Path\nfrom batch_lock import VerificationLock\n"
                f"with VerificationLock(Path({str(root)!r})):\n"
                f"    Path({str(ready)!r}).touch()\n"
                "    sys.stdin.read()\n")
        with tempfile.TemporaryFile() as log, subprocess.Popen(
                [sys.executable, "-c", code], stdin=subprocess.PIPE, stdout=log, stderr=log) as process:
            try:
                self.wait_for(ready, process)
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
        self.assertEqual(self.run_profile(), 0, self.output)
        previous = {p.relative_to(self.directory): p.read_bytes()
                    for p in self.directory.rglob("*") if p.is_file()}
        marker = self.root / ".agent/tmp/selected"
        with self.holder():
            for args in (["commit"], ["milestone", "--results-dir", ".agent/tmp/other"],
                         ["test", "python"], ["check", "formatting"],
                         ["runner", "bench", "inventory", "--results-dir", ".agent/tmp/focused"]):
                code = self.invocation(args)
                code = code.replace("verify.select_phases = lambda *args:",
                                    f"verify.select_phases = lambda *args: Path({str(marker)!r}).touch() or")
                code = code.replace("sys.exit(verify.main(",
                                    f"verify.snapshot = lambda *args: Path({str(marker)!r}).touch()\nsys.exit(verify.main(")
                with self.subTest(args=args):
                    result = subprocess.run([sys.executable, "-c", code], cwd=self.root,
                                            capture_output=True, text=True, timeout=10)
                    self.assertEqual(result.returncode, 1, result.stderr)
                    self.assertEqual(result.stdout.strip(),
                                     "[FAIL] Verification is already running in this repository working directory.")
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
        with self.holder():
            before = path.stat().st_size, path.stat().st_mtime_ns
            for args in ([], ["--help"], ["commit", "--plan"], ["check", "formatting", "--plan"]):
                with self.subTest(args=args):
                    result = subprocess.run([sys.executable, "-c", self.invocation(args)],
                                            cwd=self.root, capture_output=True, text=True, timeout=10)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual((path.stat().st_size, path.stat().st_mtime_ns), before)
            self.assertEqual(self.events(), [])
            self.assertFalse(self.directory.exists())
        self.assertEqual(path.read_bytes(), contents)
        path.unlink()
        for args in ([], ["--help"], ["commit", "--plan"]):
            subprocess.run([sys.executable, "-c", self.invocation(args)], cwd=self.root,
                           capture_output=True, check=True, timeout=10)
            self.assertFalse(path.exists())

    def test_normal_failure_and_forced_exit_release_persistent_lock(self):
        with self.holder():
            pass
        path = self.root / ".agent/tmp/verification.lock"
        path.write_text("Arbitrary contents are not ownership metadata.")
        for failed in (False, True):
            phases = [("Verification", [("Fixture", [sys.executable, "-c", f"raise SystemExit({int(failed)})"], {})])]
            result = subprocess.run([sys.executable, "-c", self.invocation(["check", "formatting"], phases)],
                                    cwd=self.root, capture_output=True, timeout=10)
            self.assertEqual(result.returncode, int(failed), result.stderr)
            with VerificationLock(self.root) as lock:
                self.assertFalse(os.get_inheritable(lock.handle.fileno()))
            self.assertEqual(path.read_text(), "Arbitrary contents are not ownership metadata.")
        result = subprocess.run([sys.executable, "-c", self.invocation(["check", "formatting"], [])],
                                cwd=self.root, capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        with VerificationLock(self.root):
            pass
        with self.holder() as process:
            process.kill()
            process.wait(timeout=5)
            with VerificationLock(self.root):
                self.assertTrue(path.exists())

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
        self.phases[1][1][0][1][-1] += (
            "\nimport os,time\n"
            "if not Path('.agent/tmp/ready').exists():\n"
            "    ready = Path('.agent/tmp/ready'); pending = ready.with_suffix('.pending')\n"
            "    pending.write_text(str(os.getpid())); pending.replace(ready)\n"
            "    deadline = time.monotonic() + 20\n"
            "    while not Path('.agent/tmp/release').exists() and time.monotonic() < deadline: time.sleep(0.02)\n")
        with tempfile.TemporaryFile() as log, subprocess.Popen(
                [sys.executable, "-c", self.invocation(["commit"])], cwd=self.root, stdout=log, stderr=log) as process:
            survivor = None
            try:
                self.wait_for(ready, process)
                survivor = psutil.Process(int(ready.read_text()))
                process.kill()
                process.wait(timeout=5)
                self.assertTrue(survivor.is_running())
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
        self.phases.insert(1, ("Build checks", [("Clippy Invariants", self.child("clippy"), {})]))
        self.write(self.words, "invalid\n")
        self.assertEqual(self.run_profile(), 1)
        self.assertEqual(self.events(), ["english"])
        self.assertEqual(self.run_profile(), 1)
        self.assertEqual(self.events(), ["english", "english"])
        self.write(self.words, "accepted\n")
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), ["english", "english", "english", "clippy", "coverage", "contracts"])

    def test_build_gate_is_separate_and_blocks_coverage_on_failure(self):
        _, args = verify.parse_args(["commit", "--plan"])
        with patch.object(commands, "select_rust_scope", return_value=RustScope(("config",), "fixture")):
            selected = commands.select_phases(args, {"crates/config/src/config.rs"})
        self.assertEqual([phase for phase, _ in selected][:3], ["Preflight", "Build checks", "Coverage"])
        self.assertEqual([name for name, _, _ in selected[1][1]],
                         ["Clippy Invariants", "WebAssembly build"])
        command = self.child("clippy")
        command[-1] += "sys.exit(1)"
        self.phases.insert(1, ("Build checks", [("Clippy Invariants", command, {})]))
        self.assertEqual(self.run_profile(), 1)
        self.assertEqual(self.events(), ["english", "clippy"])

    def test_failed_coverage_step_stops_export_but_keeps_other_commit_checks(self):
        names = ["prepare", "reset", "tests", "export", "summary"]
        for failing in names[:-1]:
            with self.subTest(failing=failing):
                items = [(name, self.child(name), {}) for name in names]
                items[names.index(failing)][1][-1] += "sys.exit(1)"
                self.phases = [("Coverage", items), ("Verification", [("Remaining gate", self.child("gate"), {})])]
                before = len(self.events())
                self.assertEqual(self.run_profile(extra=("--results-dir", f".agent/tmp/{failing}")), 1)
                self.assertEqual(self.events()[before:], names[:names.index(failing) + 1] + ["gate"])

    def test_failed_web_build_blocks_commit_and_cannot_be_reused_as_success(self):
        command = self.child("web")
        successful_script = command[-1]
        command[-1] += "sys.exit(1)"
        self.phases.insert(1, ("Build checks", [("WebAssembly build", command, {})]))
        self.assertEqual(self.run_profile(), 1)
        self.assertEqual(self.events(), ["english", "web"])
        self.assertEqual(self.record()["groups"]["WebAssembly build"]["status"], "failed")
        self.assertEqual(self.run_profile(), 1)
        self.assertEqual(self.events(), ["english", "web", "web"])
        command[-1] = successful_script
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), ["english", "web", "web", "web", "coverage", "contracts"])

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

    def test_changed_source_test_criterion_and_configuration_invalidate_execution(self):
        self.assertEqual(self.run_profile(), 0, self.output)
        for name in ("source.rs", "tests/assertions.rs", "tools/verification/checks/check_architecture.py",
                     "Cargo.toml", ".config/nextest.toml", "Obsidian/Amiga/Design/Contract.md", "unknown.input"):
            self.write(name, "new input\n")
            before = self.events().count("coverage")
            self.assertEqual(self.run_profile(), 0, self.output)
            self.assertEqual(self.events().count("coverage"), before + 1, name)

    def test_changed_execution_conditions_invalidate_all_groups(self):
        self.assertEqual(self.run_profile(), 0, self.output)
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

    def test_missing_and_corrupt_coverage_artifacts_require_the_whole_bundle(self):
        self.assertEqual(self.run_profile(), 0, self.output)
        for corrupt in (False, True):
            record = self.record()["groups"]["Coverage"]
            relative = next(name for name in record["artifacts"] if name.endswith("summary.json"))
            path = self.directory / relative
            path.write_text("corrupt") if corrupt else path.unlink()
            before = self.events().count("coverage")
            self.assertEqual(self.run_profile(), 0, self.output)
            self.assertEqual(self.events().count("coverage"), before + 1)

    def test_interrupted_group_and_new_command_cannot_reuse_success(self):
        self.assertEqual(self.run_profile(), 0, self.output)
        record = self.record()
        record["groups"]["Coverage"]["status"] = "running"
        (self.directory / "commit.json").write_text(json.dumps(record))
        self.phases[-1][1].append(("New required suite", self.child("new"), {}))
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), ["english", "coverage", "contracts", "coverage", "new"])

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
        self.write("docs/included.md", "Build or test input.\n")
        self.write("source.rs", 'const TEXT: &str = include_str!("docs/included.md");')
        self.assertEqual(self.run_profile(), 0, self.output)
        self.write("docs/included.md", "Changed input.\n")
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events().count("coverage"), 2)

    def test_missing_or_corrupt_record_requires_execution(self):
        self.assertEqual(self.run_profile(), 0, self.output)
        path = self.directory / "commit.json"
        for value in (None, "broken", "[]", '{"groups": []}'):
            with self.subTest(value=value):
                path.unlink() if value is None else path.write_text(value)
                before = len(self.events())
                self.assertEqual(self.run_profile(), 0, self.output)
                self.assertEqual(self.events()[before:], ["english", "coverage", "contracts"])
                self.assertTrue(all(item["status"] == "executed" for item in self.record()["results"].values()))

    def test_new_head_starts_fresh_even_when_input_contents_are_unchanged(self):
        self.assertEqual(self.run_profile(), 0, self.output)
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
        self.write(self.words, "invalid\n")
        for notice in ("[INFO] Stable update available. Run: rustup update stable\n",
                       "[INFO] Could not check Stable updates: timed out\n"):
            self.notice.return_value = notice
            self.assertEqual(self.run_profile(), 1, self.output)
            self.assertEqual(self.record()["outcome"], "failed")
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
                if failing:
                    left[-1] += "sys.exit(1)\n"
                self.phases = [
                    ("Preflight", [("Left gate", left, {}), ("Right gate", right, {})]),
                    ("Build checks", [("Build gate", self.child("build"), {})]),
                ]
                before = len(self.events())
                self.assertEqual(self.run_profile(("--results-dir", f".agent/tmp/parallel-{failing}")), int(failing), self.output)
                events = self.events()[before:]
                self.assertCountEqual(events[:2], ["left", "right"])
                self.assertEqual(events[2:], [] if failing else ["build"])

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
        self.write(name, json.dumps({"extensions": [".rs"], "file_words": {}}))
        self.assertEqual(self.run_profile(), 0, self.output)
        self.write(name, json.dumps({"extensions": [".rs"], "file_words": {"source.rs": ["literal"]}}))
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events().count("coverage"), 1)
        self.write(name, json.dumps({"extensions": [".py"], "file_words": {}}))
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events().count("coverage"), 2)


if __name__ == "__main__":
    unittest.main()

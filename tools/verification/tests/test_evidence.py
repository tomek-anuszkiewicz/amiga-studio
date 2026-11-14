"""Exercise batch continuation against real Git fixtures and child processes."""
import contextlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import evidence
import execution
import repository
import verify
import commands
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
        self.assertEqual(self.events(), ["english", "coverage", "contracts", "english", "contracts"])
        self.assertEqual(self.record()["results"]["Coverage"]["status"], "reused")
        self.assertEqual(reports, {p.name: p.read_bytes() for p in (self.root / ".agent/tmp/coverage").iterdir()})

    def test_unchanged_answer_runs_no_children(self):
        self.assertEqual(self.run_profile(), 0, self.output)
        events = self.events()
        self.assertEqual(self.run_profile(), 0, self.output)
        self.assertEqual(self.events(), events)

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
        self.assertEqual([name for name, _, _ in selected[1][1]], ["Clippy Invariants"])
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
                     "Cargo.toml", "Obsidian/Amiga/Design/Contract.md", "unknown.input"):
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
             patch.object(evidence.importlib.metadata, "distributions", return_value=[]), \
             patch.object(evidence, "file_digest", return_value="content"):
            original = evidence.execution_conditions(self.root)
            evidence.os.environ["CODEX_TOOL_CALL_ID"] = "second"
            self.assertEqual(evidence.execution_conditions(self.root), original)
            evidence.os.environ["RUSTFLAGS"] = "-C opt-level=1"
            self.assertNotEqual(evidence.execution_conditions(self.root), original)

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

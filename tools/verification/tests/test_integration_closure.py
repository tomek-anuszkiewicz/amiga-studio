"""Post-publication verification against independent temporary Git repositories."""
from hashlib import sha256
import contextlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import commands
import execution
import verify
from evidence import digest
from integration import accept_tested, cleanup_record, freeze, publish, read, seal
from repository import candidate_tree
from integration_cleanup import cleanup
from integration_closure import GRAPH_FILE, SCOPE_FILE, closure_path
from integration_state import save


TOOLS = Path(__file__).resolve().parents[2]
WORD_LIST = "tools/verification/checks/english_words/software.txt"
THREAD = "38e0b2b3-5101-4730-a1e0-6ea70126e200"
START = "2026-11-19T10:00:00Z"


def fixture_registration(common, task_id, execution_id):
    return {"schema_version": 1, "task_id": task_id, "execution_id": execution_id,
            "git_common_dir": str(common), "root_thread_id": THREAD, "start_utc": START}


class PublishedClosureTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        directory = Path(temporary.name).resolve()
        self.root = directory / "primary"
        self.source = directory / "task"
        self.root.mkdir()
        self.git(self.root, "init", "-b", "master")
        self.git(self.root, "config", "user.name", "Closure Trial")
        self.git(self.root, "config", "user.email", "trial@example.com")
        self.git(self.root, "config", "core.autocrlf", "false")
        self.write(self.root, ".gitignore", ".agent/\ngraphify-out\nref_src/\nignored.md\n")
        self.write(self.root, ".graphifyignore", "/tools/\n")
        # Imported test drivers are local fixture inputs, not authored trial files.
        self.write(self.root, ".git/info/exclude", "/tools/\n")
        for module in (TOOLS / "verification").glob("*.py"):
            self.write(self.root, "tools/verification/" + module.name, module.read_bytes())
        for name in ("check_english.py", "check_english.json", "english_words"):
            source = TOOLS / "verification/checks" / name
            target = self.root / "tools/verification/checks" / name
            if source.is_dir():
                shutil.copytree(source, target)
            else:
                self.write(self.root, target.relative_to(self.root).as_posix(), source.read_bytes())
        self.write(self.root, "tools/graphify/refresh.py", (TOOLS / "graphify/refresh.py").read_bytes())
        self.git(self.root, "add", "-f", SCOPE_FILE, WORD_LIST)
        self.write(self.root, "notes.md", "Initial task notes.\n")
        self.base = self.commit(self.root, "Establish closure trial")
        self.git(self.root, "worktree", "add", "-b", "task", str(self.source))
        for module in (TOOLS / "verification").glob("*.py"):
            self.write(self.source, "tools/verification/" + module.name, module.read_bytes())
        for module in (TOOLS / "workflow/capture").glob("*.py"):
            relative = module.relative_to(TOOLS).as_posix()
            for checkout in (self.root, self.source):
                self.write(checkout, "tools/" + relative, module.read_bytes())
        self.write(self.root, "ref_src/graphify-out/graph.json", b"reference")
        self.write(self.source, GRAPH_FILE, b"independent task graph")

    def git(self, root, *args):
        result = subprocess.run(["git", *args], cwd=root, capture_output=True, check=True)
        return result.stdout.decode("utf-8").strip()

    def write(self, root, name, content):
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content.encode("utf-8") if isinstance(content, str) else content)
        return path

    def commit(self, root, message):
        self.git(root, "add", "-A")
        self.git(root, "commit", "-m", message)
        return self.git(root, "rev-parse", "HEAD")

    def seal(self, extra=None):
        self.write(self.source, "accepted.py", "def accepted_closure_symbol():\n    return True\n")
        self.commit(self.source, "Add accepted project source")
        self.write(self.source, "DIARY.md", "Accepted project source and verification.\n")
        if extra:
            for name, content in extra.items():
                self.write(self.source, name, content)
        self.published = self.commit(self.source, "Record closure and test inputs")
        confirmation = self.source / ".agent/tmp/confirmation.json"
        save(confirmation, {"base": self.base, "commit": self.published,
                            "tree": self.git(self.source, "rev-parse", "HEAD^{tree}"),
                            "plan_approved": True, "tested": True,
                            "confirmation": "Fixture approval for the exact accepted source"})
        self.sealed = accept_tested(self.source, confirmation.parent, confirmation, "master")
        self.record = cleanup_record(read(self.sealed))
        self.closure = closure_path(read(self.sealed))

    def register_capture(self, execution_id="execution"):
        path = self.root / ".agent/task-archives/trial" / execution_id / "registration.json"
        save(path, fixture_registration(self.root / ".git", "trial", execution_id))
        return path

    def canonical_correction(self, *, linked):
        source = self.root.parent / "correction"
        self.git(self.root, "worktree", "add", "-b", "correction", str(source), self.published)
        self.write(source, "notes.md", "The task is supposed to succeed.\n")
        directory = source / ".agent/tmp/verification/commit"
        phases = [("Preflight", [("Fixture check", [sys.executable, "-c", "print('fixture checked')"], {})])]
        with (patch.object(verify, "REPO_ROOT", source), patch.object(execution, "REPO_ROOT", source),
              patch.object(verify, "execution_conditions", return_value="isolated conditions"),
              patch("integration.execution_conditions", return_value="isolated conditions"),
              patch.object(verify, "select_phases", return_value=phases),
              patch.object(commands, "select_phases", return_value=phases),
              patch.object(verify, "publish", return_value={}),
              patch.object(verify, "stable_update_notice", return_value=""),
              contextlib.redirect_stdout(io.StringIO()) as output):
            self.assertEqual(verify.main(["commit", "--integration-base", self.published]), 0, output.getvalue())
            evidence = read(directory / "commit.json")
            partition = [{"message": "Fix the reported English wording", "tree": candidate_tree(source)}]
            review = {"verdict": "APPROVED", "reviewer": "Independent fixture reviewer",
                      "source": evidence["source"], "integration_base": self.published,
                      "partition": digest(partition)}
            partition_path = directory / "partition.json"
            review_path = directory / "review.json"
            save(partition_path, partition)
            save(review_path, review)
            candidate = freeze(source, directory, review_path, partition_path, "master")
            self.commit(source, partition[0]["message"])
            seal(source, candidate)
            args = ["integrate", "publish", "record", str(candidate)]
            if linked:
                args += ["correction-for", str(self.record)]
            self.assertEqual(verify.main(args), 0, output.getvalue())
        return cleanup_record(read(candidate))

    def command(self, *args, root=None):
        cwd = root or self.root
        return subprocess.run([sys.executable, str(cwd / "tools/verification/verify.py"), *args],
                              cwd=cwd, capture_output=True, encoding="utf-8", errors="replace")

    def stage(self, action, *args, record=None):
        return self.command("integrate", action, "record", str(record or self.record), *args)

    def assert_status(self, result, expected):
        self.assertEqual(result.returncode, expected, result.stdout + result.stderr)

    def assert_published_open(self):
        self.assertEqual(self.git(self.root, "rev-parse", "HEAD"), self.published)
        self.assertEqual(self.git(self.root, "rev-parse", "refs/heads/task"), self.published)
        self.assertEqual(read(self.record)["state"], "published")
        self.assertTrue((self.source / ".git").is_file())
        self.assertFalse("graph" in read(self.closure))
        with self.assertRaisesRegex(ValueError, "requires completed closure English and project graph"):
            cleanup(self.root, self.record)

    def assert_graph(self, final):
        receipt = read(self.closure)
        self.assertEqual(receipt["graph"]["commit"], final)
        self.assertEqual(receipt["graph"]["path"], GRAPH_FILE)
        graph = (self.root / GRAPH_FILE).read_bytes()
        self.assertIn(b"accepted_closure_symbol", graph)
        self.assertEqual(receipt["graph"]["sha256"], sha256(graph).hexdigest())
        self.assertEqual((self.root / "ref_src/graphify-out/graph.json").read_bytes(), b"reference")
        self.assertEqual((self.source / GRAPH_FILE).read_bytes(), b"independent task graph")

    def logs(self, name):
        return list(self.closure.parent.glob(f"{name}-*.log"))

    def test_failed_publication_cannot_begin_closure(self):
        self.seal()
        self.assert_status(self.stage("closure-language", record=self.sealed), 1)
        self.write(self.root, "notes.md", "Unrelated master change.\n")
        advanced = self.commit(self.root, "Advance master outside the task")
        with self.assertRaisesRegex(ValueError, "Target advanced"):
            publish(self.source, self.sealed)
        self.assertEqual(self.git(self.root, "rev-parse", "HEAD"), advanced)
        self.assertFalse(self.closure.exists())
        self.assertFalse(self.logs("english"))
        self.assertFalse(self.logs("graph"))

    def test_wrong_primary_branch_or_sha_blocks_before_scanning(self):
        self.seal()
        publish(self.source, self.sealed)
        with self.assertRaisesRegex(ValueError, "recorded primary master checkout"):
            from integration_closure import scan_language
            scan_language(self.source, self.record)
        self.git(self.root, "switch", "-c", "other")
        result = self.stage("closure-language")
        self.assert_status(result, 1)
        self.assertIn("require local master", result.stdout)
        self.git(self.root, "switch", "master")
        self.write(self.root, "notes.md", "An unrelated follow-up.\n")
        self.commit(self.root, "Advance the published SHA")
        result = self.stage("closure-language")
        self.assert_status(result, 1)
        self.assertIn("differs from the recorded closure commit", result.stdout)
        self.assertFalse(self.closure.exists())

    def test_clean_closure_scans_once_then_refreshes_final_project_sources(self):
        self.seal()
        publish(self.source, self.sealed)
        self.assert_status(self.stage("closure-graph"), 1)
        self.assert_status(self.stage("cleanup"), 1)
        first = self.stage("closure-language")
        self.assert_status(first, 0)
        self.assertIn("0 unknown word occurrences; 0 read errors", first.stdout)
        self.assertIn("tools/verification/verify.py check english", first.stdout)
        self.assert_graph_after_refresh(self.published)
        self.assertEqual(self.git(self.root, "rev-parse", "HEAD"), self.published)
        self.assert_status(self.stage("closure-language"), 0)
        replay = self.stage("closure-graph")
        self.assert_status(replay, 0)
        self.assertIn("Reusing project graph", replay.stdout)
        self.assertEqual(len(self.logs("english")), 1)
        self.assertEqual(len(self.logs("graph")), 1)

    def assert_graph_after_refresh(self, final):
        self.assert_status(self.stage("closure-graph"), 0)
        self.assert_graph(final)

    def test_public_scanner_preserves_full_scope_and_explicit_file_selection(self):
        self.write(self.root, "notes.md", "worddztracked\n")
        self.commit(self.root, "Keep an unchanged scanner input")
        self.write(self.root, "new.md", "worddzuntracked\n")
        self.write(self.root, "ignored.md", "worddzignored\n")
        self.write(self.root, "scratch/notes.md", "worddzexcluded\n")
        self.write(self.root, ".agent/tasks/plan.md", "worddzplan\n")
        full = self.command("check", "english", "--log-level", "full")
        self.assert_status(full, 1)
        self.assertIn("[worddztracked]", full.stdout)
        self.assertIn("[worddzuntracked]", full.stdout)
        for word in ("worddzignored", "worddzexcluded", "worddzplan"):
            self.assertNotIn(f"[{word}]", full.stdout)
        focused = self.command("check", "english", "files", "notes.md", ".agent/tasks/plan.md")
        self.assert_status(focused, 1)
        self.assertIn("Line 1: [worddztracked]", focused.stdout)
        self.assertNotIn("[worddzplan]", focused.stdout)
        self.assertNotIn("[worddzuntracked]", focused.stdout)

    def vocabulary_trial(self):
        self.seal({"notes.md": "The accepted terminology includes deinstrumentation.\n",
                   "literal.py": "rejected = 'deliberatebadliteral'\n"})
        publish(self.source, self.sealed)
        self.assert_status(self.stage("closure-language"), 1)
        self.assertEqual({(item["path"], item["word"]) for item in read(self.closure)["language"]["findings"]},
                         {("notes.md", "deinstrumentation"), ("literal.py", "deliberatebadliteral")})
        choices = self.root / ".agent/tmp/additions.json"
        save(choices, {"word_lists": {"english_words/software.txt": ["deinstrumentation"]},
                       "file_words": {"literal.py": ["deliberatebadliteral"]}})
        return choices

    def test_vocabulary_only_commit_preserves_every_other_input_and_graph_uses_its_sha(self):
        choices = self.vocabulary_trial()
        before = self.git(self.root, "show", f"{self.published}:DIARY.md")
        self.assert_status(self.stage("closure-vocabulary", "additions", str(choices)), 0)
        final = self.git(self.root, "rev-parse", "HEAD")
        self.assertNotEqual(final, self.published)
        self.assertEqual(self.git(self.root, "show", "-s", "--format=%P", final), self.published)
        self.assertEqual(set(self.git(self.root, "diff", "--name-only", self.published, final).splitlines()),
                         {SCOPE_FILE, WORD_LIST})
        self.assertEqual(self.git(self.root, "show", f"{final}:DIARY.md"), before)
        config = json.loads(self.git(self.root, "show", f"{self.published}:{SCOPE_FILE}"))
        config["file_words"]["literal.py"] = ["deliberatebadliteral"]
        self.assertEqual(json.loads((self.root / SCOPE_FILE).read_text(encoding="utf-8")), config)
        old_words = subprocess.check_output(["git", "show", f"{self.published}:{WORD_LIST}"], cwd=self.root)
        newline = b"\r\n" if b"\r\n" in old_words else b"\n"
        self.assertEqual((self.root / WORD_LIST).read_bytes(), old_words + b"deinstrumentation" + newline)
        self.assertEqual(self.git(self.root, "rev-parse", "refs/heads/task"), self.published)
        self.assertEqual(self.git(self.root, "status", "--porcelain"), "")
        self.assert_graph_after_refresh(final)
        self.assert_status(self.stage("closure-vocabulary", "additions", str(choices)), 0)
        self.assert_status(self.stage("closure-language"), 0)
        self.assertEqual(len(self.logs("english")), 1)
        self.assertEqual(len(self.logs("graph")), 1)

    def test_vocabulary_cannot_commit_concurrent_staged_content(self):
        choices = self.vocabulary_trial()
        self.write(self.root, "notes.md", "Another writer's note.\n")
        self.git(self.root, "add", "notes.md")
        index = (self.root / ".git/index").read_bytes()
        result = self.stage("closure-vocabulary", "additions", str(choices))
        self.assert_status(result, 1)
        self.assertIn("pending changes", result.stdout)
        self.assertEqual((self.root / ".git/index").read_bytes(), index)
        self.assertEqual(self.git(self.root, "rev-parse", "HEAD"), self.published)
        self.assert_status(self.stage("closure-graph"), 1)

    def test_vocabulary_cannot_commit_after_an_external_master_advance(self):
        choices = self.vocabulary_trial()
        self.write(self.root, "notes.md", "Another writer's note.\n")
        advanced = self.commit(self.root, "Concurrent master change")
        result = self.stage("closure-vocabulary", "additions", str(choices))
        self.assert_status(result, 1)
        self.assertIn("differs from the recorded closure commit", result.stdout)
        self.assertEqual(self.git(self.root, "rev-parse", "HEAD"), advanced)
        self.assertEqual((self.root / "notes.md").read_text(), "Another writer's note.\n")
        self.assert_status(self.stage("closure-graph"), 1)

    def test_ambiguous_term_blocks_reviewed_additions_and_cleanup(self):
        self.seal({"notes.md": "Use deinstrumentation and wordingxunknown.\n"})
        publish(self.source, self.sealed)
        self.assert_status(self.stage("closure-language"), 1)
        language = read(self.closure)["language"]
        self.assertEqual(language["scanner_exit_code"], 1)
        self.assertIn({"path": "notes.md", "line": 1, "word": "wordingxunknown",
                       "context": "Use deinstrumentation and wordingxunknown."}, language["findings"])
        choices = self.root / ".agent/tmp/additions.json"
        save(choices, {"word_lists": {"english_words/software.txt": ["deinstrumentation"]}})
        result = self.stage("closure-vocabulary", "additions", str(choices))
        self.assert_status(result, 1)
        self.assertIn("Unresolved English findings", result.stdout)
        self.assert_published_open()

    def test_source_typo_remains_for_a_separate_scoped_correction(self):
        self.seal({"notes.md": "The task is suposed to succeed.\n"})
        publish(self.source, self.sealed)
        result = self.stage("closure-language")
        self.assert_status(result, 1)
        self.assertIn("Line 1: [suposed]", result.stdout)
        self.assert_published_open()
        original_language = read(self.closure)["language"]
        original_record = self.record.read_bytes()
        correction = self.canonical_correction(linked=True)
        final = read(correction)["commit"]
        self.assertNotEqual(final, self.published)
        self.assertEqual(read(correction)["base"], self.published)
        self.assertEqual(self.git(self.root, "rev-parse", "HEAD"), final)
        self.assert_status(self.stage("closure-language"), 1)
        self.assert_status(self.stage("closure-graph"), 1)
        self.assert_status(self.stage("closure-language", "correction", str(correction)), 0)
        updated = read(self.closure)
        self.assertEqual(updated["published_commit"], self.published)
        self.assertEqual(updated["language_history"], [original_language])
        self.assertEqual(updated["language"]["scan_commit"], final)
        self.assertEqual(updated["language"]["correction"], correction.parent.name)
        self.assertEqual(self.record.read_bytes(), original_record)
        self.assert_graph_after_refresh(final)
        self.assert_status(self.stage("closure-language", "correction", str(correction)), 0)
        self.assert_status(self.stage("closure-graph"), 0)
        self.assertEqual(len(self.logs("english")), 2)
        self.assertEqual(len(self.logs("graph")), 1)

    def test_unlinked_canonical_advance_cannot_resume_unresolved_closure(self):
        self.seal({"notes.md": "The task is suposed to succeed.\n"})
        publish(self.source, self.sealed)
        self.assert_status(self.stage("closure-language"), 1)
        correction = self.canonical_correction(linked=False)
        result = self.stage("closure-language", "correction", str(correction))
        self.assert_status(result, 1)
        self.assertIn("was not linked", result.stdout)
        self.assertEqual(len(self.logs("english")), 1)
        self.assert_status(self.stage("closure-graph"), 1)
        self.assert_status(self.stage("cleanup"), 1)
        self.assertTrue(self.source.is_dir())

    def test_scan_read_error_retains_distinct_child_status_and_blocks_closure(self):
        self.seal({"invalid.md": b"\xff"})
        publish(self.source, self.sealed)
        result = self.stage("closure-language")
        self.assert_status(result, 1)
        self.assertIn("Cannot read", result.stdout)
        self.assertIn("invalid.md", result.stdout)
        language = read(self.closure)["language"]
        self.assertEqual((language["exit_code"], language["scanner_exit_code"]), (1, 2))
        self.assert_published_open()

    def test_resolved_scanner_read_error_can_retry_at_the_same_master_sha(self):
        self.seal()
        publish(self.source, self.sealed)
        bad = self.write(self.root, "invalid.md", b"\xff")
        failed = self.stage("closure-language")
        self.assert_status(failed, 1)
        self.assertIn("Cannot read", failed.stdout)
        previous = read(self.closure)["language"]
        failed_log = self.closure.parent / previous["log"]
        failed_bytes = failed_log.read_bytes()
        bad.write_bytes(b"Ordinary English notes.\n")
        self.assert_status(self.stage("closure-language"), 1)
        self.assertEqual(len(self.logs("english")), 1)
        self.assert_status(self.stage("closure-language", "retry", "blocked"), 0)
        self.assertEqual(read(self.closure)["language_history"], [previous])
        self.assertEqual(failed_log.read_bytes(), failed_bytes)
        bad.unlink()
        self.assert_graph_after_refresh(self.published)
        self.assert_status(self.stage("closure-language", "retry", "blocked"), 0)
        self.assert_status(self.stage("closure-graph"), 0)
        self.assertEqual(len(self.logs("english")), 2)
        self.assertEqual(len(self.logs("graph")), 1)

    def test_graph_failure_retries_only_graph_and_reuses_a_successful_receipt(self):
        self.seal()
        publish(self.source, self.sealed)
        self.assert_status(self.stage("closure-language"), 0)
        blocked = self.write(self.root, "graphify-out", b"blocks output")
        failed = self.stage("closure-graph")
        self.assertNotEqual(failed.returncode, 0, failed.stdout)
        self.assertIn("graphify-out", failed.stdout)
        self.assertIn("Graphify refresh failed:", failed.stdout)
        self.assert_published_open()
        self.assertEqual(read(self.closure)["graph_attempt"]["state"], "failed")
        blocked.unlink()
        self.write(self.root, "accepted.py", "def unrelated_change():\n    return None\n")
        dirty = self.stage("closure-graph")
        self.assert_status(dirty, 1)
        self.assertIn("pending changes", dirty.stdout)
        self.assertEqual(len(self.logs("graph")), 1)
        self.git(self.root, "restore", "accepted.py")
        saved = read(self.closure)
        save(self.closure, {**saved, "publication": "different"})
        self.assert_status(self.stage("closure-graph"), 1)
        self.assertEqual(len(self.logs("graph")), 1)
        save(self.closure, saved)
        self.assert_graph_after_refresh(self.published)
        self.assert_status(self.stage("closure-graph"), 0)
        self.assertEqual(len(self.logs("english")), 1)
        self.assertEqual(len(self.logs("graph")), 2)

    def test_cleanup_requires_the_same_completed_preservation_before_removing_worktree(self):
        self.seal()
        self.write(self.root, "tools/worktree.ps1", (TOOLS / "worktree.ps1").read_bytes())
        active = self.write(self.source, ".agent/tmp/worker.jsonl", b"partial")
        main = self.write(self.source, ".agent/tmp/main.jsonl", b"main result\r\n\xff")
        selection = self.root / ".agent/tmp/files.json"
        choices = {"files": [{"id": "main", "source": str(main), "path": "main.jsonl", "complete": True},
                             {"id": "worker", "source": str(active), "path": "worker.jsonl", "complete": False}]}
        save(selection, choices)
        archive = self.root / ".agent/task-archives/trial/execution"

        def preserve(publication=None, execution_id="execution"):
            return subprocess.run([sys.executable, str(TOOLS / "workflow/capture/records.py"), "preserve",
                                   "--primary", str(self.root), "--task-id", "trial", "--execution-id", execution_id,
                                   "--input", str(selection), "--publication", str(publication or self.record)],
                                  cwd=self.source, capture_output=True, encoding="utf-8")

        self.assert_status(preserve(self.sealed), 2)
        self.assertFalse(archive.exists())
        original = self.git(self.root, "rev-parse", "HEAD")
        failed = self.command("integrate", "publish", "record", str(self.sealed),
                              "task", "trial", "execution", "typo", root=self.source)
        self.assert_status(failed, 1)
        self.assertIn("existing capture registration", failed.stdout)
        self.assertEqual(self.git(self.root, "rev-parse", "HEAD"), original)
        self.assertFalse(self.record.exists())
        registration = self.register_capture()
        related = self.register_capture("other")
        accepted = self.command("integrate", "publish", "record", str(self.sealed),
                                "task", "trial", "execution", "execution", root=self.source)
        self.assert_status(accepted, 0)
        self.assert_status(self.stage("closure-language"), 0)
        self.assert_status(preserve(), 2)
        self.assertFalse((archive / "main.jsonl").exists())
        self.assert_graph_after_refresh(self.published)
        blocked = self.stage("cleanup")
        self.assert_status(blocked, 1)
        self.assertIn("requires completed preservation", blocked.stdout)
        self.assertTrue(self.source.is_dir())
        for wrong in ("typo", "other"):
            failure = preserve(execution_id=wrong)
            self.assert_status(failure, 2)
            self.assertIn("registered for this publication", failure.stderr)
        self.assertFalse((archive.parent / "typo").exists())
        self.assertEqual(list(related.parent.iterdir()), [related])
        self.assertNotIn("preservation", read(self.closure))
        partial = preserve()
        self.assert_status(partial, 2)
        self.assertIn("producer is still active", partial.stderr)
        self.assertEqual((archive / "main.jsonl").read_bytes(), main.read_bytes())
        self.assertFalse((archive / "worker.jsonl").exists())
        self.assert_status(self.stage("cleanup"), 1)
        self.assertTrue((self.source / ".git").is_file())
        saved = choices["files"].pop()
        save(selection, choices)
        self.assert_status(preserve(), 2)
        self.assert_status(self.stage("cleanup"), 1)
        choices["files"].append(saved)
        saved["complete"] = True
        self.write(self.source, ".agent/tmp/worker.jsonl", b"partial final tail")
        save(selection, choices)
        complete = preserve()
        self.assert_status(complete, 0)
        self.assertIn("[RETAINED] main", complete.stdout)
        self.assertEqual({path.name for path in archive.iterdir()}, {"main.jsonl", "worker.jsonl", "registration.json"})
        self.assertTrue(read(self.closure)["preservation"]["complete"])
        self.assertEqual(len(self.logs("english")), 1)
        self.assertEqual(len(self.logs("graph")), 1)
        original_registration = read(registration)
        save(registration, {**original_registration, "start_utc": "2026-11-19T10:00:01Z"})
        refused = self.stage("cleanup")
        self.assert_status(refused, 1)
        self.assertIn("Capture registration changed", refused.stdout)
        self.assertTrue(self.source.is_dir())
        save(registration, original_registration)
        self.assert_status(self.stage("cleanup"), 0)
        self.assertFalse(self.source.exists())
        self.assertNotIn(str(self.source), self.git(self.root, "worktree", "list", "--porcelain"))
        self.assertEqual(self.git(self.root, "branch", "--list", "task"), "")
        self.assertEqual((archive / "worker.jsonl").read_bytes(), b"partial final tail")

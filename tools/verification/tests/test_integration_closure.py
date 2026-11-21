"""Post-publication verification against independent temporary Git repositories."""
from hashlib import sha256
import contextlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from git_fixtures import restore_repository

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import commands
import execution
import verify
from evidence import digest, file_digest
from integration import accept_tested, cleanup_record, freeze, publish, read, seal
from repository import candidate_tree
from integration_cleanup import cleanup
from integration_closure import GRAPH_FILE, SCOPE_FILE, closure_path
from integration_state import save


TOOLS = Path(__file__).resolve().parents[2]
CLOSURE_ARCHIVE = Path(__file__).with_name("fixtures") / "closure-repositories.zip"
STARTING_FIXTURES = {
    "test_failed_publication_cannot_begin_closure": "clean",
    "test_wrong_primary_branch_or_sha_blocks_before_scanning": "clean",
    "test_clean_closure_scans_once_then_refreshes_final_project_sources": "clean",
    "test_short_route_blocks_unchanged_typo_then_publishes_the_verified_correction": "typo",
    "test_task_vocabulary_and_exact_file_corrections_require_a_new_receipt": "vocabulary",
    "test_publisher_rejects_changed_scan_inputs_and_missing_log_at_the_same_commit": "clean",
    "test_divergent_and_combined_candidates_require_a_full_scan_of_the_final_merge": "divergent",
    "test_resolved_scanner_read_error_can_retry_at_the_same_master_sha": "clean",
    "test_graph_failure_retries_only_graph_and_reuses_a_successful_receipt": "clean",
    "test_cleanup_requires_the_same_completed_preservation_before_removing_worktree": "clean",
}
WORD_LIST = "tools/verification/checks/english_words/software.txt"
MISSPELLED_NOTES = "The task is suposed to succeed.\n"
REVIEWED_TERM = "deinstrumentation"
EXACT_LITERAL = "deliberatebadliteral"
INVALID_TEXT = b"\xff"
THREAD = "38e0b2b3-5101-4730-a1e0-6ea70126e200"
START = "2026-11-19T10:00:00Z"


def fixture_registration(common, task_id, execution_id):
    return {"schema_version": 1, "task_id": task_id, "execution_id": execution_id,
            "git_common_dir": str(common), "root_thread_id": THREAD, "start_utc": START}


class PublishedClosureTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.configure(Path(temporary.name).resolve())
        fixture = STARTING_FIXTURES.get(self._testMethodName)
        if fixture is None:
            self.initialize_repository()
        else:
            self.sealed = self.restore_sealed(fixture)
            self.remember_sealed()
            self.install_tools(self.root)
            self.install_tools(self.source)

    def configure(self, directory):
        self.root = directory / "primary"
        self.source = directory / "task"

    def initialize_repository(self):
        self.root.mkdir()
        self.git(self.root, "init", "--template=", "-b", "master")
        self.git(self.root, "config", "user.name", "Closure Trial")
        self.git(self.root, "config", "user.email", "trial@example.com")
        self.git(self.root, "config", "core.autocrlf", "false")
        self.write(self.root, ".gitignore", ".agent/\ngraphify-out\nref_src/\nignored.md\n")
        self.write(self.root, ".graphifyignore", "/tools/\n")
        # Imported test drivers are local fixture inputs, not authored trial files.
        self.write(self.root, ".git/info/exclude", "/tools/\n")
        for name in (SCOPE_FILE, WORD_LIST):
            self.write(self.root, name, (TOOLS.parent / name).read_bytes())
        self.git(self.root, "add", "-f", SCOPE_FILE, WORD_LIST)
        self.write(self.root, "notes.md", "Initial task notes.\n")
        self.base = self.commit(self.root, "Establish closure trial")
        self.git(self.root, "worktree", "add", "-b", "task", str(self.source))
        self.install_tools(self.root)
        self.install_tools(self.source)
        self.write(self.root, "ref_src/graphify-out/graph.json", b"reference")
        self.write(self.source, GRAPH_FILE, b"independent task graph")

    def install_tools(self, checkout):
        # The tracked scope and software vocabulary belong to the fixture commit.
        for pattern in ("verification/*.py", "verification/checks/check_english.py",
                        "verification/checks/english_words/*.txt", "workflow/capture/*.py"):
            for tool in TOOLS.glob(pattern):
                relative = "tools/" + tool.relative_to(TOOLS).as_posix()
                if relative != WORD_LIST:
                    self.write(checkout, relative, tool.read_bytes())
        if checkout == self.root:
            self.write(checkout, "tools/graphify/refresh.py", (TOOLS / "graphify/refresh.py").read_bytes())

    def restore_sealed(self, name, *, advance=False):
        parent = self.root.parent
        restore_repository(name, parent, CLOSURE_ARCHIVE)
        path = self.source / f".agent/tmp/sealed-{name}.json"
        frozen = read(path)
        if advance:
            self.git(self.source, "merge", "--ff-only", frozen["commit"])
        for key in ("source_root", "target_root"):
            frozen[key] = str(parent / frozen[key])
        for source in frozen["sources"]:
            source["root"] = str(parent / source["root"])
        if frozen["verification"] == "confirmed-fast-forward":
            frozen["confirmation_path"] = str(parent / frozen["confirmation_path"])
        else:
            frozen["evidence_directory"] = str(parent / frozen["evidence_directory"])
            frozen["evidence"]["checkout"] = digest(str(self.source))
            artifacts = {str(parent / name): identity for name, identity in frozen["review_artifacts"].items()}
            if frozen["sources_explicit"]:
                frozen["review"]["sources"] = digest(frozen["sources"])
                for artifact, data in (("review.json", frozen["review"]), ("sources.json", frozen["sources"])):
                    rebound = Path(frozen["evidence_directory"]) / artifact
                    save(rebound, data)
                    artifacts[str(rebound)] = file_digest(rebound)
            frozen["review_artifacts"] = artifacts
            save(Path(frozen["evidence_directory"]) / "commit.json", frozen["evidence"])
        save(path, frozen)
        return path

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
        self.remember_sealed()

    def remember_sealed(self):
        frozen = read(self.sealed)
        self.base = frozen["base"]
        self.published = frozen["commit"]
        self.record = cleanup_record(frozen)
        self.closure = closure_path(frozen)

    def register_capture(self, execution_id="execution"):
        path = self.root / ".agent/task-archives/trial" / execution_id / "registration.json"
        save(path, fixture_registration(self.root / ".git", "trial", execution_id))
        return path

    def record_historical_publication(self):
        """Restore the version-two state previously left by a successful ref update."""
        self.git(self.root, "merge", "--ff-only", self.published)
        published = {**read(self.sealed), "state": "published"}
        save(self.record, published)
        save(self.sealed, published)

    def canonical_correction(self, *, linked):
        source = self.root.parent / "correction"
        self.git(self.root, "worktree", "add", "-b", "correction", str(source), self.published)
        self.install_scan_tools(source)
        self.write(source, "notes.md", "The task is supposed to succeed.\n")
        candidate = self.seal_canonical(source, self.published, "Fix the reported English wording")
        self.assert_status(self.canonical_stage(source, candidate, "scan-english"), 0)
        args = ["integrate", "publish", "record", str(candidate)]
        if linked:
            args += ["correction-for", str(self.record)]
        with patch("integration.execution_conditions", return_value="isolated conditions"), \
                patch.object(verify, "REPO_ROOT", source), contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(verify.main(args), 0, output.getvalue())
        return cleanup_record(read(candidate))

    def install_scan_tools(self, source):
        tools = self.source / "tools/verification"
        for module in (*tools.glob("*.py"), *tools.glob("checks/check_english.py"), *tools.glob("checks/english_words/*.txt")):
            relative = module.relative_to(self.source)
            if not (source / relative).exists():
                self.write(source, relative, module.read_bytes())

    def seal_canonical(self, source, base, message, sources=None):
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
            self.assertEqual(verify.main(["commit", "--integration-base", base]), 0, output.getvalue())
            evidence = read(directory / "commit.json")
            partition = [] if self.git(source, "status", "--porcelain") == "" else [
                {"message": message, "tree": candidate_tree(source)}]
            review = {"verdict": "APPROVED", "reviewer": "Independent fixture reviewer",
                      "source": evidence["source"], "integration_base": base,
                      "partition": digest(partition)}
            partition_path = directory / "partition.json"
            review_path = directory / "review.json"
            sources_path = None
            if sources is not None:
                review["sources"] = digest(sources)
                sources_path = directory / "sources.json"
                save(sources_path, sources)
            save(partition_path, partition)
            save(review_path, review)
            candidate = freeze(source, directory, review_path, partition_path, "master", sources_path)
            if partition:
                self.commit(source, message)
            seal(source, candidate)
        return candidate

    def command(self, *args, root=None):
        cwd = root or self.root
        return subprocess.run([sys.executable, str(cwd / "tools/verification/verify.py"), *args],
                              cwd=cwd, capture_output=True, encoding="utf-8", errors="replace")

    def stage(self, action, *args, record=None):
        return self.command("integrate", action, "record", str(record or self.record), *args)

    def source_stage(self, action, *args, record=None):
        return self.command("integrate", action, "record", str(record or self.sealed), *args, root=self.source)

    def canonical_stage(self, source, candidate, action):
        arguments = ["integrate", action, "record", str(candidate)]
        with (patch.object(verify, "REPO_ROOT", source),
              patch("integration.execution_conditions", return_value="isolated conditions"),
              contextlib.redirect_stdout(io.StringIO()) as output):
            status = verify.main(arguments)
        return subprocess.CompletedProcess(arguments, status, output.getvalue(), "")

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
        self.record_historical_publication()
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
        self.assert_status(self.stage("closure-graph"), 1)
        self.assert_status(self.stage("cleanup"), 1)
        first = self.source_stage("scan-english")
        self.assert_status(first, 0)
        self.assertIn("0 unknown word occurrences; 0 read errors", first.stdout)
        self.assertIn("tools/verification/verify.py check english", first.stdout)
        self.assert_status(self.source_stage("publish"), 0)
        retained = read(self.record)["english"]
        self.assertEqual(retained["commit"], self.published)
        self.assert_graph_after_refresh(self.published)
        self.assertEqual(self.git(self.root, "rev-parse", "HEAD"), self.published)
        self.assertEqual(read(self.closure)["language"]["receipt"], retained)
        self.assert_status(self.stage("closure-language"), 1)
        historical = self.stage("closure-vocabulary", "additions", ".agent/tmp/additions.json")
        self.assert_status(historical, 1)
        self.assertIn("already carries its passing English receipt", historical.stdout)
        replay = self.stage("closure-graph")
        self.assert_status(replay, 0)
        self.assertIn("Reusing project graph", replay.stdout)
        self.assertEqual(len(self.logs("english")), 1)
        self.assertEqual(len(self.logs("graph")), 1)

    def assert_unpublished(self, base, candidate):
        self.assertEqual(self.git(self.root, "rev-parse", "refs/heads/master"), base)
        frozen = read(candidate)
        self.assertEqual(frozen["state"], "sealed")
        self.assertFalse(cleanup_record(frozen).exists())
        self.assertFalse((self.root / ".git/local-integration.json").exists())

    def test_short_route_blocks_unchanged_typo_then_publishes_the_verified_correction(self):
        self.assert_status(self.source_stage("publish"), 1)
        self.assert_unpublished(self.base, self.sealed)
        result = self.source_stage("scan-english")
        self.assert_status(result, 1)
        self.assertIn("[suposed]", result.stdout)
        self.assertEqual(self.git(self.source, "diff", "--name-only", self.base, self.published), "DIARY.md\naccepted.py")
        old_receipt_path = self.record.with_name("english.json")
        old_receipt = read(old_receipt_path)
        self.assertEqual((old_receipt["state"], old_receipt["exit_code"], old_receipt["scanner_exit_code"]),
                         ("blocked", 1, 1))
        self.assert_status(self.source_stage("publish"), 1)
        self.assert_unpublished(self.base, self.sealed)

        candidate = self.restore_sealed("typo-corrected", advance=True)
        self.assert_status(self.canonical_stage(self.source, candidate, "publish"), 1)
        self.assert_unpublished(self.base, candidate)
        self.assert_status(self.canonical_stage(self.source, candidate, "scan-english"), 0)
        final = read(candidate)["commit"]
        self.assert_status(self.canonical_stage(self.source, candidate, "publish"), 0)
        self.assertEqual(self.git(self.root, "rev-parse", "refs/heads/master"), final)
        self.assertEqual(read(old_receipt_path), old_receipt)

    def test_task_vocabulary_and_exact_file_corrections_require_a_new_receipt(self):
        self.assert_status(self.source_stage("scan-english"), 1)
        self.assert_status(self.source_stage("publish"), 1)
        self.assert_unpublished(self.base, self.sealed)

        candidate = self.restore_sealed("vocabulary-corrected", advance=True)
        self.assert_status(self.canonical_stage(self.source, candidate, "publish"), 1)
        self.assert_unpublished(self.base, candidate)
        self.assert_status(self.canonical_stage(self.source, candidate, "scan-english"), 0)
        frozen = read(candidate)
        self.assert_status(self.canonical_stage(self.source, candidate, "publish"), 0)
        self.assertEqual(self.git(self.root, "rev-parse", "refs/heads/master"), frozen["commit"])
        self.assertEqual(set(self.git(self.root, "diff", "--name-only", self.published, frozen["commit"]).splitlines()),
                         {SCOPE_FILE, WORD_LIST})

    def test_publisher_rejects_changed_scan_inputs_and_missing_log_at_the_same_commit(self):
        self.assert_status(self.source_stage("scan-english"), 0)
        receipt_path = self.record.with_name("english.json")
        receipt = read(receipt_path)
        log = receipt_path.parent / receipt["log"]
        saved = log.read_bytes()
        log.unlink()
        missing = self.source_stage("publish")
        self.assert_status(missing, 1)
        self.assertIn("log is missing or changed", missing.stdout)
        self.assert_unpublished(self.base, self.sealed)
        log.write_bytes(saved)

        # This fixture vocabulary file remains ignored and changes outside the sealed tree.
        vocabulary = self.source / "tools/verification/checks/english_words/hardware.txt"
        original = vocabulary.read_bytes()
        vocabulary.write_bytes(original + b"\ninvalid vocabulary entry\n")
        stale = self.source_stage("publish")
        self.assert_status(stale, 1)
        self.assertIn("scan inputs changed", stale.stdout)
        self.assert_unpublished(self.base, self.sealed)
        failed = self.source_stage("scan-english")
        self.assert_status(failed, 1)
        self.assertIn("Cannot prepare English scan", failed.stdout)
        blocked_receipt = read(receipt_path)
        self.assertEqual(blocked_receipt["scanner_exit_code"], 2)
        vocabulary.write_bytes(original + b"\n# Reviewed vocabulary.\n")
        self.assert_status(self.source_stage("scan-english"), 0)
        self.assertNotEqual(read(receipt_path)["inputs"]["vocabulary"], receipt["inputs"]["vocabulary"])
        self.assertEqual(read(receipt_path.parent / receipt["attempt"]), receipt)
        self.assertEqual(read(receipt_path.parent / blocked_receipt["attempt"]), blocked_receipt)
        self.assert_status(self.source_stage("publish"), 0)

    def test_divergent_and_combined_candidates_require_a_full_scan_of_the_final_merge(self):
        candidate = self.sealed
        divergent = self.git(self.source, "rev-parse", "refs/heads/divergent-task")
        base = self.base
        self.assert_status(self.canonical_stage(self.source, candidate, "publish"), 1)
        self.assert_unpublished(base, candidate)
        self.assert_status(self.canonical_stage(self.source, candidate, "scan-english"), 0)
        first = read(candidate)
        passing = read(cleanup_record(first).with_name("english.json"))
        self.assertEqual(passing["commit"], first["commit"])
        self.git(self.root, "merge", "--ff-only", "refs/heads/next-master")
        base = self.git(self.root, "rev-parse", "HEAD")
        moved = self.canonical_stage(self.source, candidate, "publish")
        self.assert_status(moved, 1)
        self.assertIn("Target advanced", moved.stdout)
        self.assert_unpublished(base, candidate)
        candidate = self.restore_sealed("combined", advance=True)
        other = self.root.parent / "other"
        second = self.git(other, "rev-parse", "HEAD")
        sources = [{"ref": "refs/heads/task", "root": str(self.source), "tip": divergent},
                   {"ref": "refs/heads/other", "root": str(other), "tip": second}]
        combined = cleanup_record(read(candidate)).with_name("english.json")
        self.assert_status(self.canonical_stage(self.source, candidate, "publish"), 1)
        self.assert_unpublished(base, candidate)
        save(combined, passing)
        self.assert_status(self.canonical_stage(self.source, candidate, "publish"), 1)
        self.assert_unpublished(base, candidate)
        failed = self.canonical_stage(self.source, candidate, "scan-english")
        self.assert_status(failed, 1)
        self.assertIn("Cannot read", failed.stdout)
        failure_receipt = read(combined)
        self.assertEqual((failure_receipt["state"], failure_receipt["scanner_exit_code"]), ("blocked", 2))
        self.assert_status(self.canonical_stage(self.source, candidate, "publish"), 1)
        self.assert_unpublished(base, candidate)

        candidate = self.restore_sealed("combined-corrected", advance=True)
        self.assert_status(self.canonical_stage(self.source, candidate, "scan-english"), 0)
        frozen = read(candidate)
        self.assert_status(self.canonical_stage(self.source, candidate, "publish"), 0)
        self.assertEqual(self.git(self.root, "rev-parse", "refs/heads/master"), frozen["commit"])
        self.assertEqual(self.git(self.root, "reflog", "-1", "--format=%gs", "master"),
                         "local integration " + base + " -> " + frozen["commit"])
        self.assertEqual(read(cleanup_record(frozen))["sources"], sources)

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
        self.record_historical_publication()
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
        self.record_historical_publication()
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
        self.record_historical_publication()
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
        self.assertEqual(len(self.logs("english")), 1)
        self.assertEqual(len(list(correction.parent.glob("english-*.log"))), 1)
        self.assertEqual(len(self.logs("graph")), 1)

    def test_unlinked_canonical_advance_cannot_resume_unresolved_closure(self):
        self.seal({"notes.md": "The task is suposed to succeed.\n"})
        self.record_historical_publication()
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
        self.record_historical_publication()
        result = self.stage("closure-language")
        self.assert_status(result, 1)
        self.assertIn("Cannot read", result.stdout)
        self.assertIn("invalid.md", result.stdout)
        language = read(self.closure)["language"]
        self.assertEqual((language["exit_code"], language["scanner_exit_code"]), (1, 2))
        self.assert_published_open()

    def test_resolved_scanner_read_error_can_retry_at_the_same_master_sha(self):
        self.record_historical_publication()
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
        self.assert_status(self.source_stage("scan-english"), 0)
        self.assert_status(self.source_stage("publish"), 0)
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
        self.assert_status(self.source_stage("scan-english"), 0)
        accepted = self.command("integrate", "publish", "record", str(self.sealed),
                                "task", "trial", "execution", "execution", root=self.source)
        self.assert_status(accepted, 0)
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

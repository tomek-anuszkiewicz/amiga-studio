"""Language scan scope and vocabulary boundaries."""

import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "checks"))
import check_english as english


REPORT_WORD_USAGE = Path(".agents/skills/TA-WF-Step-report-word-usage")
REPORT_WORD_LISTS = (
    REPORT_WORD_USAGE / "assets/common-words.txt",
    REPORT_WORD_USAGE / "assets/project-words.txt",
)


class EnglishScope(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name).resolve()

    def write(self, name, content):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        return path

    def scan(self, files, scope=None, workers=1):
        output = io.StringIO()
        scope_file = english.SCOPE_FILE
        if scope is not None:
            scope_file = self.write("scope.json", json.dumps(scope))
        with patch.object(english, "REPO_ROOT", self.root), \
             patch.object(english, "SCOPE_FILE", scope_file), \
             patch.object(english, "worker_count", return_value=workers), \
             patch.multiple(english, WORKER_SCAN=None, WORKER_DICTIONARY=None), \
             patch.object(sys, "argv", ["check_english.py", *map(str, files)]), \
             contextlib.chdir(self.root), contextlib.redirect_stdout(output), \
             contextlib.redirect_stderr(output):
            result = english.main()
        return result, output.getvalue()

    def test_explicit_task_paths_are_ignored(self):
        task = self.write(".agent/tasks/nested/plan.md", "wrong" * 8)
        for path in (task, task.relative_to(self.root),
                     self.root / ".agent/tasks/../tasks/nested/plan.md"):
            with self.subTest(path=path):
                result, output = self.scan([path])
                self.assertEqual(result, 0, output)
                self.assertIn("English scan: 0 files;", output)

    def test_repository_scan_ignores_task_plans(self):
        self.git("init", "--quiet")
        self.configure([])
        self.write(".agent/tasks/plan.md", "wrong" * 8)
        self.write("docs/example.md", "Valid English text.")
        with patch.object(english, "SCOPE_FILE", self.root / "scope.json"):
            result, output = self.scan([])
        self.assertEqual(result, 0, output)
        self.assertIn("English scan: 1 files;", output)

    def test_neighboring_paths_and_other_explicit_exclusions_still_scan(self):
        task = self.write(".agent/tasks/plan.md", "wrong" * 8)
        for name in (".agent/tasks-other/example.md", ".agent/tmp/example.md"):
            document = self.write(name, "wrong" * 8)
            with self.subTest(path=name):
                result, output = self.scan([task, document])
                self.assertEqual(result, 1, output)
                self.assertIn(str(document), output)
                self.assertNotIn(str(task), output)
                self.assertIn("English scan: 1 files;", output)

    def test_repository_discovery_keeps_authored_files_and_excludes_data(self):
        subprocess.run(["git", "init", "--quiet", str(self.root)], check=True, capture_output=True)
        self.write(".gitignore", "ignored/\n")
        tracked = self.write("src/tracked.rs", "Valid English text.")
        for relative in REPORT_WORD_LISTS:
            self.write(relative, "wrong" * 8)
        subprocess.run(["git", "-C", str(self.root), "add", "src/tracked.rs",
                        REPORT_WORD_LISTS[0].as_posix()], check=True, capture_output=True)
        new = self.write("docs/new.md", "Valid English text.")
        neighbor = self.write("ref_src-other/example.md", "Valid English text.")
        report = self.write(REPORT_WORD_USAGE / "SKILL.md", "Valid English text.")
        asset = self.write(REPORT_WORD_USAGE / "assets/notes.txt", "Valid English text.")
        same_name = self.write("docs/project-words.txt", "Valid English text.")
        for name in ("ref_src/example.md", "nested/target/example.rs", "graphify-out/example.json",
                     "tools/bootstrap/books/example.md", ".agent/tasks/plan.md",
                     "ignored/example.md", "docs/package.bin", "docs/Cargo.lock"):
            self.write(name, "wrong" * 8)
        scope = json.loads(english.SCOPE_FILE.read_text(encoding="utf-8"))
        with patch.object(english, "REPO_ROOT", self.root):
            targets = english.repository_targets(scope)
        self.assertEqual(set(targets), {self.root / ".gitignore", tracked, new, neighbor,
                                       report, asset, same_name})

    def test_explicit_report_word_lists_are_ignored(self):
        paths = []
        for relative in REPORT_WORD_LISTS:
            target = self.write(relative, "wrong" * 8)
            paths.extend((target, relative, relative.parent / ".." / "assets" / relative.name))
        result, output = self.scan(paths)
        self.assertEqual(result, 0, output)
        self.assertIn("English scan: 0 files;", output)

    def test_report_word_lists_do_not_supply_accepted_vocabulary(self):
        terms = ("wrong" * 8, "wrong" * 9)
        for relative, term in zip(REPORT_WORD_LISTS, terms):
            self.write(relative, term)
        scope_text = english.SCOPE_FILE.read_text(encoding="utf-8")
        scope_file = self.write(english.SCOPE_FILE.relative_to(english.REPO_ROOT), scope_text)
        for relative in json.loads(scope_text)["word_lists"]:
            vocabulary_file = (scope_file.parent / relative).resolve().relative_to(self.root)
            if not (self.root / vocabulary_file).is_file():
                self.write(vocabulary_file, "word\n")
        other_files = (
            self.write(REPORT_WORD_USAGE / "assets/notes.txt", terms[0]),
            self.write("docs/project-words.txt", terms[1]),
            self.write("tools/bootstrap/book-to-markdown/package-lock.json", terms[0]),
        )
        with patch.object(english, "SCOPE_FILE", scope_file):
            result, output = self.scan([*REPORT_WORD_LISTS, *other_files])
        self.assertEqual(result, 1, output)
        self.assertIn("English scan: 3 files;", output)
        self.assertIn("3 unknown word occurrences", output)
        for path, term in zip(other_files, (*terms, terms[0])):
            self.assertIn(f"Unknown English vocabulary in {path}:\n  Line 1: [{term}]", output)

    def test_exact_file_terms_do_not_hide_findings_in_another_file(self):
        term = "wrong" * 8
        approved = self.write("examples/first.md", term)
        other = self.write("examples/second.md", term)
        scope = {"word_lists": [], "always_excluded_directories": [],
                 "file_words": {"examples/first.md": [term]}}
        result, output = self.scan([approved, other], scope=scope)
        self.assertEqual(result, 1, output)
        self.assertNotIn(str(approved), output)
        self.assertIn(str(other), output)
        self.assertIn("1 unknown word occurrences", output)

    def test_invalid_exact_file_terms_are_configuration_errors(self):
        document = self.write("docs/example.md", "Valid English text.")
        for terms in (["two words"], [""], [42], "word"):
            with self.subTest(terms=terms):
                scope = {"word_lists": [], "always_excluded_directories": [],
                         "file_words": {"docs/example.md": terms}}
                result, output = self.scan([document], scope=scope)
                self.assertEqual(result, 2, output)
                self.assertIn("Cannot prepare English scan", output)

    def test_exact_file_terms_require_repository_relative_file_paths(self):
        document = self.write("docs/example.md", "Valid English text.")
        for name in ("docs/*.md", "../outside.md", "/absolute.md", "docs\\example.md"):
            with self.subTest(path=name):
                scope = {"word_lists": [], "always_excluded_directories": [],
                         "file_words": {name: ["word"]}}
                result, output = self.scan([document], scope=scope)
                self.assertEqual(result, 2, output)
                self.assertIn("Cannot prepare English scan", output)

    def test_vocabulary_requires_one_term_per_line(self):
        scope_file = self.write("scope.json", "{}")
        self.write("words.txt", "valid English")
        with patch.object(english, "SCOPE_FILE", scope_file):
            with self.assertRaisesRegex(ValueError, "Invalid vocabulary entry"):
                english.load_vocabulary({"word_lists": ["words.txt"]})

    def test_vocabulary_does_not_approve_unlisted_inflections(self):
        dictionary = english.SpellChecker(language="en")
        term = "wrong" * 8
        findings = english.unknown_words(f"{term} {term}s", dictionary, {term})
        self.assertEqual([word for _, word, _ in findings], [f"{term}s"])



    def git(self, *arguments):
        return subprocess.run(
            ["git", "-c", "user.name=English tests", "-c", "user.email=tests@example.com", *arguments],
            cwd=self.root, check=True, capture_output=True,
            env={**os.environ, "GIT_CONFIG_GLOBAL": os.devnull},
        )

    def configure(self, lists, allowances=None):
        scope = {
            "word_lists": lists,
            "file_words": allowances or {},
            "excluded_files": ["scope.json", "proposal.json"],
            "excluded_directories": [".agent/tmp", "excluded"],
            "always_excluded_files": [],
            "always_excluded_directories": [".agent/tasks"],
            "excluded_directory_names": [".git"],
            "text_extensions": [".md", ".json", ".txt"],
            "text_filenames": [".gitignore"],
        }
        self.write("scope.json", json.dumps(scope, indent=2))

    def audit(self, files=(), removals=None, workers=1):
        options = [] if removals is None else ["--removals", self.write("proposal.json", json.dumps(removals))]
        with patch.object(english, "SCOPE_FILE", self.root / "scope.json"):
            result, output = self.scan([*options, *files], workers=workers)
        records = [line for line in output.splitlines() if line.startswith("[INFO] English scan report: ")]
        self.assertEqual(len(records), 1, output)
        locator = records[0].split(": ", 1)[1].split(" | ", 1)[0]
        self.assertTrue(locator.startswith(".agent/tmp/english/"), locator)
        report = json.loads((self.root / locator).read_text(encoding="utf-8"))
        self.assertEqual(report["exit_code"], result, output)
        return report, locator

    def findings(self, report, kind):
        return [finding for finding in report["findings"] if finding["kind"] == kind]

    def candidates(self, report):
        return self.findings(report, "unused_entry_candidate")

    def test_repository_reports_global_evidence_and_selected_files_keep_exact_ownership(self):
        self.git("init", "--quiet")
        used, unused, literal, redundant = ("wrong" * count for count in range(3, 7))
        self.configure(["words.txt"], {"first.md": [literal, redundant]})
        self.write("words.txt", f"{used}\nred\n{unused}\n")
        self.write("first.md", literal)
        self.git("add", "first.md")
        self.write("new.md", used)
        self.write("excluded/notes.md", unused)
        self.write(".agent/tasks/plan.md", unused)

        full, first_locator = self.audit()
        self.assertTrue(full["complete"])
        self.assertEqual(full["inputs"]["target_paths"], ["first.md", "new.md", "words.txt"])
        self.assertEqual(full["target_count"], full["scanned_count"])
        self.assertEqual([(finding["entry"]["term"], finding["reason"]) for finding in self.candidates(full)],
                         [("red", "base_english"), (unused, "no_observed_use"), (redundant, "no_observed_use")])
        self.assertEqual(self.candidates(full)[1]["entry"],
                         {"type": "word_list", "path": "words.txt", "spelling": unused,
                          "term": unused, "line": 3})

        self.write("second.md", literal)
        selected, next_locator = self.audit(["first.md", "second.md", ".agent/tasks/plan.md"])
        self.assertNotEqual(first_locator, next_locator)
        self.assertEqual(selected["scope"], "files")
        self.assertEqual(selected["requested_files"], ["first.md", "second.md", ".agent/tasks/plan.md"])
        self.assertEqual(selected["target_count"], 2)
        self.assertEqual([(finding["path"], finding["word"])
                          for finding in self.findings(selected, "unknown_word")], [("second.md", literal)])
        self.assertEqual([finding["entry"] for finding in self.candidates(selected)],
                         [{"type": "file_word", "path": "scope.json", "owner": "first.md",
                           "position": 1, "spelling": redundant, "term": redundant}])

    def test_read_error_keeps_unknown_and_supported_exact_candidate(self):
        unknown, redundant = "wrong" * 8, "wrong" * 9
        self.configure([], {"first.md": [redundant]})
        self.write("first.md", f"{unknown} {unknown}\n")
        broken = self.write("broken.txt", "")
        broken.write_bytes(bytes([255]))
        report, _ = self.audit(["first.md", "broken.txt"])
        self.assertEqual(report["exit_code"], 2)
        self.assertFalse(report["complete"])
        self.assertEqual((report["target_count"], report["scanned_count"]), (2, 1))
        self.assertEqual(self.findings(report, "unknown_word"),
                         [{"kind": "unknown_word", "path": "first.md", "line": 1,
                           "word": unknown, "preview": f"{unknown} {unknown}"}])
        [error] = self.findings(report, "scan_error")
        self.assertEqual((error["path"], error["phase"]), ("broken.txt", "read"))
        self.assertTrue(error["diagnostic"])
        self.assertEqual([finding["entry"]["term"] for finding in self.candidates(report)], [redundant])

        self.write("scope.json", "{")
        invalid, _ = self.audit(["first.md"])
        self.assertEqual(invalid["exit_code"], 2)
        self.assertFalse(invalid["complete"])
        self.assertEqual(invalid["findings"][0]["phase"], "configuration")
        self.assertEqual(invalid["findings"][0]["path"], "scope.json")

    def test_worker_error_preserves_observations_from_other_files(self):
        unknown = "wrong" * 8
        self.configure([])
        self.write("broken.txt", "Fail the worker\n")
        self.write("first.txt", unknown)
        self.write("last.txt", unknown)
        prepare = english.prepare_line

        def fail_one_line(line, positions=False):
            if line == "Fail the worker":
                raise ValueError("Worker lost its input")
            return prepare(line, positions)

        with patch.object(english, "prepare_line", side_effect=fail_one_line):
            report, _ = self.audit(["first.txt", "broken.txt", "last.txt"])
        self.assertEqual(report["exit_code"], 2)
        self.assertFalse(report["complete"])
        self.assertEqual(report["scanned_count"], 2)
        self.assertEqual([item["path"] for item in self.findings(report, "unknown_word")],
                         ["first.txt", "last.txt"])
        self.assertEqual(self.findings(report, "scan_error"),
                         [{"kind": "scan_error", "phase": "worker", "path": "broken.txt",
                           "diagnostic": "Worker lost its input"}])

        self.configure(["broken.txt"])
        (self.root / "broken.txt").write_bytes(bytes([255]))
        invalid, _ = self.audit(["first.txt"])
        self.assertFalse(invalid["complete"])
        self.assertEqual([(item["path"], item["phase"]) for item in self.findings(invalid, "scan_error")],
                         [("broken.txt", "vocabulary")])

    def test_combined_removal_replays_whole_identifier_escape_and_inflection(self):
        self.git("init", "--quiet")
        piece = "wrong" * 3
        compound = piece.title() + "42Red"
        escaped_prefix = "n" + "red"
        inflection = "wrong" * 5 + "s"
        self.configure(["words.txt"])
        self.write("words.txt", f"{compound}\n{piece}\n{escaped_prefix}\n{inflection}\n")
        self.write("notes.md", f"{compound} \\{escaped_prefix} {inflection}\n")
        full, _ = self.audit()
        self.assertEqual(full["exit_code"], 0, full["findings"])
        candidates = {finding["entry"]["term"]: finding for finding in self.candidates(full)}
        self.assertEqual(set(candidates), {compound.lower(), piece, escaped_prefix})
        self.assertEqual(candidates[compound.lower()]["alternative"]["accepted_words"], [piece, "red"])
        self.assertEqual(candidates[escaped_prefix]["alternative"]["accepted_words"], ["red"])
        self.assertEqual(candidates[piece]["reason"], "no_observed_use")

        joint = [candidates[term]["entry"] for term in (compound.lower(), piece)]
        error, first_locator = self.audit(removals=joint)
        replay, next_locator = self.audit(removals=joint, workers=2)
        self.assertNotEqual(first_locator, next_locator)
        self.assertEqual(replay, error)
        self.assertEqual(error["exit_code"], 1)
        self.assertFalse(error["validation"]["accepted"])
        self.assertEqual(error["validation"]["entries"], joint)
        self.assertEqual(error["validation"]["added_unknowns"],
                         [{"path": "notes.md", "line": 1, "column": 1, "word": piece,
                           "preview": f"{compound} \\{escaped_prefix} {inflection}"}])
        self.assertEqual(error["validation"]["hidden_unknowns"], [])
        safe, _ = self.audit(removals=[joint[0], candidates[escaped_prefix]["entry"]])
        self.assertTrue(safe["validation"]["accepted"])
        self.assertEqual(safe["exit_code"], 0)

    def test_duplicate_declarations_prefer_exact_support_and_keep_real_list_comments(self):
        self.git("init", "--quiet")
        duplicate, unused, comment = ("wrong" * count for count in range(3, 6))
        self.configure(["first.txt", "second.txt"], {"notes.md": [duplicate, duplicate]})
        list_text = f"{duplicate}\n{duplicate}\n{unused}  # {comment}\n{comment}\n"
        first = self.write("first.txt", list_text)
        self.write("second.txt", duplicate)
        self.write("notes.md", duplicate)
        full, _ = self.audit()
        candidates = self.candidates(full)
        self.assertEqual([(entry["path"], entry.get("line", entry.get("position")))
                          for entry in (finding["entry"] for finding in candidates)],
                         [("first.txt", 1), ("first.txt", 2), ("first.txt", 3),
                          ("second.txt", 1), ("scope.json", 1)])
        self.assertEqual(candidates[0]["alternatives"][0]["owner"], "notes.md")
        self.assertEqual(candidates[0]["alternatives"][0]["position"], 0)
        proposal = [finding["entry"] for finding in candidates]
        safe, _ = self.audit(removals=proposal)
        self.assertTrue(safe["validation"]["accepted"])
        self.assertEqual(first.read_text(encoding="utf-8"), list_text)
        required_comment = {"type": "word_list", "path": "first.txt", "line": 4,
                            "spelling": comment, "term": comment}
        rejected, _ = self.audit(removals=[*proposal, required_comment])
        self.assertFalse(rejected["validation"]["accepted"])
        self.assertEqual([(finding["path"], finding["line"], finding["word"])
                          for finding in rejected["validation"]["added_unknowns"]],
                         [("first.txt", 3, comment)])

        self.write("outside.md", duplicate)
        outside, _ = self.audit()
        entries = [finding["entry"] for finding in self.candidates(outside)]
        self.assertNotIn({"type": "word_list", "path": "first.txt", "line": 1,
                          "spelling": duplicate, "term": duplicate}, entries)
        other_duplicate = next(finding for finding in self.candidates(outside)
                               if finding["entry"]["path"] == "second.txt")
        self.assertEqual(other_duplicate["alternatives"][0]["path"], "first.txt")
        self.assertEqual(other_duplicate["alternatives"][0]["line"], 1)

    def test_allowance_declarations_supply_no_usage_and_keep_ordinary_unknowns(self):
        term = "wrong" * 8
        self.write("words", "json")
        scope = {"word_lists": ["words"], "file_words": {"scope.json": [term, term, term.upper()]},
                 "always_excluded_directories": []}
        self.write("scope.json", json.dumps(scope))
        report, _ = self.audit(["scope.json"])
        self.assertEqual(report["exit_code"], 0, report["findings"])
        self.assertEqual([finding["entry"]["position"] for finding in self.candidates(report)], [0, 1, 2])
        self.assertEqual([finding["reason"] for finding in self.candidates(report)], ["no_observed_use"] * 3)
        scope["file_words"]["scope.json"] = [term.upper()]
        self.write("scope.json", json.dumps(scope))
        uppercase, _ = self.audit(["scope.json"])
        self.assertEqual(uppercase["exit_code"], 1)
        self.assertEqual([finding["word"] for finding in self.findings(uppercase, "unknown_word")], [term])

        # The same spelling in another JSON value is a genuine use.
        scope["file_words"]["scope.json"] = [term]
        scope["example"] = term
        self.write("scope.json", json.dumps(scope))
        used, _ = self.audit(["scope.json"])
        self.assertEqual(used["exit_code"], 0)
        self.assertEqual(self.candidates(used), [])

    def test_orphans_require_recorded_deletion_and_sparse_input_blocks_global_absence(self):
        self.git("init", "--quiet")
        names = ["removed.md", "missing.md", "sparse/hidden.md", "ignored/generated.md", "ready.md", "excluded/notes.md"]
        redundant = "wrong" * 8
        self.configure(["words.txt"], {name: [redundant] for name in names})
        self.write("words.txt", f"{redundant}\n")
        self.write(".gitignore", "/ignored/\n")
        for name in names[:3] + [names[4]]:
            self.write(name, "Red\n")
        self.git("add", ".gitignore", "removed.md", "missing.md", "sparse/hidden.md", "ready.md")
        self.git("commit", "--quiet", "-m", "Add example files")
        self.git("rm", "--quiet", "removed.md")
        self.git("commit", "--quiet", "-m", "Remove example file")
        self.git("sparse-checkout", "set", "--no-cone", "/.gitignore", "/missing.md", "/ready.md")
        (self.root / "missing.md").unlink()
        self.write("ignored/generated.md", "Red\n")

        repository, _ = self.audit()
        self.assertEqual(repository["exit_code"], 2)
        self.assertFalse(repository["complete"])
        self.assertEqual({finding["path"] for finding in self.findings(repository, "scan_error")},
                         {"missing.md", "sparse/hidden.md"})
        self.assertEqual({item["owner"]: item["reason"] for item in repository["deferred_owners"]},
                         {"missing.md": "missing_tracked_owner", "sparse/hidden.md": "sparse_owner",
                          "ignored/generated.md": "ignored_owner", "excluded/notes.md": "excluded_owner"})
        self.assertEqual([(finding["entry"]["owner"], finding["reason"])
                          for finding in self.candidates(repository)],
                         [("removed.md", "confirmed_owner_deletion"), ("ready.md", "no_observed_use")])
        [commit] = repository["inputs"]["deleted_owners"].values()
        self.assertEqual(commit, self.git("rev-parse", "HEAD").stdout.decode("utf-8").strip())
        selected, _ = self.audit(["words.txt"])
        self.assertEqual(selected["exit_code"], 0)
        self.assertEqual(selected["deferred_owners"], [])
        self.assertEqual(self.candidates(selected), [])
        rejected, _ = self.audit(removals=[self.candidates(repository)[0]["entry"]])
        self.assertEqual(rejected["exit_code"], 2)
        self.assertFalse(rejected["validation"]["accepted"])

    def test_validation_rejects_equal_unknown_counts_when_the_occurrence_changes(self):
        self.git("init", "--quiet")
        used, orphan, possessive = ("wrong" * count for count in range(3, 6))
        self.configure(["words.txt"])
        self.write("words.txt", f"{possessive}'s\n{used}\n{orphan}\n")
        document = self.write("notes.md", used)
        original = document.read_bytes()

        baseline, _ = self.audit()
        [unknown] = self.findings(baseline, "unknown_word")
        self.assertEqual((unknown["path"], unknown["line"], unknown["word"]), ("words.txt", 1, possessive))
        by_term = {finding["entry"]["term"]: finding["entry"] for finding in self.candidates(baseline)}
        safe, _ = self.audit(removals=[by_term[orphan]])
        self.assertEqual(safe["exit_code"], 1)
        self.assertTrue(safe["validation"]["accepted"])
        used_entry = {"type": "word_list", "path": "words.txt", "line": 2, "spelling": used, "term": used}
        rejected, _ = self.audit(removals=[by_term[f"{possessive}'s"], used_entry])
        self.assertEqual(rejected["exit_code"], 1)
        self.assertFalse(rejected["validation"]["accepted"])
        self.assertEqual([item["word"] for item in rejected["validation"]["hidden_unknowns"]], [possessive])
        self.assertEqual([item["word"] for item in rejected["validation"]["added_unknowns"]], [used])
        self.assertEqual(baseline["inputs"], rejected["inputs"])
        self.assertEqual(document.read_bytes(), original)

        self.write("notes.md", f"{used}\n{orphan}\n")
        stale, _ = self.audit(removals=[by_term[orphan]])
        self.assertFalse(stale["validation"]["accepted"])
        self.assertNotEqual(baseline["inputs"]["sha256"]["notes.md"], stale["inputs"]["sha256"]["notes.md"])
        self.assertEqual([item["word"] for item in stale["validation"]["added_unknowns"]], [orphan])

    def test_validation_rejects_stale_declaration_identity_and_retains_current_unknown(self):
        self.git("init", "--quiet")
        term, unknown = "wrong" * 8, "wrong" * 9
        self.configure(["words.txt"])
        self.write("words.txt", term)
        self.write("notes.md", unknown)
        baseline, _ = self.audit()
        [candidate] = self.candidates(baseline)
        self.write("words.txt", f"red\n{term}\n")
        rejected, _ = self.audit(removals=[candidate["entry"]])
        self.assertEqual(rejected["exit_code"], 2)
        self.assertFalse(rejected["complete"])
        self.assertFalse(rejected["validation"]["accepted"])
        self.assertEqual([item["word"] for item in self.findings(rejected, "unknown_word")], [unknown])
        self.assertEqual([item["phase"] for item in self.findings(rejected, "scan_error")], ["proposal"])

if __name__ == "__main__":
    unittest.main()

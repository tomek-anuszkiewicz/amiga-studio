"""Language scan scope and vocabulary boundaries."""

import contextlib
import io
import json
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

    def scan(self, files, repository_files=(), scope=None):
        output = io.StringIO()
        scope_file = english.SCOPE_FILE
        if scope is not None:
            scope_file = self.write("scope.json", json.dumps(scope))
        with patch.object(english, "REPO_ROOT", self.root), \
             patch.object(english, "SCOPE_FILE", scope_file), \
             patch.object(english, "repository_targets", return_value=repository_files), \
             patch.object(english, "worker_count", return_value=1), \
             patch.multiple(english, WORKER_SCOPE=None, WORKER_VOCABULARY=None, WORKER_DICTIONARY=None), \
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
        task = self.write(".agent/tasks/plan.md", "wrong" * 8)
        document = self.write("docs/example.md", "Valid English text.")
        result, output = self.scan([], [task, document])
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


if __name__ == "__main__":
    unittest.main()

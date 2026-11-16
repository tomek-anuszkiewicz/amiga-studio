"""Generated timing data must not become authored verification input."""
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
import commands
import repository
import timing_report
from cli import parse_args


class TimingReportTests(unittest.TestCase):
    def test_scope_preserves_selectors_without_host_paths_or_inline_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            command = [sys.executable, "-m", "unittest", "discover", "-s", str(root / "tools/tests")]
            scope = timing_report.command_scope(root, command)
            self.assertIn("-m unittest discover -s tools/tests", scope)
            self.assertNotIn(directory, scope)
            self.assertNotIn(str(Path(sys.executable).parent), scope)
            self.assertNotIn("private source", timing_report.command_scope(
                root, [sys.executable, "-c", "private source"]))

    def test_report_only_selection_preserves_quality_gates_without_selecting_tests(self):
        _, args = parse_args(["commit"])
        phases = commands.select_phases(args, {repository.TIMING_REPORT})
        selected = {name for _, items in phases for name, _, _ in items}
        self.assertFalse(any(phase == "Coverage" for phase, _ in phases))
        self.assertNotIn("All standard harness tests", selected)
        self.assertNotIn("Verification CLI contracts", selected)
        self.assertTrue({"Strict English Policy", "Architecture Rules", "WebAssembly build",
                         "RAG MCP contracts", "Artifact report contracts"} <= selected)

    def test_selection_tests_tool_code_but_skips_vocabulary_docs_and_generated_data(self):
        for path, expected in (("tools/verification/evidence.py", True),
                               ("tools/verification/checks/check_english.json", True),
                               ("tools/web.ps1", True), ("tools/worktree.ps1", True),
                               ("tools/graphify_query.py", True),
                               (".agents/skills/obsidian-vault-linking/scripts/audit_vault.py", True),
                               ("tools/verification/checks/english_words/software.txt", False),
                               ("crates/cpu/src/cpu.rs", False),
                               ("tools/verification/README.md", False),
                               (repository.TIMING_REPORT, False)):
            with self.subTest(path=path):
                self.assertEqual(commands.verification_contract_input(path), expected)
                names = {name for name, _, _ in commands.python_commands(changes={path})}
                self.assertEqual("Verification CLI contracts" in names, expected)
                _, args = parse_args(["commit"])
                selected = {name for _, items in commands.select_phases(args, {path}) for name, _, _ in items}
                self.assertEqual("Verification CLI contracts" in selected, expected)

    def test_generated_report_is_excluded_from_language_scan_but_neighbors_remain(self):
        sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "checks"))
        import check_english
        scope = json.loads(check_english.SCOPE_FILE.read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in (repository.TIMING_REPORT, "tests/authored.md"):
                target = root / name
                target.parent.mkdir(exist_ok=True)
                target.write_text("Example text.", encoding="utf-8")
            paths = (repository.TIMING_REPORT + "\0tests/authored.md\0").encode()
            with patch.object(check_english, "REPO_ROOT", root), \
                 patch.object(check_english.subprocess, "run",
                              return_value=subprocess.CompletedProcess([], 0, paths)):
                self.assertEqual(check_english.repository_targets(scope), [root / "tests/authored.md"])

    def test_report_distinguishes_execution_reuse_failure_and_omission(self):
        rows = [timing_report.measurement("Coverage", "Coverage", "Export", "executed", 0.25),
                timing_report.measurement("Verification", "CPU", "CPU", "reused", previous_seconds=12),
                timing_report.measurement("Preflight", "English", "English", "failed", 0.5),
                timing_report.measurement("Build checks", "Web", "Web", "missing"),
                timing_report.measurement("Verification", "Other", "Other", "skipped")]
        with tempfile.TemporaryDirectory() as directory, \
             patch.object(timing_report, "git", return_value=b"1234567"), \
             contextlib.redirect_stdout(io.StringIO()) as console:
            root = Path(directory)
            measured = timing_report.publish(root, "commit", rows, 2.0)
            content = (root / repository.TIMING_REPORT).read_text(encoding="utf-8")
        for status in ("executed", "reused", "failed", "missing", "skipped"):
            self.assertIn(status, content)
            self.assertIn(status, console.getvalue())
        self.assertIn("12.000", content)
        self.assertIn("0.250s current", console.getvalue())
        self.assertGreaterEqual(measured["generation_seconds"], 0)
        self.assertGreaterEqual(measured["write_seconds"], 0)


if __name__ == "__main__":
    unittest.main()

"""Generated timing data must not become authored verification input."""
import contextlib
from hashlib import sha256
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from git_fixtures import restore_repository

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import commands
import impact
import timing_report
from cli import parse_args
from repository import git


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

    def test_retired_report_path_selects_full_rust_and_normal_quality_gates(self):
        _, args = parse_args(["commit"])
        phases = commands.select_phases(args, {"tests/test-runtime-report.md"})
        selected = {name for _, items in phases for name, _, _ in items}
        self.assertIn("All ordinary crate tests", selected)
        self.assertIn("All standard harness tests", selected)
        self.assertNotIn("Verification CLI contracts", selected)
        self.assertTrue({"Strict English Policy", "Architecture Rules", "WebAssembly build",
                         "RAG MCP contracts", "Artifact report contracts"} <= selected)

    def test_selection_tests_tool_code_but_skips_vocabulary_docs_and_generated_data(self):
        routes = {
            "tools/verification/evidence.py": True,
            "tools/verification/checks/check_english.json": True,
            "tools/web.ps1": True,
            "tools/verification/checks/english_words/software.txt": False,
            "crates/cpu/src/cpu.rs": False,
            "tools/verification/README.md": False,
            ".agent/tmp/verification/diagnostics/attempt/test-runtime-report.md": False,
        }
        predicate_only = {
            "tools/worktree.ps1": True,
            "tools/graphify_query.py": True,
            ".agents/skills/TA-audit-vault-links/scripts/audit_vault.py": True,
        }
        for path, expected in (routes | predicate_only).items():
            with self.subTest(path=path):
                self.assertEqual(commands.verification_contract_input(path), expected)

        root = commands.REPO_ROOT
        graph = ({root / "crates/cpu": "cpu"}, {"cpu": set()})
        _, args = parse_args(["commit"])
        with patch.object(impact, "workspace_graph", return_value=graph):
            for path, expected in routes.items():
                with self.subTest(route=path):
                    names = {name for name, _, _ in commands.python_commands(changes={path})}
                    self.assertEqual("Verification CLI contracts" in names, expected)
                    selected = {name for _, items in commands.select_phases(args, {path}) for name, _, _ in items}
                    self.assertEqual("Verification CLI contracts" in selected, expected)

    def test_ignored_report_is_outside_scan_and_both_authored_paths_are_checked(self):
        checks = Path(__file__).resolve().parents[1] / "checks"
        with patch.object(sys, "path", [str(checks), *sys.path]):
            import check_english
        scope = json.loads(check_english.SCOPE_FILE.read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            restore_repository("empty", root)
            names = ("tests/authored.md", "tests/test-runtime-report.md")
            generated = ".agent/tmp/verification/diagnostics/attempt/test-runtime-report.md"
            for name in (*names, generated):
                target = root / name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text("Example text.", encoding="utf-8")
            # The scanner must exclude the report even if it is present in the index.
            git(root, "add", "--force", "--", names[1], generated)
            with patch.object(check_english, "REPO_ROOT", root):
                self.assertEqual(check_english.repository_targets(scope), [root / name for name in names])

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
            attempt = root / "verify/attempt"
            measured = timing_report.publish(root, root, attempt, "commit", rows, 2.0)
            report = (attempt / "test-runtime-report.md").read_bytes()
            self.assertEqual(measured["path"], "verify/attempt/test-runtime-report.md")
            self.assertEqual(measured["sha256"], sha256(report).hexdigest())
            with self.assertRaises(FileExistsError):
                timing_report.publish(root, root, attempt, "commit", [], 0.0)
            self.assertEqual((root / measured["path"]).read_bytes(), report)
            content = report.decode("utf-8")
        for status in ("executed", "reused", "failed", "missing", "skipped"):
            self.assertIn(status, content)
            self.assertIn(status, console.getvalue())
        self.assertIn("12.000", content)
        self.assertIn("0.250s current", console.getvalue())
        self.assertGreaterEqual(measured["generation_seconds"], 0)
        self.assertGreaterEqual(measured["write_seconds"], 0)


if __name__ == "__main__":
    unittest.main()

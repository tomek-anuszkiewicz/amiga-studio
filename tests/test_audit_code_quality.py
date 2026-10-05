"""Verify code audit exit status against isolated Rust source fixtures."""

import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "audit_code_quality", Path(__file__).resolve().parents[1] / "tools/harness/audit_code_quality.py"
)
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)

CONDITION_SOURCE = """
fn check(a: bool, b: bool, c: bool, d: bool) -> bool {
    if a && b && c && d { true } else { false }
}
"""
ACCESSOR_SOURCE = """
struct Probe {
    value: u32,
}
impl Probe {
    fn get_value(&self) -> u32 { self.value }
}
"""


class TestAuditCodeQuality(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        crate = self.root / "crates/fixture"
        (crate / "src").mkdir(parents=True)
        (crate / "Cargo.toml").write_text('[package]\nname = "fixture"\n', encoding="utf-8")
        self.source = crate / "src/fixture.rs"
        self.source.write_text("", encoding="utf-8")
        for name, value in (
            ("REPO_ROOT", self.root),
            ("CRATES_DIR", self.root / "crates"),
            ("_CORPUS_CACHE", {}),
            ("_INVERTED_INDEX_CACHE", {}),
        ):
            patcher = patch.object(AUDIT, name, value)
            patcher.start()
            self.addCleanup(patcher.stop)

    def invoke(self, *args):
        output = io.StringIO()
        with patch.object(AUDIT.sys, "argv", ["audit_code_quality.py", "--crate", "fixture", *args]), patch.object(
            AUDIT.sys, "stdout", output
        ):
            try:
                code = AUDIT.main()
            except SystemExit as exc:
                code = exc.code
        return code or 0, output.getvalue()

    def test_strict_milestone_rejects_condition_and_accessor_violations(self):
        for source, diagnostic in (
            (CONDITION_SOURCE, "compound condition(s)"),
            (ACCESSOR_SOURCE, "accessor convention violation(s)"),
        ):
            with self.subTest(diagnostic=diagnostic):
                self.source.write_text(source, encoding="utf-8")
                code, output = self.invoke("--milestone", "--strict")
                self.assertIn(diagnostic, output)
                self.assertEqual(code, 1)

    def test_strict_json_preserves_findings_and_rejects_violations(self):
        self.source.write_text(CONDITION_SOURCE + ACCESSOR_SOURCE, encoding="utf-8")
        code, output = self.invoke("--milestone", "--strict", "--json")
        report = json.loads(output)
        self.assertEqual(len(report["condition_issues"]), 1)
        self.assertEqual(len(report["accessor_issues"]), 1)
        self.assertEqual(code, 1)

    def test_non_strict_milestone_reports_findings_without_failure(self):
        self.source.write_text(CONDITION_SOURCE + ACCESSOR_SOURCE, encoding="utf-8")
        code, output = self.invoke("--milestone")
        self.assertIn("compound condition(s)", output)
        self.assertIn("accessor convention violation(s)", output)
        self.assertEqual(code, 0)

    def test_clean_strict_milestone_passes_in_text_and_json_modes(self):
        for options in ((), ("--json",)):
            with self.subTest(options=options):
                code, _ = self.invoke("--milestone", "--strict", *options)
                self.assertEqual(code, 0)

    def test_per_commit_does_not_run_milestone_scanners(self):
        with patch.object(AUDIT, "scan_condition_soup") as conditions, patch.object(
            AUDIT, "scan_accessor_conventions"
        ) as accessors:
            code, _ = self.invoke("--per-commit", "--strict")
        conditions.assert_not_called()
        accessors.assert_not_called()
        self.assertEqual(code, 0)

    def test_strict_individual_scanners_reject_violations(self):
        for source, flag in (
            (CONDITION_SOURCE, "--conditions"),
            (ACCESSOR_SOURCE, "--accessors"),
        ):
            with self.subTest(flag=flag):
                self.source.write_text(source, encoding="utf-8")
                code, _ = self.invoke(flag, "--strict")
                self.assertEqual(code, 1)

    def test_per_commit_still_rejects_dead_code_in_text_and_json_modes(self):
        self.source.write_text("pub fn unused() {}\n", encoding="utf-8")
        for options in ((), ("--json",)):
            with self.subTest(options=options):
                code, _ = self.invoke("--per-commit", "--strict", *options)
                self.assertEqual(code, 1)


if __name__ == "__main__":
    unittest.main()

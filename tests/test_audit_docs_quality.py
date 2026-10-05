"""Verify concrete documentation failures with isolated repository fixtures."""

import importlib.util
import io
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


REPO_ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "audit_docs_quality", REPO_ROOT / "tools/harness/audit_docs_quality.py"
)
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)


class TestAuditDocsQuality(unittest.TestCase):
    def setUp(self):
        scratch = REPO_ROOT / ".agent/tmp"
        scratch.mkdir(parents=True, exist_ok=True)
        temp = tempfile.TemporaryDirectory(dir=scratch)
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        patcher = patch.object(AUDIT, "REPO_ROOT", self.root)
        patcher.start()
        self.addCleanup(patcher.stop)

    def write(self, relative, content):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        return path

    def invoke(self, *args):
        output = io.StringIO()
        with patch.object(AUDIT.sys, "argv", ["audit_docs_quality.py", *args]), patch.object(
            AUDIT.sys, "stdout", output
        ):
            with self.assertRaises(SystemExit) as result:
                AUDIT.main()
        return result.exception.code, output.getvalue()

    def test_links_detect_missing_targets_and_accept_encoded_existing_targets(self):
        self.write("Obsidian/Amiga/Design/Hub.md", "[Spec](New%20Spec.md#scope)\n")
        self.assertEqual(len(AUDIT.check_vault_links()["broken_links"]), 1)
        self.write("Obsidian/Amiga/Design/New Spec.md", "# Scope\n")
        self.assertEqual(AUDIT.check_vault_links()["broken_links"], [])

    def test_new_rules_and_root_instructions_still_obey_byte_limits(self):
        agents = self.write("AGENTS.md", "a" * AUDIT.MAX_AGENTS_MD_BYTES)
        rule = self.write(".agents/rules/new-rule.md", "a" * AUDIT.MAX_RULE_FILE_BYTES)
        self.assertEqual(AUDIT.check_size_limits(), [])
        agents.write_text(agents.read_text(encoding="utf-8") + "a", encoding="utf-8")
        rule.write_text(rule.read_text(encoding="utf-8") + "a", encoding="utf-8")
        self.assertEqual(len(AUDIT.check_size_limits()), 2)

    def test_diary_requires_file_and_section_ten(self):
        self.assertTrue(AUDIT.check_diary_structure_and_chronology()["issues"])
        self.write("DIARY.md", "# Diary\n")
        self.assertTrue(AUDIT.check_diary_structure_and_chronology()["issues"])
        self.write("DIARY.md", "## 10. Living Chronological Engineering Log\n")
        self.assertEqual(AUDIT.check_diary_structure_and_chronology()["issues"], [])

    def test_diary_rejects_decreasing_dates_and_accepts_chronological_entries(self):
        for dates, issue_count in (
            (["2026-10-05 12:00", "2026-10-04 12:00"], 1),
            (["2026-10-04 12:00", "2026-10-05 12:00"], 0),
        ):
            with self.subTest(dates=dates):
                self.write(
                    "DIARY.md",
                    "## 10. Living Chronological Engineering Log\n"
                    + "\n".join(f"### [{date} CEST] - Entry" for date in dates),
                )
                self.assertEqual(len(AUDIT.check_diary_structure_and_chronology()["issues"]), issue_count)

    def test_roadmap_requires_file_and_rejects_retained_completed_checkboxes(self):
        self.assertTrue(AUDIT.check_roadmap_zero_retention()["issues"])
        self.write("ROADMAP.md", "- [x] Completed task\n- [ ] Pending task\n")
        self.assertEqual(len(AUDIT.check_roadmap_zero_retention()["issues"]), 1)
        self.write("ROADMAP.md", "- [ ] Pending task\n")
        self.assertEqual(AUDIT.check_roadmap_zero_retention()["issues"], [])

    def test_workflow_cli_accepts_new_rules_and_specs_without_registration(self):
        self.write(".agents/rules/new-rule.md", "# New rule\n")
        self.write("Obsidian/Amiga/Design/New Spec.md", "# New specification\n")
        self.write("DIARY.md", "## 10. Living Chronological Engineering Log\n")
        self.write("ROADMAP.md", "- [ ] Pending task\n")
        code, output = self.invoke("--workflow-invariants")
        self.assertEqual(code, 0, output)
        self.assertIn("0 total issue(s)", output)

    def test_workflow_cli_reports_actual_diary_and_roadmap_failures(self):
        code, output = self.invoke("--workflow-invariants")
        self.assertEqual(code, 1, output)
        self.assertIn("DIARY.md does not exist", output)
        self.assertIn("ROADMAP.md does not exist", output)
        self.assertIn("2 total issue(s)", output)


if __name__ == "__main__":
    unittest.main()

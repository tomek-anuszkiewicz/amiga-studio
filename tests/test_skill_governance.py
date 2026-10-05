"""Exercise skill governance with isolated repository fixtures."""

import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "audit_docs_quality", Path(__file__).resolve().parents[1] / "tools/harness/audit_docs_quality.py"
)
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)


class TestSkillGovernance(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.addCleanup(patch.stopall)
        patch.object(AUDIT, "REPO_ROOT", self.root).start()

    def write(self, relative, content=""):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def skill(self, name):
        self.write(f".agents/skills/{name}/SKILL.md", f"---\nname: {name}\ndescription: Fixture procedure.\n---\n")

    def test_procedural_skill_needs_no_workflow(self):
        self.skill("compact-diary")
        result = AUDIT.check_skill_and_rule_governance()
        self.assertEqual(result["skill_count"], 1)
        self.assertEqual(result["rule_issues"], [])

    def test_active_rule_requires_a_companion(self):
        self.write(".agents/rules/docs-maintenance.md")
        self.assertEqual(AUDIT.check_skill_and_rule_governance()["rule_issues"][0]["type"], "missing_rule_skill")
        self.skill("sync-design-docs")
        self.assertEqual(AUDIT.check_skill_and_rule_governance()["rule_issues"], [])

    def test_passive_rule_keeps_redundant_skill_detection(self):
        self.write(".agents/rules/language-policy.md")
        self.skill("language-policy")
        self.assertEqual(AUDIT.check_skill_and_rule_governance()["rule_issues"][0]["type"], "redundant_passive_skill")

    def test_reference_and_entrypoint_count_as_one_consumer(self):
        self.skill("consumer")
        self.write("tools/harness/special.py")
        self.write(".agents/skills/consumer/SKILL.md", "special.py")
        self.write(".agents/skills/consumer/references/execution.md", "special.py")
        issues = AUDIT.check_script_locality_and_governance()
        self.assertEqual(issues[0]["consumers"], ["consumer"])

    def test_references_from_two_skills_are_shared_consumers(self):
        self.skill("first")
        self.skill("second")
        self.write("tools/harness/shared.py")
        self.write(".agents/skills/first/references/execution.md", "shared.py")
        self.write(".agents/skills/second/references/execution.md", "shared.py")
        self.assertEqual(AUDIT.check_script_locality_and_governance(), [])

    def test_foreign_reference_triggers_shared_script_promotion(self):
        self.skill("owner")
        self.skill("consumer")
        self.write(".agents/skills/owner/scripts/shared.py")
        self.write(".agents/skills/consumer/references/execution.md", "shared.py")
        issues = AUDIT.check_script_locality_and_governance()
        self.assertEqual(issues[0]["type"], "promote_to_harness")
        self.assertEqual(issues[0]["external_consumers"], ["consumer"])


if __name__ == "__main__":
    unittest.main()

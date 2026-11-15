"""Exit-status contract for evidence-based quality findings."""

import contextlib
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "checks"))
import audit_code_quality as audit


class CodeQualityExitStatus(unittest.TestCase):
    def run_audit(self, dead, zombies, visibility, accessors):
        output = io.StringIO()
        with (
            patch.object(sys, "argv", ["audit_code_quality.py"]),
            patch.object(audit, "scan_dead_and_zombie_code", return_value=(dead, zombies)),
            patch.object(audit, "scan_least_visibility", return_value=visibility),
            patch.object(audit, "scan_accessor_conventions", return_value=accessors),
            contextlib.redirect_stdout(output),
        ):
            result = audit.main()
        return result, output.getvalue()

    def test_visibility_suggestions_do_not_fail_quality_gate(self):
        suggestion = {
            "crate": "example", "name": "public_api", "file": "crates/example/src/example.rs",
            "line": 1, "recommended": "pub(crate)", "reason": "Only called within crate",
        }
        result, output = self.run_audit([], [], [suggestion], [])
        self.assertEqual(result, 0)
        self.assertIn("1 visibility suggestions (advisory)", output)

    def test_dead_code_still_fails_quality_gate(self):
        finding = {"kind": "fn", "crate": "example", "name": "unused", "file": "example.rs", "line": 1}
        result, _ = self.run_audit([finding], [], [], [])
        self.assertEqual(result, 1)

    def test_javascript_production_and_test_references_remain_distinct(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            crate = root / "crates/example"
            (crate / "src").mkdir(parents=True)
            (crate / "tests").mkdir()
            (crate / "Cargo.toml").write_text("[package]\nname = 'example'\n", encoding="utf-8")
            (crate / "src/example.rs").write_text(
                "pub fn browser_entry() {}\npub fn test_entry() {}\npub fn unused_entry() {}\n",
                encoding="utf-8")
            (crate / "worker.js").write_text("browser_entry();\n", encoding="utf-8")
            (crate / "tests/worker.js").write_text("test_entry();\n", encoding="utf-8")
            with patch.object(audit, "REPO_ROOT", root), patch.object(audit, "CRATES_DIR", root / "crates"), \
                 patch.object(audit, "_CORPUS_CACHE", {}), patch.object(audit, "_INVERTED_INDEX_CACHE", {}):
                dead, zombies = audit.scan_dead_and_zombie_code()
            self.assertEqual([item["name"] for item in dead], ["unused_entry"])
            self.assertEqual([item["name"] for item in zombies], ["test_entry"])
            self.assertEqual(zombies[0]["prod_callers_count"], 0)
            self.assertEqual(zombies[0]["test_callers_count"], 1)


if __name__ == "__main__":
    unittest.main()

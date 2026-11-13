"""Coverage summaries identify the executed test scope and reject stale data."""
import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import coverage_report


class CoverageReportTests(unittest.TestCase):
    def test_summary_records_selected_packages_without_claiming_workspace_coverage(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report = root / ".agent/tmp/coverage/summary.json"
            report.parent.mkdir(parents=True)
            source = root / "crates/gui/src/lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text("fixture", encoding="utf-8")
            unselected = root / "crates/unselected/src/lib.rs"
            unselected.parent.mkdir(parents=True)
            unselected.write_text("fixture", encoding="utf-8")
            summary = {name: {"count": 4, "covered": 3, "percent": 75}
                       for name in ("lines", "functions", "regions")}
            files = [{"filename": str(source), "summary": summary},
                     {"filename": str(unselected), "summary": summary},
                     {"filename": str(root / "crates/retired/src/lib.rs"), "summary": summary},
                     {"filename": str(source.parent / "deleted.rs"), "summary": summary}]
            report.write_text(json.dumps({"data": [{"totals": summary, "files": files}]}), encoding="utf-8")
            with patch.object(coverage_report, "__file__", str(root / "tools/verification/coverage_report.py")), \
                 patch.object(coverage_report, "workspace_packages", return_value=([], {
                     root / "crates/gui": "gui", root / "crates/cpu": "cpu",
                     root / "crates/unselected": "unselected"})), \
                 contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(coverage_report.main(["--package", "gui", "--package", "cpu"]), 0)
            scope = json.loads(report.read_text(encoding="utf-8"))["verification_scope"]
            self.assertEqual(scope["test_packages"], ["cpu", "gui"])
            self.assertFalse(scope["full_workspace"])
            self.assertEqual(len(json.loads(report.read_text(encoding="utf-8"))["data"][0]["files"]), 1)
            self.assertIn("Excluded 3", output.getvalue())
            self.assertIn("75.00% (3/4)", output.getvalue())
            self.assertNotIn("workspace", output.getvalue())

    def test_prepare_removes_old_summary_and_missing_export_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report = root / ".agent/tmp/coverage/summary.json"
            report.parent.mkdir(parents=True)
            report.write_text("old measurement", encoding="utf-8")
            with patch.object(coverage_report, "__file__", str(root / "tools/verification/coverage_report.py")), \
                 contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(coverage_report.main(["--prepare"]), 0)
                self.assertFalse(report.exists())
                self.assertEqual(coverage_report.main(["--package", "gui"]), 1)


if __name__ == "__main__":
    unittest.main()

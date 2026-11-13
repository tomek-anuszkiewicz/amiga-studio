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
            for item in files:
                item["segments"] = [[10, 1, 0, True, True, False], [10, 8, 0, False, False, False]]
            functions = [{"name": "fixture", "filenames": [item["filename"]], "count": 0,
                          "regions": [[10, 1, 10, 8, 0, 0, 0, 0]]} for item in files]
            detailed = report.with_name("coverage.json")
            detailed.write_text(json.dumps({"data": [{"totals": summary, "files": files,
                                                       "functions": functions}]}), encoding="utf-8")
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
            scoped = json.loads(detailed.read_text(encoding="utf-8"))["data"][0]
            self.assertEqual(len(scoped["functions"]), 1)
            self.assertEqual(scoped["functions"][0]["filenames"], [str(source)])
            self.assertIn("segments", scoped["files"][0])
            gaps = report.with_name("gaps.md").read_text(encoding="utf-8")
            self.assertIn("crates/gui/src/lib.rs", gaps)
            self.assertIn("10:1-10:8", gaps)
            self.assertIn("Function entries: 10:1", gaps)
            self.assertNotIn("unselected", gaps)
            self.assertNotIn("retired", gaps)

    def test_prepare_removes_old_summary_and_missing_export_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report = root / ".agent/tmp/coverage/summary.json"
            report.parent.mkdir(parents=True)
            artifacts = [report, report.with_name("coverage.json"), report.with_name("gaps.md")]
            for artifact in artifacts:
                artifact.write_text("old measurement", encoding="utf-8")
            with patch.object(coverage_report, "__file__", str(root / "tools/verification/coverage_report.py")), \
                 contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(coverage_report.main(["--prepare"]), 0)
                self.assertFalse(report.exists())
                self.assertFalse(any(artifact.exists() for artifact in artifacts))
                self.assertEqual(coverage_report.main(["--package", "gui"]), 1)

    def test_zero_count_spans_keep_partial_lines_and_skip_unmapped_or_gap_regions(self):
        segments = [[10, 1, 4, True, True, False],
                    [10, 9, 0, True, True, False],
                    [10, 12, 0, True, False, False],
                    [10, 16, 4, True, False, False],
                    [11, 1, 0, False, False, False],
                    [12, 1, 0, True, False, True],
                    [13, 1, 0, True, True, False],
                    [14, 1, 0, False, False, False]]
        self.assertEqual(coverage_report.uncovered_spans(segments),
                         [((10, 9), (10, 16)), ((13, 1), (14, 1))])

    def test_function_entries_combine_executed_and_unexecuted_instantiations(self):
        root = Path.cwd()
        filename = str(root / "example.rs")
        summary = {name: {"count": 2, "covered": 1} for name in ("lines", "functions", "regions")}
        functions = [{"name": name, "filenames": [filename], "count": count,
                      "regions": [[line, 1, line, 8, count, 0, 0, 0]]}
                     for name, count, line in (("generic_u8", 0, 10), ("generic_u16", 4, 10), ("unused", 0, 20))]
        document = {"verification_scope": {"test_packages": ["example"], "full_workspace": False},
                    "data": [{"files": [{"filename": filename, "summary": summary, "segments": []}],
                              "functions": functions}]}
        report = coverage_report.render_gaps(document, root)
        self.assertIn("Function entries: 20:1", report)
        self.assertNotIn("Function entries: 10:1", report)
        self.assertIn("Zero-count function-instance regions: 10:1-10:8, 20:1-20:8", report)
        self.assertIn("selected packages only", report)

    def test_summary_only_export_is_rejected_instead_of_claiming_no_gaps(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report = root / ".agent/tmp/coverage/coverage.json"
            report.parent.mkdir(parents=True)
            report.write_text(json.dumps({"data": [{"files": []}]}), encoding="utf-8")
            with patch.object(coverage_report, "__file__", str(root / "tools/verification/coverage_report.py")), \
                 patch.object(coverage_report, "workspace_packages", return_value=([], {})), \
                 contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(coverage_report.main([]), 1)
            self.assertFalse(report.with_name("gaps.md").exists())


if __name__ == "__main__":
    unittest.main()

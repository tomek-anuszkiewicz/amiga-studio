"""Contracts for CPU performance test selection and stored-run analysis."""

import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import cpu_performance_tests as bench


class CatalogTests(unittest.TestCase):
    def test_post_series_monitor_exit_does_not_mark_load_disturbed(self):
        samples = [{"series": index, "external_busy_percent": 4.0} for index in (1, 2, 3)]
        samples.append({"error": "process no longer exists"})
        self.assertFalse(bench.load_disturbed(samples, 3))
        self.assertTrue(bench.load_disturbed(samples[:2], 3))

    def test_selectors_include_umbrella_families_and_reject_unknown_values(self):
        cases = {"add/add_w_d1_d0": {"target_primary_opcode": "D041"},
                 "bcc/bne_taken": {"target_primary_opcode": "6600"},
                 "dbcc/dbf_taken": {"target_primary_opcode": "51C8"},
                 "shift/lsl_w_d3_d4": {"target_primary_opcode": "E36C"}}
        self.assertEqual(bench.select_cases(cases, ("case", "add/add_w_d1_d0")), ["add/add_w_d1_d0"])
        self.assertEqual(bench.select_cases(cases, ("mnemonic", "DBF")), ["dbcc/dbf_taken"])
        self.assertEqual(bench.select_cases(cases, ("opcode", "d041")), ["add/add_w_d1_d0"])
        for selector in (("case", "absent"), ("mnemonic", "MULU"), ("opcode", "D04")):
            with self.subTest(selector=selector), self.assertRaises(ValueError):
                bench.select_cases(cases, selector)

    def test_bounded_load_retry_retains_both_attempts(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            calls = []

            def attempt(_binary, _case, _duration, _series, directory):
                calls.append(directory)
                return {"exit_code": 0, "disturbed": len(calls) == 1,
                        "report": "unused", "load_samples": [], "external_busy_percent": 25.0}

            with patch.object(bench, "RESULTS", root), patch.object(bench, "catalog", return_value={
                    "add/add_w_d1_d0": {"target_primary_opcode": "D041"}}), \
                 patch.object(bench, "executable", return_value="bench.exe"), \
                 patch.object(bench, "attempt", side_effect=attempt), \
                 patch.object(bench, "analyze"):
                self.assertEqual(bench.measure(("case", "add/add_w_d1_d0"), 50, 1), 0)
            manifest = json.loads(next(root.glob("*/manifest.json")).read_text(encoding="utf-8"))
            self.assertEqual(len(calls), 2)
            self.assertEqual(len(manifest["cases"]["add/add_w_d1_d0"]["attempts"]), 2)
            self.assertEqual(manifest["status"], "complete")

    def test_failed_attempt_remains_a_failure_without_retry(self):
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(bench, "RESULTS", Path(tmp)), patch.object(bench, "catalog", return_value={
                    "add/add_w_d1_d0": {"target_primary_opcode": "D041"}}), \
                 patch.object(bench, "executable", return_value="bench.exe"), \
                 patch.object(bench, "attempt", return_value={"exit_code": 1, "disturbed": False,
                                                              "report": "unused", "error": "execution failed"}) as attempt, \
                 patch.object(bench, "analyze"):
                self.assertEqual(bench.measure(None, 50, 1), 1)
            attempt.assert_called_once()

    def test_incomplete_run_cannot_be_analyzed(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "manifest.json").write_text('{"status":"incomplete"}', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "incomplete"):
                bench.analyze(tmp)

    def test_interruption_leaves_manifest_incomplete(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with patch.object(bench, "RESULTS", root), patch.object(bench, "catalog", return_value={
                    "add/add_w_d1_d0": {"target_primary_opcode": "D041"}}), \
                 patch.object(bench, "executable", return_value="bench.exe"), \
                 patch.object(bench, "attempt", side_effect=KeyboardInterrupt):
                with self.assertRaises(KeyboardInterrupt):
                    bench.measure(None, 50, 1)
            manifest = json.loads(next(root.glob("*/manifest.json")).read_text(encoding="utf-8"))
            self.assertEqual(manifest["status"], "incomplete")

    def test_comparison_uses_latest_compatible_local_full_run(self):
        case_id = "add/add_w_d1_d0"
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            host = {"os": "Windows", "architecture": "AMD64", "processor": "host"}
            settings = {"duration_ms": 400, "series": 3}

            def save_run(name, image_hash, time_ns):
                directory = root / name
                report_path = directory / "attempts/case/1/report.json"
                report_path.parent.mkdir(parents=True)
                report = {"case_id": case_id, "case_config": {"origin": 2097152},
                          "benchmark_config": settings, "assembler_sha256": "assembler",
                          "source_sha256": "source", "assembly_source_sha256": "assembly",
                          "image_sha256": image_hash, "host_os": "windows", "host_arch": "x86_64",
                          "affinity_core": 2, "priority_class": 32,
                          "results": [{"min_ns_per_target": time_ns,
                                       "median_ns_per_target": time_ns + 5,
                                       "target_instructions_per_run": 100000,
                                       "clocks_per_run": 800000, "range_pct": 1.0, "samples": []}]}
                bench.write_json(report_path, report)
                retry_path = directory / "attempts/case/2/report.json"
                retry_path.parent.mkdir(parents=True)
                retry_report = json.loads(json.dumps(report))
                retry_report["results"][0]["min_ns_per_target"] = time_ns + 5
                bench.write_json(retry_path, retry_report)
                manifest = {"run_id": name, "status": "complete", "full_catalog": True, "host": host,
                            "settings": settings, "selected_cases": [case_id],
                            "cases": {case_id: {"status": "measured", "disturbed": False,
                                                "selected_attempt": 0,
                                                "attempts": [{"report": "attempts/case/1/report.json",
                                                              "load_samples": [
                                                                  {"series": index, "external_busy_percent": 0.0}
                                                                  for index in (1, 2, 3)]},
                                                             {"report": "attempts/case/2/report.json"}]}}}
                bench.write_json(directory / "manifest.json", manifest)
                return directory

            save_run("001", "same", 10.0)
            save_run("002", "changed", 20.0)
            current = save_run("003", "same", 11.0)
            with patch.object(bench, "RESULTS", root):
                bench.analyze(current)
            comparison = json.loads((current / "comparison.json").read_text(encoding="utf-8"))
            aggregate = json.loads((current / "aggregate.json").read_text(encoding="utf-8"))
            self.assertEqual(aggregate["rows"][0]["ns_per_target"], 11.0)
            self.assertEqual(comparison["cases"][0]["previous_run"], "001")
            self.assertAlmostEqual(comparison["cases"][0]["change_percent"], 10.0)
            self.assertIn("## Previous-run comparison", (current / "report.md").read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()

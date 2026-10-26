"""Contracts for CPU performance test selection and stored-run analysis."""

import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import cpu_performance_tests as bench
import benchmark_anomalies as anomalies


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
        with tempfile.TemporaryDirectory() as tmp:
            case_list = Path(tmp) / "cases.txt"
            case_list.write_text("dbcc/dbf_taken\nadd/add_w_d1_d0\n", encoding="utf-8")
            self.assertEqual(bench.select_cases(cases, ("cases", str(case_list))),
                             ["dbcc/dbf_taken", "add/add_w_d1_d0"])
            case_list.write_text("dbcc/dbf_taken\ndbcc/dbf_taken\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "duplicate"):
                bench.select_cases(cases, ("cases", str(case_list)))

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

            initial = save_run("001", "same", 10.0)
            save_run("002", "changed", 20.0)
            current = save_run("003", "same", 11.0)
            filtered = save_run("004", "same", 12.0)
            filtered_manifest = json.loads((filtered / "manifest.json").read_text(encoding="utf-8"))
            filtered_manifest["full_catalog"] = False
            bench.write_json(filtered / "manifest.json", filtered_manifest)
            failed = save_run("005", "same", 13.0)
            failed_manifest = json.loads((failed / "manifest.json").read_text(encoding="utf-8"))
            failed_manifest["cases"][case_id]["status"] = "failed"
            bench.write_json(failed / "manifest.json", failed_manifest)
            cases_root = root / "cases"
            cases_root.mkdir()
            (cases_root / "config.json").write_text(
                '{"target_series_ms":400,"series":3}', encoding="utf-8")
            baseline_path = root / "m68k_benchmark_baseline.json"
            current_path = root / "m68k_benchmark_current.json"
            old_report = json.loads((initial / "attempts/case/1/report.json").read_text(encoding="utf-8"))
            old_row = {"case_id": case_id, "ns_per_target": 10.0, "target_count": 100000,
                       "cpu_clocks": 800000, "fingerprint": bench.fingerprint(old_report),
                       "disturbed": False}
            baseline_csv = anomalies.csv_bytes([old_row])
            baseline_snapshot = {"schema_version": 3, "kind": "handwritten_m68000",
                                 "source_run": "001", "host": host, "settings": settings,
                                 "case_count": 1, "rows": [old_row],
                                 "aggregate_csv_sha256": hashlib.sha256(baseline_csv).hexdigest()}
            anomalies.publish_current(baseline_path, baseline_snapshot, baseline_csv)
            with patch.object(bench, "RESULTS", root), patch.object(bench, "CASES", cases_root), \
                 patch.object(bench, "BASELINE", baseline_path), \
                 patch.object(bench, "CURRENT", current_path), \
                 patch.object(bench, "catalog", return_value={case_id: {}}):
                bench.analyze(current)
                bench.analyze(initial)
                bench.analyze(filtered)
                bench.analyze(failed)
            comparison = json.loads((current / "comparison.json").read_text(encoding="utf-8"))
            aggregate = json.loads((current / "aggregate.json").read_text(encoding="utf-8"))
            baseline_comparison = json.loads((current / "baseline_comparison.json").read_text(encoding="utf-8"))
            self.assertEqual(aggregate["rows"][0]["ns_per_target"], 11.0)
            self.assertEqual(comparison["cases"][0]["previous_run"], "001")
            self.assertAlmostEqual(comparison["cases"][0]["change_percent"], 10.0)
            self.assertAlmostEqual(baseline_comparison["cases"][0]["change_percent"], 10.0)
            self.assertEqual(anomalies.load_snapshot(current_path)["source_run"], "003")
            root_report = root / "m68k_benchmark_anomaly_report.md"
            root_related = root / "m68k_benchmark_cross_case_anomalies.json"
            root_a7 = root / "m68k_benchmark_a7_comparison.json"
            self.assertIn("Base full run: `003`", root_report.read_text(encoding="utf-8"))
            self.assertIn("m68k_benchmark_baseline_comparison.json",
                          root_report.read_text(encoding="utf-8"))
            self.assertEqual(json.loads(root_related.read_text(encoding="utf-8"))["source_run"], "003")
            self.assertEqual(json.loads(root_a7.read_text(encoding="utf-8"))["source_run"], "003")
            self.assertTrue((root / "m68k_benchmark_anomalies.json").is_file())
            self.assertTrue((root / "m68k_benchmark_baseline_comparison.json").is_file())
            self.assertIn("## Previous-run comparison", (current / "report.md").read_text(encoding="utf-8"))
            self.assertTrue((current / "baseline_comparison.json").is_file())
            self.assertTrue((current / "cross_case_anomalies.json").is_file())
            self.assertTrue((current / "a7_comparison.json").is_file())

    def test_baseline_comparison_keeps_incompatible_and_disturbed_cases_explicit(self):
        def row(case_id, ns, fingerprint="same", disturbed=False):
            return {"case_id": case_id, "ns_per_target": ns, "target_count": 100000,
                    "cpu_clocks": 800000, "fingerprint": fingerprint, "disturbed": disturbed}

        baseline = {"source_run": "old", "host": {"processor": "same"},
                    "settings": {"series": 3},
                    "rows": [row("stable", 20), row("slower", 20), row("faster", 20), row("changed", 20),
                             row("noisy", 20), row("removed", 20)]}
        current = {"source_run": "new", "host": baseline["host"],
                   "settings": baseline["settings"],
                   "rows": [row("stable", 20), row("slower", 22), row("faster", 18),
                            row("changed", 30, "different"), row("noisy", 30, disturbed=True),
                            row("added", 20)]}
        result = anomalies.compare_baseline(baseline, current)
        cases = {entry["case_id"]: entry for entry in result["cases"]}
        self.assertAlmostEqual(cases["slower"]["change_percent"], 10)
        self.assertTrue(cases["slower"]["flagged"])
        self.assertFalse(cases["stable"]["flagged"])
        self.assertFalse(cases["faster"]["flagged"])
        self.assertEqual(cases["changed"]["status"], "incompatible workload")
        self.assertEqual(cases["noisy"]["status"], "disturbed measurement")
        self.assertEqual(cases["removed"]["status"], "missing case")
        self.assertEqual(cases["added"]["status"], "new case")
        focused = anomalies.compare_baseline(baseline,
                                             dict(current, rows=current["rows"][:2]),
                                             ["stable", "slower"])
        self.assertEqual({entry["case_id"] for entry in focused["cases"]},
                         {"stable", "slower"})

    def test_related_cases_detect_mode_shift_and_skip_mismatched_clocks(self):
        rows = []
        for mode, add_ns, sub_ns, clocks in (("d1_d0", 20, 20, 800000),
                                             ("indirect", 30, 30, 1200000),
                                             ("predecrement", 40, 40, 1400000),
                                             ("indexed", 65, 40, 1600000)):
            for family, ns in (("add", add_ns), ("sub", sub_ns)):
                rows.append({"case_id": f"{family}/{family}_w_{mode}",
                             "ns_per_target": ns, "target_count": 100000,
                             "cpu_clocks": clocks, "disturbed": False})
        rows.append({"case_id": "add/add_w_displacement", "ns_per_target": 35,
                     "target_count": 100000, "cpu_clocks": 1200000, "disturbed": False})
        rows.append({"case_id": "sub/sub_w_displacement", "ns_per_target": 35,
                     "target_count": 100000, "cpu_clocks": 1400000, "disturbed": False})
        result = anomalies.compare_related(rows)
        self.assertEqual(len(result["peer_comparisons"]), 4)
        self.assertEqual(len(result["mode_profiles"]), 1)
        self.assertEqual(sum(row["flagged"] for row in result["mode_profiles"][0]["forms"]), 1)
        self.assertEqual(result["skipped"]["different target count or CPU clocks"], 1)
        profile = result["mode_profiles"][0]
        self.assertEqual(profile["direction"], "source")
        self.assertEqual(profile["form_count"], 3)
        self.assertAlmostEqual(profile["left_mean_ns_per_target"], 45)
        self.assertAlmostEqual(profile["right_mean_ns_per_target"], 110 / 3)
        self.assertEqual(sum(row["form_count"] for row in result["mode_profiles"]) +
                         len(result["other_pairs"]), len(result["peer_comparisons"]))
        indexed = next(row for row in profile["forms"] if row["mode"] == "indexed")
        self.assertAlmostEqual(indexed["average_mode_effect_ns"], 35 / 3)
        self.assertTrue(indexed["flagged"])
        report = "\n".join(anomalies.report_lines(
            {"baseline": "old", "current": "new", "cases": [], "status_counts": {}}, result))
        self.assertIn("| Case | Compared with | Times ns/target |", report)
        self.assertIn("`add/add_w_indexed`", report)
        self.assertNotIn("`add/add_w_indirect`", report)

    def test_a7_report_compares_only_matching_program_shapes_and_clocks(self):
        rows = []
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for case_id, source, ns, clocks in (
                ("add/add_w_postincrement", "add.w (a1)+,d5\nmove.l d7,d0", 20, 800000),
                ("add/add_w_a7_postincrement_d0", "add.w (a7)+,d0\nmove.l d7,d1", 25, 800000),
                ("add/add_w_a7_predecrement_d0", "add.w -(a7),d0\nmove.l d7,d1", 40, 800000),
                ("add/add_l_a7_postincrement_d0", "add.l (a7)+,d0\nmove.l d7,d1", 30, 1200000),
            ):
                path = root / case_id
                path.mkdir(parents=True)
                (path / "program.s").write_text(source, encoding="utf-8")
                rows.append({"case_id": case_id, "ns_per_target": ns,
                             "target_count": 100000, "cpu_clocks": clocks,
                             "disturbed": False})
            comparison = anomalies.compare_a7(rows, root)
        self.assertEqual(comparison["a7_case_count"], 3)
        self.assertEqual(len(comparison["comparisons"]), 1)
        self.assertEqual(comparison["comparisons"][0]["ordinary_cases"],
                         ["add/add_w_postincrement"])
        self.assertAlmostEqual(comparison["comparisons"][0]["difference_percent"], 25)

    def test_logical_peer_comparison_skips_different_program_shapes(self):
        rows = []
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for family, mode, source in (
                ("and", "indirect", "and.w (a1),d0\nmove.l d7,d1"),
                ("or", "indirect", "or.w (a2),d3\nmove.l d7,d4"),
                ("and", "indexed", "and.w 2(a1,d0.w),d2\nmove.l d7,d1"),
                ("or", "indexed", "or.w 2(a2,d0.w),d2\nmove.l d7,d1\nmove.l d7,d1"),
            ):
                case_id = f"{family}/{family}_w_{mode}"
                path = root / case_id
                path.mkdir(parents=True)
                (path / "program.s").write_text(source, encoding="utf-8")
                rows.append({"case_id": case_id, "ns_per_target": 30,
                             "target_count": 100000, "cpu_clocks": 800000,
                             "disturbed": False})
            related = anomalies.compare_related(rows, root)
        self.assertEqual(len(related["peer_comparisons"]), 1)
        self.assertEqual(related["peer_comparisons"][0]["family_pair"], "and/or")
        self.assertEqual(related["skipped"]["different program shape"], 1)

    def test_current_snapshot_cannot_be_replaced_by_older_run(self):
        row = {"case_id": "add/add_w_d1_d0", "ns_per_target": 20,
               "target_count": 100000, "cpu_clocks": 800000,
               "fingerprint": "fingerprint", "disturbed": False}
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "m68k_benchmark_current.json"
            snapshot = {"schema_version": 3, "kind": "handwritten_m68000",
                        "source_run": "002", "case_count": 1,
                        "aggregate_csv_sha256": "", "rows": [row]}
            csv_data = anomalies.csv_bytes([row])
            snapshot["aggregate_csv_sha256"] = hashlib.sha256(csv_data).hexdigest()
            self.assertTrue(anomalies.publish_current(path, snapshot, csv_data))
            older = dict(snapshot, source_run="001")
            self.assertFalse(anomalies.publish_current(path, older, csv_data))
            self.assertEqual(anomalies.load_snapshot(path)["source_run"], "002")

    def test_refresh_current_replaces_matching_rows_and_records_sources(self):
        def row(case_id, ns, fingerprint="same"):
            return {"case_id": case_id, "ns_per_target": ns, "target_count": 100000,
                    "cpu_clocks": 800000, "fingerprint": fingerprint, "disturbed": False}

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            host = {"os": "Windows", "architecture": "AMD64", "processor": "host"}
            settings = {"duration_ms": 400, "series": 3}
            baseline_path = root / "m68k_benchmark_baseline.json"
            current_path = root / "m68k_benchmark_current.json"
            baseline_rows = [row("other/other", 20), row("test/test", 20)]
            current_rows = [row("other/other", 21), row("test/test", 24)]
            for path, source, rows in ((baseline_path, "001", baseline_rows),
                                       (current_path, "002", current_rows)):
                csv_data = anomalies.csv_bytes(rows)
                snapshot = {"schema_version": 3, "kind": "handwritten_m68000",
                            "source_run": source, "host": host, "settings": settings,
                            "case_count": 2, "rows": rows,
                            "aggregate_csv_sha256": hashlib.sha256(csv_data).hexdigest()}
                anomalies.write_snapshot(path, snapshot, csv_data)
            bench.write_json(root / "m68k_benchmark_anomalies.json",
                             {"source_run": "002", "series_spread": [
                                 {"case_id": "other/other", "range_pct": 1.0},
                                 {"case_id": "test/test", "range_pct": 12.0}],
                              "load_disturbances": [], "failures": []})
            run = root / "003"
            run.mkdir()
            bench.write_json(run / "manifest.json",
                             {"run_id": "003", "status": "complete", "full_catalog": False,
                              "selected_cases": ["test/test"], "host": host, "settings": settings,
                              "cases": {"test/test": {"status": "measured", "disturbed": False}}})
            replacement = row("test/test", 20)
            bench.write_json(run / "aggregate.json", {"rows": [replacement]})
            (run / "aggregate.csv").write_bytes(anomalies.csv_bytes([replacement]))
            bench.write_json(run / "anomalies.json", {"series_spread": [
                {"case_id": "test/test", "range_pct": 2.0}],
                "load_disturbances": [], "failures": []})
            cases_root = root / "cases"
            cases_root.mkdir()
            with patch.object(bench, "CURRENT", current_path), \
                 patch.object(bench, "BASELINE", baseline_path), \
                 patch.object(bench, "CASES", cases_root):
                bench.refresh_current(run)
            refreshed = anomalies.load_snapshot(current_path)
            self.assertEqual(refreshed["source_run"], "002")
            self.assertEqual(refreshed["case_sources"], {"test/test": "003"})
            self.assertEqual([item["ns_per_target"] for item in refreshed["rows"]], [21, 20])
            self.assertEqual(anomalies.load_snapshot(baseline_path)["rows"], baseline_rows)
            report = (root / "m68k_benchmark_anomaly_report.md").read_text(encoding="utf-8")
            self.assertIn("1 refreshed cases", report)
            self.assertEqual(json.loads((root / "m68k_benchmark_anomalies.json").read_text())
                             ["series_spread"][1]["range_pct"], 2.0)
            older_full = dict(refreshed, source_run="002", case_sources={})
            self.assertFalse(anomalies.publish_current(current_path, older_full,
                                                        anomalies.csv_bytes(refreshed["rows"])))
            replacement["fingerprint"] = "changed"
            bench.write_json(run / "aggregate.json", {"rows": [replacement]})
            (run / "aggregate.csv").write_bytes(anomalies.csv_bytes([replacement]))
            with patch.object(bench, "CURRENT", current_path), self.assertRaisesRegex(
                    ValueError, "workload differs"):
                bench.refresh_current(run)
            self.assertEqual(anomalies.load_snapshot(current_path)["case_sources"],
                             {"test/test": "003"})


if __name__ == "__main__":
    unittest.main()

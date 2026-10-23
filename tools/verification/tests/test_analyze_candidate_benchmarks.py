"""Reject malformed counts and undersized accepted benchmark series."""

from pathlib import Path
import hashlib
import json
import statistics
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from analyze_candidate_benchmarks import (compare_baseline, control_for_case,
                                          controls_for_run, prepare_retry_plan,
                                          validate_measurement_series,
                                          validate_planned_retry)


def measurement(elapsed_ns=400_000_000, target_count=700_000):
    ns = elapsed_ns / target_count
    samples = [
        {"attempt": 0, "ordinal": ordinal, "blocks": 1000,
         "target_count": target_count, "elapsed_ns": elapsed_ns, "ns_per_target": ns}
        for ordinal in range(3)
    ]
    return {
        "case_id": "synthetic-contract", "series": samples,
        "blocks_per_series": 1000, "target_duration_capped": False,
        "status": "measured", "dispersion_pct": 0.0,
        "selected_median_ns_per_target": statistics.median(sample["ns_per_target"] for sample in samples),
    }


class CandidateReportContracts(unittest.TestCase):
    def test_valid_fixed_count_series_is_accepted(self):
        row = measurement()
        self.assertEqual(len(validate_measurement_series(row, 700)), 3)

    def test_declared_target_count_must_match_actual_blocks(self):
        row = measurement(target_count=699_000)
        with self.assertRaisesRegex(ValueError, "invalid target divisor"):
            validate_measurement_series(row, 700)

    def test_short_series_cannot_be_reported_as_valid(self):
        row = measurement(elapsed_ns=70_000_000)
        with self.assertRaisesRegex(ValueError, "accepted a short or long series"):
            validate_measurement_series(row, 700)

    def test_unstable_series_cannot_publish_a_median(self):
        row = measurement()
        row["status"] = "unstable"
        with self.assertRaisesRegex(ValueError, "invalid sample accepted"):
            validate_measurement_series(row, 700)

    def test_one_initial_control_covers_all_candidates(self):
        control = {"role": "control", "case_id": "nop"}
        raw = [control] + [{"role": "candidate"} for _ in range(201)]
        controls = controls_for_run(raw, 201, "once_at_start")
        self.assertIs(control_for_case(controls, 200, "once_at_start"), control)
        with self.assertRaisesRegex(ValueError, "expected 3 NOP controls"):
            controls_for_run(raw, 201, "every_100")

    def test_initial_control_uses_longer_target_duration(self):
        row = measurement(elapsed_ns=1_200_000_000)
        row["target_series_ns"] = 1_200_000_000
        self.assertEqual(len(validate_measurement_series(row, 700, 1_200_000_000)), 3)
        with self.assertRaisesRegex(ValueError, "wrong target series duration"):
            validate_measurement_series(row, 700, 400_000_000)

    def test_baseline_comparison_requires_same_executed_fixture(self):
        old = {"case_id": "one", "representative_opcode": "4E71", "path": "normal",
               "cpu_mode": "supervisor", "fixture_sha256": "fixture", "target_count_per_block": 700,
               "clocks_per_block": 2800, "accepted_for_comparison": True,
               "selected_median_ns_per_target": 20.0, "control_median_ns_per_target": 10.0}
        env = {"host_os": "Windows", "host_arch": "x86_64"}
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "baseline.json"
            csv_data = ("case_id,fixture_sha256,selected_median_ns_per_target,control_median_ns_per_target\n"
                        "one,fixture,20.0,10.0\n").encode()
            path.with_suffix(".csv").write_bytes(csv_data)
            path.write_text(json.dumps({"schema_version": 2, "kind": "decoder_derived_m68000",
                                        "full_run": "full_test", "retry_run": None,
                                        "case_csv_sha256": hashlib.sha256(csv_data).hexdigest(),
                                        "summary": {"environment": env}, "results": [old]}), encoding="utf-8")
            same, summary = compare_baseline([old], env, path)
            self.assertEqual((same[0]["status"], summary["comparable_case_count"]), ("stable", 1))
            changed = dict(old, selected_median_ns_per_target=22.0)
            slower, _ = compare_baseline([changed], env, path)
            self.assertEqual(slower[0]["status"], "regression")
            self.assertAlmostEqual(slower[0]["raw_delta_pct"], 10.0)
            self.assertEqual(slower[0]["current_ns_per_target"], 22.0)
            changed["fixture_sha256"] = "other"
            mismatched, _ = compare_baseline([changed], env, path)
            self.assertEqual(mismatched[0]["status"], "incompatible")
            new_policy, _ = compare_baseline([old], dict(env, control_policy="once_at_start"), path)
            self.assertEqual(new_policy[0]["reason"], "NOP control measurement policy differs")

    def test_retry_plan_selects_raw_outlier_and_invalid_case_without_replacing_baseline(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            inventory = root / "inventory"
            run = inventory / "full_100"
            run.mkdir(parents=True)
            manifest = [{"case_id": name, "representative_opcode": "4E71",
                         "path": "normal", "cpu_mode": "supervisor"}
                        for name in ("slow", "fast", "same", "invalid")]
            probes = [{"case_id": name, "fixture_sha256": name,
                       "block_targets": 700, "block_clocks": 2800} for name in ("slow", "fast", "same", "invalid")]
            manifest_bytes = json.dumps(manifest).encode()
            probe_bytes = json.dumps(probes).encode()
            (inventory / "case_candidates.json").write_bytes(manifest_bytes)
            (inventory / "candidate_probe.json").write_bytes(probe_bytes)
            rows = [{"case_id": "nop", "role": "control", "status": "measured"}]
            for name, ns, status in (("slow", 12.0, "measured"),
                                     ("fast", 8.0, "measured"),
                                     ("same", 10.0, "measured"),
                                     ("invalid", None, "unstable")):
                rows.append({"case_id": name, "role": "candidate", "status": status,
                             "selected_median_ns_per_target": ns,
                             "fixture_sha256": name, "target_count_per_block": 700,
                             "clocks_per_block": 2800})
            raw = "".join(json.dumps(row) + "\n" for row in rows).encode()
            (run / "measurements.jsonl").write_bytes(raw)
            environment = {"pilot": False, "retry_source": None,
                           "control_policy": "once_at_start", "host_os": "Windows",
                           "host_arch": "x86_64", "active_power_plan": "plan",
                           "priority_class": 32, "core_id": 4, "affinity_verified": True,
                           "candidate_manifest_sha256": hashlib.sha256(manifest_bytes).hexdigest(),
                           "validation_sha256": hashlib.sha256(probe_bytes).hexdigest()}
            (run / "environment.json").write_text(json.dumps(environment), encoding="utf-8")
            baseline_rows = [{"case_id": name, "representative_opcode": "4E71",
                              "path": "normal", "cpu_mode": "supervisor",
                              "fixture_sha256": name, "target_count_per_block": 700,
                              "clocks_per_block": 2800, "accepted_for_comparison": True,
                              "selected_median_ns_per_target": 10.0,
                              "control_median_ns_per_target": 5.0}
                             for name in ("slow", "fast", "same", "invalid")]
            baseline_csv = ("case_id,fixture_sha256,selected_median_ns_per_target,control_median_ns_per_target\n"
                            + "".join(f"{name},{name},10.0,5.0\n" for name in ("slow", "fast", "same", "invalid"))).encode()
            baseline_path = root / "baseline.json"
            baseline_path.with_suffix(".csv").write_bytes(baseline_csv)
            baseline_content = {"schema_version": 2, "kind": "decoder_derived_m68000",
                                "case_csv_sha256": hashlib.sha256(baseline_csv).hexdigest(),
                                "summary": {"environment": dict(environment, control_policy="every_100")},
                                "results": baseline_rows}
            baseline_path.write_text(json.dumps(baseline_content), encoding="utf-8")
            baseline_before = baseline_path.read_bytes()
            plan = prepare_retry_plan(run, baseline_path)
            self.assertEqual([(row["case_id"], row["reason"]) for row in plan["selected"]],
                             [("slow", "raw_time_difference"), ("fast", "raw_time_difference"),
                              ("invalid", "invalid")])
            self.assertEqual(plan["selection_policy"], "provisional_raw_time")
            self.assertEqual(baseline_path.read_bytes(), baseline_before)
            self.assertEqual(json.loads((run / "retry_plan.json").read_text(encoding="utf-8")), plan)

    def test_retry_summary_must_match_a_confirmed_preserved_attempt(self):
        with tempfile.TemporaryDirectory() as temporary:
            full = Path(temporary) / "full_100"
            retry = Path(temporary) / "retry_200"
            full.mkdir()
            retry.mkdir()
            original = {"case_id": "one", "role": "candidate", "status": "measured",
                        "selected_median_ns_per_target": 400_000_000 / 700_000}
            raw = (json.dumps(original) + "\n").encode()
            (full / "measurements.jsonl").write_bytes(raw)
            plan = {"version": 1, "source_run": full.name,
                    "full_measurements_sha256": hashlib.sha256(raw).hexdigest(),
                    "max_repeat_attempts": 3, "agreement_tolerance_pct": 3.0,
                    "selected": [{"case_id": "one"}]}
            plan_bytes = (json.dumps(plan) + "\n").encode()
            (full / "retry_plan.json").write_bytes(plan_bytes)
            attempt = dict(measurement(), case_id="one", role="retry_attempt", retry_repeat=1,
                           fixture_sha256="fixture", target_count_per_block=700,
                           clocks_per_block=2800)
            summary = dict(attempt, role="candidate", confirmed_with=0)
            retry_env = {"retry_plan_sha256": hashlib.sha256(plan_bytes).hexdigest(),
                         "target_series_ns": 400_000_000}
            probes = [{"case_id": "one", "fixture_sha256": "fixture",
                       "block_targets": 700, "block_clocks": 2800}]
            self.assertEqual(validate_planned_retry(full, retry_env,
                                                    [attempt, summary], [original], probes), ["one"])
            summary["confirmed_with"] = 1
            with self.assertRaisesRegex(ValueError, "missing retry confirmation"):
                validate_planned_retry(full, retry_env,
                                       [attempt, summary], [original], probes)


if __name__ == "__main__":
    unittest.main()

"""Validate and summarize one decoder-derived instruction benchmark run."""

import argparse
import csv
import hashlib
import json
import statistics
from collections import Counter, defaultdict
from pathlib import Path

BASELINE_PATH = Path(__file__).resolve().parents[2] / "baselines" / "benchmarks" / "m68k_benchmark_baseline.json"
BASELINE_CSV = BASELINE_PATH.with_suffix(".csv")
HOST_IDENTITY_KEYS = ("host_os", "host_arch")
HOST_SETTING_KEYS = ("active_power_plan", "priority_class", "core_id", "affinity_verified")


def read_csv(path):
    with path.open(encoding="utf-8-sig", newline="") as source:
        return list(csv.DictReader(source))


def write_csv(path, rows, fields):
    with path.open("w", encoding="utf-8", newline="") as target:
        writer = csv.DictWriter(target, fieldnames=fields, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)


def ensure(test, message):
    if not test:
        raise ValueError(message)


def control_policy(environment):
    policy = environment.get("control_policy", "every_100")
    ensure(policy in ("every_100", "once_at_start"), f"unknown NOP control policy: {policy}")
    return policy


def controls_for_run(raw, candidate_count, policy):
    controls = [row for row in raw if row.get("role") == "control"]
    expected = 1 if policy == "once_at_start" else (candidate_count + 99) // 100
    ensure(len(controls) == expected, f"expected {expected} NOP controls, found {len(controls)}")
    ensure(raw and raw[0].get("role") == "control", "NOP control must precede candidates")
    return controls


def control_for_case(controls, index, policy):
    return controls[0 if policy == "once_at_start" else index // 100]


def load_baseline(baseline_path):
    baseline = json.loads(baseline_path.read_text(encoding="utf-8"))
    ensure(baseline.get("schema_version") == 2 and baseline.get("kind") == "decoder_derived_m68000",
           "benchmark baseline schema is incompatible")
    baseline_csv = baseline_path.with_suffix(".csv")
    ensure(hashlib.sha256(baseline_csv.read_bytes()).hexdigest() == baseline["case_csv_sha256"],
           "benchmark baseline CSV hash differs from JSON")
    baseline_rows = read_csv(baseline_csv)
    ensure(len(baseline_rows) == len(baseline["results"]), "baseline JSON and CSV row counts differ")
    for typed, flat in zip(baseline["results"], baseline_rows):
        ensure(all(str(typed[key]) == flat[key] for key in (
            "case_id", "fixture_sha256", "selected_median_ns_per_target",
            "control_median_ns_per_target")), "baseline JSON and CSV results differ")
    return baseline


def comparable_host(environment, previous_environment):
    return all(environment.get(key) == previous_environment.get(key)
               for key in HOST_IDENTITY_KEYS + HOST_SETTING_KEYS)


def prepare_retry_plan(run_dir, baseline_path=BASELINE_PATH, threshold_pct=3.0):
    """Select invalid cases and provisional raw-time outliers before final analysis."""
    inventory = run_dir.parent
    environment = json.loads((run_dir / "environment.json").read_text(encoding="utf-8"))
    ensure(control_policy(environment) == "once_at_start", "retry requires a new-policy full run")
    ensure(not environment["pilot"] and environment["retry_source"] is None, "retry source is not a full run")
    manifest_bytes = (inventory / "case_candidates.json").read_bytes()
    probe_bytes = (inventory / "candidate_probe.json").read_bytes()
    ensure(hashlib.sha256(manifest_bytes).hexdigest() == environment["candidate_manifest_sha256"],
           "candidate manifest changed since full run")
    ensure(hashlib.sha256(probe_bytes).hexdigest() == environment["validation_sha256"],
           "validation record changed since full run")
    manifest = json.loads(manifest_bytes)
    probes = json.loads(probe_bytes)
    raw_bytes = (run_dir / "measurements.jsonl").read_bytes()
    raw = [json.loads(line) for line in raw_bytes.splitlines() if line.strip()]
    candidates = [row for row in raw if row.get("role") == "candidate"]
    controls = controls_for_run(raw, len(manifest), "once_at_start")
    ensure(len(candidates) == len(manifest) == len(probes), "full run is incomplete")
    ensure(controls[0]["status"] == "measured", "full-run NOP control is invalid")
    ensure(all(row["case_id"] == case["case_id"] == probe["case_id"]
               for row, case, probe in zip(candidates, manifest, probes)), "full-run case order differs")
    baseline = load_baseline(baseline_path)
    previous = {row["case_id"]: row for row in baseline["results"]}
    ensure(len(previous) == len(baseline["results"]), "duplicate baseline case ID")
    same_host = comparable_host(environment, baseline["summary"]["environment"])
    selected = []
    comparable_count = 0
    for row, case, probe in zip(candidates, manifest, probes):
        if row["status"] != "failed":
            ensure(row["fixture_sha256"] == probe["fixture_sha256"]
                   and row["target_count_per_block"] == probe["block_targets"]
                   and row["clocks_per_block"] == probe["block_clocks"],
                   f"{row['case_id']}: full-run fixture changed")
        old = previous.get(row["case_id"])
        current_ns = row.get("selected_median_ns_per_target")
        reason = None
        delta = None
        if row["status"] != "measured":
            reason = "invalid"
        elif (same_host and old is not None and old.get("accepted_for_comparison")
              and all(str(old.get(key)) == str(value) for key, value in (
                  ("representative_opcode", case["representative_opcode"]),
                  ("path", case["path"]), ("cpu_mode", case["cpu_mode"]),
                  ("fixture_sha256", probe["fixture_sha256"]),
                  ("target_count_per_block", probe["block_targets"]),
                  ("clocks_per_block", probe["block_clocks"])))
              and current_ns is not None and float(old["selected_median_ns_per_target"]) > 0):
            comparable_count += 1
            delta = 100 * (float(current_ns) / float(old["selected_median_ns_per_target"]) - 1)
            if abs(delta) > threshold_pct:
                reason = "raw_time_difference"
        if reason is not None:
            selected.append({"case_id": row["case_id"], "reason": reason,
                             "full_ns_per_target": current_ns, "raw_delta_pct": delta})
    plan = {
        "version": 1, "source_run": run_dir.name,
        "full_measurements_sha256": hashlib.sha256(raw_bytes).hexdigest(),
        "candidate_manifest_sha256": environment["candidate_manifest_sha256"],
        "validation_sha256": environment["validation_sha256"],
        "baseline_csv_sha256": baseline["case_csv_sha256"],
        "baseline_control_policy": control_policy(baseline["summary"]["environment"]),
        "selection_policy": ("unavailable" if not same_host else
                             "provisional_raw_time" if control_policy(environment) != control_policy(baseline["summary"]["environment"]) else
                             "matching_raw_time"),
        "comparable_case_count": comparable_count,
        "difference_threshold_pct": threshold_pct,
        "agreement_tolerance_pct": 3.0,
        "max_repeat_attempts": 3,
        "selected": selected,
    }
    (run_dir / "retry_plan.json").write_text(json.dumps(plan, indent=2) + "\n", encoding="utf-8")
    return plan


def compare_baseline(case_rows, environment, baseline_path=BASELINE_PATH):
    """Compare identical executed fixtures; retain raw and control-adjusted deltas."""
    baseline = load_baseline(baseline_path)
    previous = {row["case_id"]: row for row in baseline["results"]}
    ensure(len(previous) == len(baseline["results"]), "duplicate baseline case ID")
    current = {row["case_id"]: row for row in case_rows}
    ensure(len(current) == len(case_rows), "duplicate current case ID")
    same_host = all(environment[key] == baseline["summary"]["environment"][key]
                    for key in HOST_IDENTITY_KEYS)
    same_settings = all(environment.get(key) == baseline["summary"]["environment"].get(key)
                        for key in HOST_SETTING_KEYS)
    same_policy = control_policy(environment) == control_policy(baseline["summary"]["environment"])
    rows = []
    for case_id in sorted(set(previous) | set(current)):
        old, new = previous.get(case_id), current.get(case_id)
        result = {"case_id": case_id, "status": "", "raw_delta_pct": None,
                  "control_adjusted_delta_pct": None, "reason": "",
                  "baseline_ns_per_target": old.get("selected_median_ns_per_target") if old else None,
                  "current_ns_per_target": new.get("selected_median_ns_per_target") if new else None,
                  "baseline_control_ns": old.get("control_median_ns_per_target") if old else None,
                  "current_control_ns": new.get("control_median_ns_per_target") if new else None,
                  "baseline_dispersion_pct": old.get("dispersion_pct") if old else None,
                  "current_dispersion_pct": new.get("dispersion_pct") if new else None}
        if old is None or new is None:
            result.update(status="incompatible", reason="case missing from baseline or current run")
        elif not same_host:
            result.update(status="incompatible", reason="host OS or architecture differs")
        elif not same_settings:
            result.update(status="incompatible", reason="host power, priority or affinity setting differs")
        elif not same_policy:
            result.update(status="incompatible", reason="NOP control measurement policy differs")
        elif any(str(old.get(key)) != str(new.get(key)) for key in (
                "representative_opcode", "path", "cpu_mode", "fixture_sha256",
                "target_count_per_block", "clocks_per_block")):
            result.update(status="incompatible", reason="executed fixture or clocks differ")
        elif not old.get("accepted_for_comparison") or not new.get("accepted_for_comparison"):
            result.update(status="invalid", reason="case or NOP control did not pass measurement validation")
        else:
            current_ns = float(new["selected_median_ns_per_target"])
            old_ns = float(old["selected_median_ns_per_target"])
            current_control = float(new["control_median_ns_per_target"])
            old_control = float(old["control_median_ns_per_target"])
            ensure(min(current_ns, old_ns, current_control, old_control) > 0,
                   f"{case_id}: benchmark time must be positive")
            raw_delta = 100 * (current_ns / old_ns - 1)
            adjusted_delta = 100 * ((current_ns / current_control) / (old_ns / old_control) - 1)
            status = ("regression" if adjusted_delta > 7 else
                      "warning" if adjusted_delta > 3 else
                      "improvement" if adjusted_delta < -3 else "stable")
            result.update(status=status, raw_delta_pct=raw_delta,
                          control_adjusted_delta_pct=adjusted_delta)
        rows.append(result)
    counts = dict(Counter(row["status"] for row in rows))
    return rows, {"version": 2, "baseline": baseline_path.name,
                  "baseline_full_run": baseline["full_run"],
                  "baseline_retry_run": baseline["retry_run"],
                  "case_count": len(rows), "comparable_case_count": sum(
                      row["control_adjusted_delta_pct"] is not None for row in rows),
                  "status_counts": counts,
                  "method": "Matching fixture and CPU clocks with the same host OS, architecture, power plan, priority, affinity and NOP control policy; classification uses target/NOP ratio change: >3% warning, >7% regression. These labels are diagnostic, not correctness gates."}


def mode_name(opcode):
    mode = (opcode >> 3) & 7
    reg = opcode & 7
    if mode == 7:
        return {0: "absolute_word", 1: "absolute_long", 2: "pc_displacement", 3: "pc_index", 4: "immediate"}.get(reg, "other")
    return ("data_register", "address_register", "indirect", "postincrement", "predecrement", "displacement", "index")[mode]


def workload_family(opcode, path):
    if path in ("divide_zero", "overflow", "overflow_positive", "overflow_negative"):
        return "divide_exception"
    if path in ("above_bound", "negative", "privilege_violation", "trap") or 0x4E40 <= opcode <= 0x4E4F:
        return "instruction_exception"
    if opcode >> 12 == 6 or opcode & 0xF0F8 == 0x50C8:
        return "conditional_control"
    if opcode & 0xFB80 == 0x4880 and ((opcode >> 3) & 7) >= 2:
        return "movem"
    if opcode & 0xF1C0 in (0x80C0, 0x81C0, 0xC0C0, 0xC1C0):
        return "multiply_divide"
    if 0x4E70 <= opcode <= 0x4E77 or 0x4E60 <= opcode <= 0x4E6F:
        return "system_stack"
    return "ordinary"


def default_register_choices(case):
    return "1" not in case["a7_choices"] and all(ch in "(), 0" for ch in case["index_choices"])


def selected_series(row):
    ensure(len(row["series"]) in (3, 6), f"{row['case_id']}: unexpected series count")
    return row["series"][-3:]


def validate_measurement_series(row, block_targets, target_series_ns=400_000_000):
    ensure(row.get("target_series_ns", target_series_ns) == target_series_ns,
           f"{row['case_id']}: wrong target series duration")
    selected = selected_series(row)
    for sample in row["series"]:
        ensure(sample["blocks"] == row["blocks_per_series"], f"{row['case_id']}: variable series count")
        ensure(sample["target_count"] == sample["blocks"] * block_targets, f"{row['case_id']}: invalid target divisor")
        ensure(abs(sample["ns_per_target"] - sample["elapsed_ns"] / sample["target_count"]) < 1e-9, f"{row['case_id']}: invalid ns/target")
    if row["status"] == "measured":
        ensure(row["selected_median_ns_per_target"] is not None, f"{row['case_id']}: missing median")
        ensure(row["dispersion_pct"] <= 5.0, f"{row['case_id']}: accepted noisy case")
        ensure(abs(row["selected_median_ns_per_target"] - statistics.median(sample["ns_per_target"] for sample in selected)) < 1e-9, f"{row['case_id']}: wrong median")
        if not row["target_duration_capped"]:
            ensure(all(target_series_ns * 3 // 4 <= sample["elapsed_ns"] <= target_series_ns * 5 // 4
                       for sample in selected), f"{row['case_id']}: accepted a short or long series")
    else:
        ensure(row["selected_median_ns_per_target"] is None, f"{row['case_id']}: invalid sample accepted")
    return selected


def retry_agrees(first, second, tolerance_pct):
    return first > 0 and second > 0 and abs(first - second) * 200 / (first + second) <= tolerance_pct


def validate_planned_retry(run_dir, retry_env, retry_raw, full_candidates, probes):
    plan_bytes = (run_dir / "retry_plan.json").read_bytes()
    ensure(hashlib.sha256(plan_bytes).hexdigest() == retry_env["retry_plan_sha256"],
           "retry plan hash differs from measurement environment")
    plan = json.loads(plan_bytes)
    ensure(plan["version"] == 1 and plan["source_run"] == run_dir.name,
           "retry plan source differs from full run")
    ensure(plan["full_measurements_sha256"] == hashlib.sha256((run_dir / "measurements.jsonl").read_bytes()).hexdigest(),
           "full run changed after retry selection")
    selected_ids = [item["case_id"] for item in plan["selected"]]
    ensure(len(set(selected_ids)) == len(selected_ids), "duplicate case in retry plan")
    retry_candidates = [row for row in retry_raw if row.get("role") == "candidate"]
    ensure(set(row["case_id"] for row in retry_candidates) == set(selected_ids)
           and len(retry_candidates) == len(selected_ids), "retry summaries differ from plan")
    full_by_id = {row["case_id"]: row for row in full_candidates}
    probe_by_id = {row["case_id"]: row for row in probes}
    attempts = defaultdict(list)
    for row in retry_raw:
        if row.get("role") != "retry_attempt":
            continue
        case_id = row["case_id"]
        ensure(case_id in selected_ids, f"unexpected retry attempt: {case_id}")
        attempts[case_id].append(row)
        ensure(row["retry_repeat"] == len(attempts[case_id]), f"{case_id}: retry order differs")
        if row["status"] != "failed":
            probe = probe_by_id[case_id]
            ensure(row["fixture_sha256"] == probe["fixture_sha256"]
                   and row["target_count_per_block"] == probe["block_targets"]
                   and row["clocks_per_block"] == probe["block_clocks"],
                   f"{case_id}: retry fixture changed")
            validate_measurement_series(row, probe["block_targets"], retry_env["target_series_ns"])
    for row in retry_candidates:
        case_id = row["case_id"]
        case_attempts = attempts[case_id]
        ensure(len(case_attempts) <= plan["max_repeat_attempts"], f"{case_id}: too many retry attempts")
        if row["status"] == "measured":
            repeat = row["retry_repeat"]
            prior = row["confirmed_with"]
            ensure(repeat == len(case_attempts) and 0 <= prior < repeat,
                   f"{case_id}: missing retry confirmation")
            source = case_attempts[repeat - 1]
            ensure(source["status"] == "measured" and source["series"] == row["series"],
                   f"{case_id}: retry summary differs from measured attempt")
            previous = full_by_id[case_id] if prior == 0 else case_attempts[prior - 1]
            ensure(previous["status"] == "measured" and retry_agrees(
                previous["selected_median_ns_per_target"], row["selected_median_ns_per_target"],
                plan["agreement_tolerance_pct"]), f"{case_id}: retry confirmation does not agree")
        else:
            ensure(row["status"] in ("unresolved", "failed"), f"{case_id}: unknown retry status")
            ensure(len(case_attempts) == plan["max_repeat_attempts"] or not case_attempts,
                   f"{case_id}: retry ended before attempt limit")
            if case_attempts and row["status"] == "unresolved":
                last_result = next((entry for entry in reversed(case_attempts)
                                    if entry["status"] != "failed"), None)
                ensure(last_result is not None and row["retry_repeat"] == last_result["retry_repeat"],
                       f"{case_id}: unresolved retry does not show the last result")
    return selected_ids


def analyze(run_dir, retry_dir=None):
    inventory = run_dir.parent
    env = json.loads((run_dir / "environment.json").read_text(encoding="utf-8"))
    retry_plan = (prepare_retry_plan(run_dir)
                  if control_policy(env) == "once_at_start" and not env["pilot"] else None)
    pending_retry = bool(retry_plan and retry_plan["selected"] and retry_dir is None)
    snapshot = json.loads((run_dir / "source_snapshot.json").read_text(encoding="utf-8"))
    manifest = json.loads((inventory / "case_candidates.json").read_text(encoding="utf-8"))
    manifest_csv = read_csv(inventory / "case_candidates.csv")
    ensure(manifest == manifest_csv, "candidate JSON and CSV disagree")
    probes = json.loads((inventory / "candidate_probe.json").read_text(encoding="utf-8"))
    ensure(len(manifest) == len(probes) == env["candidate_count"] and manifest, "candidate coverage changed")
    ensure(len({entry["case_id"] for entry in manifest}) == len(manifest), "duplicate candidate ID")
    ensure(all(row["case_id"] == probe["case_id"] for row, probe in zip(manifest, probes)), "probe ordering differs")
    ensure(all(probe["replay_status"] == "identical" and probe["block_targets"] in (1, 700) for probe in probes), "validation before timing incomplete")
    coverage = read_csv(inventory / "opcode_coverage.csv")
    ensure(len(coverage) == 65536, "opcode coverage CSV is incomplete")
    legal_opcode_count = sum(row["decoder_status"] == "legal" for row in coverage)
    ensure(legal_opcode_count > 0, "no legal decoder opcodes in coverage")
    ensure(all(int(row["candidate_count"]) > 0 for row in coverage if row["decoder_status"] == "legal"), "legal opcode has no candidate group")
    coverage_counts = Counter(row["coverage_role"] for row in coverage)
    raw = [json.loads(line) for line in (run_dir / "measurements.jsonl").read_text(encoding="utf-8").splitlines() if line.strip()]
    candidates = [entry for entry in raw if entry.get("role") == "candidate"]
    full_policy = control_policy(env)
    controls = controls_for_run(raw, len(manifest), full_policy)
    ensure(len(candidates) == len(manifest), f"incomplete run: {len(candidates)}/{len(manifest)} candidates")
    ensure([entry["case_id"] for entry in candidates] == [entry["case_id"] for entry in manifest], "candidate run order or identity differs")
    ensure(all(entry["case_id"] == "4E71-000:normal:I-:A-:S" for entry in controls), "wrong NOP control")
    original_status_counts = Counter(row["status"] for row in candidates)
    retry_rows = {}
    retry_controls = []
    retry_control_by_id = {}
    retry_env = None
    retry_snapshot = None
    if retry_dir is not None:
        retry_env = json.loads((retry_dir / "environment.json").read_text(encoding="utf-8"))
        retry_snapshot = json.loads((retry_dir / "source_snapshot.json").read_text(encoding="utf-8"))
        ensure(retry_env["retry_source"] == run_dir.name, "retry source does not match full run")
        ensure(retry_env["candidate_manifest_sha256"] == env["candidate_manifest_sha256"], "retry used a different candidate manifest")
        ensure(retry_env["validation_sha256"] == env["validation_sha256"], "retry used a different validation record")
        retry_policy = control_policy(retry_env)
        ensure(retry_policy == full_policy, "retry used a different NOP control policy")
        retry_raw = [json.loads(line) for line in (retry_dir / "measurements.jsonl").read_text(encoding="utf-8").splitlines() if line.strip()]
        retry_candidates = [row for row in retry_raw if row.get("role") == "candidate"]
        retry_controls = controls_for_run(retry_raw, len(retry_candidates), retry_policy)
        ensure(all(row["status"] == "measured" for row in retry_controls), "retry NOP control invalid")
        retry_rows = {row["case_id"]: row for row in retry_candidates}
        if retry_env.get("retry_plan_sha256"):
            selected_ids = validate_planned_retry(
                run_dir, retry_env, retry_raw, candidates, probes)
            ensure(len(retry_rows) == len(retry_candidates) == len(selected_ids),
                   "retry rows are missing or duplicated")
        else:
            invalid_ids = {row["case_id"] for index, row in enumerate(candidates)
                           if row["status"] != "measured" or control_for_case(controls, index, full_policy)["status"] != "measured"}
            ensure(len(retry_rows) == len(retry_candidates) == len(invalid_ids), "retry rows are missing or duplicated")
            ensure(set(retry_rows) == invalid_ids, "retry set differs from invalid full-run cases")
            ensure(all(row["status"] == "measured" for row in retry_candidates), "retry still has invalid measurements")
        retry_control_by_id = {row["case_id"]: control_for_case(retry_controls, index, retry_policy)
                               for index, row in enumerate(retry_candidates)}
    probe_by_id = {row["case_id"]: row for row in probes}
    case_by_id = {row["case_id"]: row for row in manifest}
    case_rows = []
    control_rows = []
    for segment, control in enumerate(controls):
        selected = validate_measurement_series(control, control["target_count_per_block"],
                                               env.get("control_target_series_ns", env["target_series_ns"]))
        control_rows.append({
            "segment": segment,
            "source": run_dir.name,
            "before_candidate": 0 if full_policy == "once_at_start" else segment * 100,
            "status": control["status"],
            "median_ns_per_target": control["selected_median_ns_per_target"],
            "dispersion_pct": control["dispersion_pct"],
            "series_ms": "|".join(f"{sample['elapsed_ns'] / 1e6:.3f}" for sample in selected),
        })
    for segment, control in enumerate(retry_controls):
        selected = validate_measurement_series(control, control["target_count_per_block"],
                                               retry_env.get("control_target_series_ns", retry_env["target_series_ns"]))
        control_rows.append({
            "segment": f"retry-{segment}", "source": retry_dir.name,
            "before_candidate": 0 if retry_policy == "once_at_start" else segment * 100,
            "status": control["status"],
            "median_ns_per_target": control["selected_median_ns_per_target"],
            "dispersion_pct": control["dispersion_pct"],
            "series_ms": "|".join(f"{sample['elapsed_ns'] / 1e6:.3f}" for sample in selected),
        })
    for index, (raw_case, case) in enumerate(zip(candidates, manifest)):
        original_status = raw_case["status"]
        source_name = run_dir.name
        control = control_for_case(controls, index, full_policy)
        if case["case_id"] in retry_rows:
            raw_case = retry_rows[case["case_id"]]
            control = retry_control_by_id[case["case_id"]]
            source_name = retry_dir.name
        probe = probe_by_id[case["case_id"]]
        if raw_case["status"] == "failed":
            case_rows.append({
                "schema_version": 1, "measurement_source": source_name,
                "original_status": original_status, "case_id": case["case_id"],
                "semantic_group": case["semantic_group"], "representative_opcode": case["representative_opcode"],
                "path": case["path"], "cpu_mode": case["cpu_mode"],
                "index_choices": case["index_choices"], "a7_choices": case["a7_choices"],
                "category": case["category"], "ea_mode": mode_name(int(case["representative_opcode"], 16)),
                "workload_family": workload_family(int(case["representative_opcode"], 16), case["path"]),
                "status": "failed", "control_status": control["status"],
                "accepted_for_comparison": False, "failure_reason": raw_case["reason"],
            })
            continue
        ensure(raw_case["fixture_sha256"] == probe["fixture_sha256"], f"{case['case_id']}: fixture changed")
        ensure(raw_case["target_count_per_block"] == probe["block_targets"], f"{case['case_id']}: target count changed")
        ensure(raw_case["clocks_per_block"] == probe["block_clocks"], f"{case['case_id']}: CPU clock count changed")
        selected = validate_measurement_series(raw_case, probe["block_targets"], env["target_series_ns"] if source_name == run_dir.name else retry_env["target_series_ns"])
        status = raw_case["status"]
        opcode = int(case["representative_opcode"], 16)
        clocks_per_target = probe["block_clocks"] / probe["block_targets"]
        ns = raw_case["selected_median_ns_per_target"] if status == "measured" and control["status"] == "measured" else None
        case_rows.append({
            "schema_version": 1, "measurement_source": source_name,
            "original_status": original_status,
            "case_id": case["case_id"], "semantic_group": case["semantic_group"],
            "representative_opcode": case["representative_opcode"], "path": case["path"],
            "cpu_mode": case["cpu_mode"], "index_choices": case["index_choices"],
            "a7_choices": case["a7_choices"], "category": case["category"],
            "ea_mode": mode_name(opcode), "workload_family": workload_family(opcode, case["path"]),
            "status": status, "control_status": control["status"], "failure_reason": None,
            "accepted_for_comparison": ns is not None,
            "fixture_sha256": raw_case["fixture_sha256"], "target_count_per_block": probe["block_targets"],
            "clocks_per_block": probe["block_clocks"], "clocks_per_target_composite": clocks_per_target,
            "blocks_per_series": raw_case["blocks_per_series"], "target_duration_capped": raw_case["target_duration_capped"],
            "selected_median_ns_per_target": ns, "min_ns_per_target": raw_case["min_ns_per_target"],
            "max_ns_per_target": raw_case["max_ns_per_target"], "dispersion_pct": raw_case["dispersion_pct"],
            "selected_series_ms": "|".join(f"{sample['elapsed_ns'] / 1e6:.3f}" for sample in selected),
            "control_median_ns_per_target": control["selected_median_ns_per_target"],
            "host_ns_per_cpu_clock_composite": ns / clocks_per_target if ns is not None else None,
        })
    fields = list(dict.fromkeys(field for row in case_rows for field in row))
    write_csv(run_dir / "cases.csv", case_rows, fields)
    write_csv(run_dir / "controls.csv", control_rows, list(control_rows[0]))
    clean = [row for row in case_rows if row["accepted_for_comparison"]
             and row["measurement_source"] == run_dir.name
             and default_register_choices(case_by_id[row["case_id"]])]
    by_mode = defaultdict(dict)
    for row in clean:
        opcode = int(row["representative_opcode"], 16)
        if opcode >> 12 not in (0, 1, 2, 3, 4, 5, 8, 9, 11, 12, 13):
            continue
        signature = (opcode & 0xFFC0, row["path"], row["cpu_mode"])
        by_mode[signature].setdefault(row["ea_mode"], row)
    mode_pairs = []
    for signature, modes in by_mode.items():
        base = modes.get("data_register")
        if base is None:
            continue
        for mode, row in modes.items():
            if mode == "data_register":
                continue
            mode_pairs.append({
                "signature": f"{signature[0]:04X}", "path": signature[1], "cpu_mode": signature[2],
                "base_case": base["case_id"], "other_case": row["case_id"], "other_ea_mode": mode,
                "base_ns_per_target": base["selected_median_ns_per_target"],
                "other_ns_per_target": row["selected_median_ns_per_target"],
                "delta_ns_per_target": row["selected_median_ns_per_target"] - base["selected_median_ns_per_target"],
                "delta_cpu_clocks_per_target": row["clocks_per_target_composite"] - base["clocks_per_target_composite"],
            })
    mode_fields = ["signature", "path", "cpu_mode", "base_case", "other_case", "other_ea_mode", "base_ns_per_target", "other_ns_per_target", "delta_ns_per_target", "delta_cpu_clocks_per_target"]
    write_csv(run_dir / "address_mode_comparisons.csv", mode_pairs, mode_fields)
    by_size = defaultdict(dict)
    for row in clean:
        opcode = int(row["representative_opcode"], 16)
        family = opcode >> 12
        if family in (1, 2, 3):
            key, size = (opcode & 0x0FFF, row["path"], row["cpu_mode"]), {1: "byte", 3: "word", 2: "long"}[family]
        elif family in (0, 5, 8, 9, 11, 12, 13, 14) and ((opcode >> 6) & 3) < 3:
            key, size = (opcode & ~0xC0, row["path"], row["cpu_mode"]), ("byte", "word", "long")[(opcode >> 6) & 3]
        else:
            continue
        by_size[key].setdefault(size, row)
    size_pairs = []
    for key, sizes in by_size.items():
        for small, large in (("byte", "word"), ("word", "long"), ("byte", "long")):
            if small not in sizes or large not in sizes:
                continue
            first, second = sizes[small], sizes[large]
            size_pairs.append({
                "signature": f"{key[0]:04X}", "path": key[1], "cpu_mode": key[2],
                "small_size": small, "large_size": large, "small_case": first["case_id"],
                "large_case": second["case_id"], "small_ns_per_target": first["selected_median_ns_per_target"],
                "large_ns_per_target": second["selected_median_ns_per_target"],
                "ratio": second["selected_median_ns_per_target"] / first["selected_median_ns_per_target"],
                "delta_cpu_clocks_per_target": second["clocks_per_target_composite"] - first["clocks_per_target_composite"],
            })
    size_fields = ["signature", "path", "cpu_mode", "small_size", "large_size", "small_case", "large_case", "small_ns_per_target", "large_ns_per_target", "ratio", "delta_cpu_clocks_per_target"]
    write_csv(run_dir / "size_comparisons.csv", size_pairs, size_fields)
    by_family = defaultdict(list)
    for row in case_rows:
        if row["accepted_for_comparison"] and row["measurement_source"] == run_dir.name:
            by_family[row["workload_family"]].append(row)
    family_rows = [{
        "family": family,
        "case_count": len(rows),
        "median_composite_ns_per_target": statistics.median(row["selected_median_ns_per_target"] for row in rows),
        "median_composite_cpu_clocks_per_target": statistics.median(row["clocks_per_target_composite"] for row in rows),
        "median_host_ns_per_cpu_clock": statistics.median(row["host_ns_per_cpu_clock_composite"] for row in rows),
    } for family, rows in sorted(by_family.items())]
    write_csv(run_dir / "workload_families.csv", family_rows, list(family_rows[0]))
    excluded_paths = [
        {"path": "decoder_exception", "status": "excluded", "reason": "No implemented legal primary instruction is decoded."},
        {"path": "unselected_legal_opcode", "status": "represented", "reason": "The selected encoding shares the classified static instruction form and representative register or operand policy."},
        {"path": "odd_address_error", "status": "deferred", "reason": "Shared alignment and bus-fault behavior needs its own exception-path fixture audit; this run does not claim every instruction fault site."},
        {"path": "trace_exception", "status": "deferred", "reason": "Trace is a CPU mode event after instruction retirement and is covered by separate CPU correctness verification."},
        {"path": "asynchronous_interrupt", "status": "deferred", "reason": "Interrupt acceptance depends on external timing and belongs to a machine or dedicated CPU interrupt workload."},
        {"path": "read_only_a7_or_register_alias", "status": "representative_policy", "reason": "The agreed register sampling does not multiply every source register, A7 read, or source/destination alias. Distinct timing paths remain subject to focused audit."},
    ]
    (run_dir / "path_scope.json").write_text(json.dumps(excluded_paths, indent=2) + "\n", encoding="utf-8")
    comparison_rows, historical = compare_baseline(case_rows, env)
    write_csv(run_dir / "baseline_comparison.csv", comparison_rows, list(comparison_rows[0]))
    write_csv(run_dir / "anomalies.csv",
              [row for row in comparison_rows if row["status"] in ("regression", "warning", "invalid")],
              list(comparison_rows[0]))
    (run_dir / "historical_comparison.json").write_text(json.dumps(historical, indent=2) + "\n", encoding="utf-8")
    status_counts = Counter(row["status"] for row in case_rows)
    valid_control_values = [row["median_ns_per_target"] for row in control_rows if row["status"] == "measured" and row["source"] == run_dir.name]
    control_drift_pct = ((max(valid_control_values) - min(valid_control_values)) / statistics.median(valid_control_values) * 100) if len(valid_control_values) > 1 else None
    control_drift_text = f"{control_drift_pct:.2f}%" if control_drift_pct is not None else "unavailable (one control)"
    summary = {
        "version": 1, "environment": env, "source_snapshot": snapshot,
        "analysis_script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "retry_environment": retry_env, "retry_source_snapshot": retry_snapshot,
        "original_status_counts": original_status_counts, "retry_count": len(retry_rows),
        "raw_full_sha256": hashlib.sha256((run_dir / "measurements.jsonl").read_bytes()).hexdigest(),
        "raw_retry_sha256": hashlib.sha256((retry_dir / "measurements.jsonl").read_bytes()).hexdigest() if retry_dir else None,
        "legal_opcodes": legal_opcode_count, "candidate_count": len(case_rows), "status_counts": status_counts,
        "cpu_mode_counts": Counter(row["cpu_mode"] for row in manifest),
        "path_counts": Counter(row["path"] for row in manifest),
        "a7_selected_case_count": sum("1" in row["a7_choices"] for row in manifest),
        "opcode_coverage_counts": coverage_counts,
        "valid_individual_cases": sum(row["accepted_for_comparison"] for row in case_rows),
        "same_session_comparison_cases": sum(row["accepted_for_comparison"] and row["measurement_source"] == run_dir.name for row in case_rows),
        "control_count": len(control_rows), "invalid_controls": [row["segment"] for row in control_rows if row["status"] != "measured"],
        "control_drift_pct": control_drift_pct,
        "address_mode_pair_count": len(mode_pairs), "size_pair_count": len(size_pairs),
        "workload_family_count": len(family_rows),
        "baseline_comparison": historical,
        "pending_retry_case_count": len(retry_plan["selected"]) if pending_retry else 0,
        "runtime_seconds_approx": ((run_dir / "measurements.jsonl").stat().st_mtime - env["run_id"])
            + (((retry_dir / "measurements.jsonl").stat().st_mtime - retry_env["run_id"]) if retry_dir else 0),
    }
    (run_dir / "summary.json").write_text(json.dumps(summary, indent=2, default=dict) + "\n", encoding="utf-8")
    lines = [
        "# Decoder-derived M68000 benchmark run", "",
        f"- Run: `{run_dir.name}`; Git HEAD: `{env['git_head']}`; binary SHA-256: `{env['binary_sha256']}`.",
        f"- Legal decoder opcodes: {legal_opcode_count:,}; candidate programs: {len(case_rows):,}; validated blocks: {len(probes):,}.",
        f"- Primary opcode disposition: {dict(coverage_counts)}. Legal opcodes not selected use representative register or operand values; they were not individually executed.",
        f"- CPU modes: {dict(summary['cpu_mode_counts'])}; selected A7 cases: {summary['a7_selected_case_count']}; divide-by-zero cases: {summary['path_counts'].get('divide_zero', 0)}; privilege-violation cases: {summary['path_counts'].get('privilege_violation', 0)}.",
        f"- Full-run statuses: {dict(original_status_counts)}; targeted retries: {len(retry_rows)}; final statuses: {dict(status_counts)}; valid individual times with a stable NOP control: {summary['valid_individual_cases']:,}.",
        f"- Pending confirmation cases: {summary['pending_retry_case_count']:,}." if pending_retry else "- Retry screening is complete.",
        f"- NOP controls: {len(control_rows)} ({len(controls)} full-run, {len(retry_controls)} retry); invalid controls: {summary['invalid_controls']}; full-run control drift: {control_drift_text}.",
        f"- Approximate wall time: {summary['runtime_seconds_approx'] / 60:.1f} minutes; affinity: logical CPU {env['core_id']}; priority class: {env['priority_class']}; power plan: {env['active_power_plan']}.",
        f"- Same-session addressing-mode comparisons: {len(mode_pairs)}; size comparisons: {len(size_pairs)}. Retried cases are excluded from cross-case deltas.",
        f"- Complex and system workload categories: {len(family_rows)}; see `workload_families.csv`. These are descriptive medians, not isolated opcode costs.",
        "", "Time is a composite workload cost per validated target. It includes executed guest helpers and host stepping loop overhead. CPU clocks per target use the same block, including helpers. No helper cost has been subtracted. CPU clocks are not CCK; the old catalog's estimated CCK labels were not used as measured clock counts.",
        "", "The retry session used the same candidate manifest, validated fixtures, and measurement algorithm, but its added selector changed the benchmark binary. Its own NOP control and binary/source hashes are retained; raw times from different sessions are not silently normalized.",
        "", f"Baseline comparison: {historical['comparable_case_count']:,}/{historical['case_count']:,} matching cases; verdicts: {historical['status_counts']}. See `baseline_comparison.csv` and `anomalies.csv`. These are diagnostic signals, not correctness failures.",
        "", "Excluded and deferred paths are listed with reasons in `path_scope.json`. Address errors, trace, asynchronous interrupts, and all register alias values are outside this measured set.",
        "", "See `cases.csv`, `controls.csv`, `address_mode_comparisons.csv`, `size_comparisons.csv`, `workload_families.csv`, `measurements.jsonl`, `environment.json`, and `source_snapshot.json` for raw and derived evidence.",
    ]
    (run_dir / "report.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    if summary["valid_individual_cases"] == len(case_rows) and not pending_retry:
        current = {
            "version": 1,
            "full_run": run_dir.name,
            "retry_run": retry_dir.name if retry_dir else None,
            "report": (run_dir / "report.md").relative_to(inventory).as_posix(),
            "summary_sha256": hashlib.sha256((run_dir / "summary.json").read_bytes()).hexdigest(),
        }
        (inventory / "latest_candidate_run.json").write_text(json.dumps(current, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({key: summary[key] for key in ("candidate_count", "original_status_counts", "retry_count", "status_counts", "valid_individual_cases", "same_session_comparison_cases", "control_count", "invalid_controls", "address_mode_pair_count", "size_pair_count", "runtime_seconds_approx")}, indent=2, default=dict))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run_dir", type=Path, nargs="?")
    parser.add_argument("--retry-dir", type=Path)
    parser.add_argument("--prepare-retry-latest", action="store_true")
    args = parser.parse_args()
    if args.prepare_retry_latest:
        if args.run_dir or args.retry_dir:
            parser.error("--prepare-retry-latest takes no run directory")
        inventory = Path(__file__).resolve().parents[2] / ".agent" / "tmp" / "benchmarks" / "inventory"
        runs = sorted(path for path in inventory.glob("full_*") if path.is_dir())
        if not runs:
            parser.exit(1, "Retry planning failed: no full measurement run exists\n")
        try:
            plan = prepare_retry_plan(runs[-1])
        except (OSError, ValueError) as error:
            parser.exit(1, f"Retry planning failed: {error}\n")
        print(json.dumps({"source_run": plan["source_run"],
                          "selection_policy": plan["selection_policy"],
                          "comparable_case_count": plan["comparable_case_count"],
                          "selected_count": len(plan["selected"]),
                          "selection_reasons": dict(Counter(row["reason"] for row in plan["selected"]))},
                         indent=2))
    else:
        if args.run_dir is None:
            parser.error("run_dir is required for analysis")
        analyze(args.run_dir.resolve(), args.retry_dir.resolve() if args.retry_dir else None)

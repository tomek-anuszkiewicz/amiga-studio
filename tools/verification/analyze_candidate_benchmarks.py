"""Validate and summarize one decoder-derived instruction benchmark run."""

import argparse
import csv
import hashlib
import json
import statistics
from collections import Counter, defaultdict
from pathlib import Path


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


def validate_measurement_series(row, block_targets):
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
            ensure(all(300_000_000 <= sample["elapsed_ns"] <= 500_000_000 for sample in selected), f"{row['case_id']}: accepted a short or long series")
    else:
        ensure(row["selected_median_ns_per_target"] is None, f"{row['case_id']}: invalid sample accepted")
    return selected


def analyze(run_dir, retry_dir=None):
    inventory = run_dir.parent
    env = json.loads((run_dir / "environment.json").read_text(encoding="utf-8"))
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
    controls = [entry for entry in raw if entry.get("role") == "control"]
    ensure(len(candidates) == len(manifest), f"incomplete run: {len(candidates)}/{len(manifest)} candidates")
    ensure(len(controls) == (len(manifest) + 99) // 100, "missing NOP controls")
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
        retry_raw = [json.loads(line) for line in (retry_dir / "measurements.jsonl").read_text(encoding="utf-8").splitlines() if line.strip()]
        retry_controls = [row for row in retry_raw if row.get("role") == "control"]
        retry_candidates = [row for row in retry_raw if row.get("role") == "candidate"]
        ensure(len(retry_controls) == (len(retry_candidates) + 99) // 100, "retry NOP control count invalid")
        ensure(all(row["status"] == "measured" for row in retry_controls), "retry NOP control invalid")
        retry_rows = {row["case_id"]: row for row in retry_candidates}
        invalid_ids = {row["case_id"] for index, row in enumerate(candidates)
                       if row["status"] != "measured" or controls[index // 100]["status"] != "measured"}
        ensure(len(retry_rows) == len(retry_candidates) == len(invalid_ids), "retry rows are missing or duplicated")
        ensure(set(retry_rows) == invalid_ids, "retry set differs from invalid full-run cases")
        ensure(all(row["status"] == "measured" for row in retry_candidates), "retry still has invalid measurements")
        retry_control_by_id = {row["case_id"]: retry_controls[index // 100]
                               for index, row in enumerate(retry_candidates)}
    probe_by_id = {row["case_id"]: row for row in probes}
    case_by_id = {row["case_id"]: row for row in manifest}
    case_rows = []
    control_rows = []
    for segment, control in enumerate(controls):
        selected = selected_series(control)
        control_rows.append({
            "segment": segment,
            "source": run_dir.name,
            "before_candidate": segment * 100,
            "status": control["status"],
            "median_ns_per_target": control["selected_median_ns_per_target"],
            "dispersion_pct": control["dispersion_pct"],
            "series_ms": "|".join(f"{sample['elapsed_ns'] / 1e6:.3f}" for sample in selected),
        })
    for segment, control in enumerate(retry_controls):
        selected = selected_series(control)
        control_rows.append({
            "segment": f"retry-{segment}", "source": retry_dir.name,
            "before_candidate": segment * 100,
            "status": control["status"],
            "median_ns_per_target": control["selected_median_ns_per_target"],
            "dispersion_pct": control["dispersion_pct"],
            "series_ms": "|".join(f"{sample['elapsed_ns'] / 1e6:.3f}" for sample in selected),
        })
    for index, (raw_case, case) in enumerate(zip(candidates, manifest)):
        original_status = raw_case["status"]
        source_name = run_dir.name
        control = controls[index // 100]
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
        selected = validate_measurement_series(raw_case, probe["block_targets"])
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
    historical = {
        "version": 1,
        "status": "incompatible",
        "reason": "Retained 108-case baseline lacks executed fixture fingerprints and source provenance; old timing includes injection/reset in its timed loop.",
        "current_case_count": len(case_rows),
        "historical_case_count": 108,
        "comparable_case_count": 0,
    }
    (run_dir / "historical_comparison.json").write_text(json.dumps(historical, indent=2) + "\n", encoding="utf-8")
    status_counts = Counter(row["status"] for row in case_rows)
    valid_control_values = [row["median_ns_per_target"] for row in control_rows if row["status"] == "measured" and row["source"] == run_dir.name]
    control_drift_pct = ((max(valid_control_values) - min(valid_control_values)) / statistics.median(valid_control_values) * 100) if valid_control_values else None
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
        f"- NOP controls: {len(control_rows)} ({len(controls)} full-run, {len(retry_controls)} retry); invalid segments: {summary['invalid_controls']}; full-run control drift: {control_drift_pct:.2f}%.",
        f"- Approximate wall time: {summary['runtime_seconds_approx'] / 60:.1f} minutes; affinity: logical CPU {env['core_id']}; priority class: {env['priority_class']}; power plan: {env['active_power_plan']}.",
        f"- Same-session addressing-mode comparisons: {len(mode_pairs)}; size comparisons: {len(size_pairs)}. Retried cases are excluded from cross-case deltas.",
        f"- Complex and system workload categories: {len(family_rows)}; see `workload_families.csv`. These are descriptive medians, not isolated opcode costs.",
        "", "Time is a composite workload cost per validated target. It includes executed guest helpers and host stepping loop overhead. CPU clocks per target use the same block, including helpers. No helper cost has been subtracted. CPU clocks are not CCK; the old catalog's estimated CCK labels were not used as measured clock counts.",
        "", "The retry session used the same candidate manifest, validated fixtures, and measurement algorithm, but its added selector changed the benchmark binary. Its own NOP control and binary/source hashes are retained; raw times from different sessions are not silently normalized.",
        "", "The retained 108-case report has no matching fixture fingerprints or source provenance, so its times are not comparable with this run.",
        "", "Excluded and deferred paths are listed with reasons in `path_scope.json`. Address errors, trace, asynchronous interrupts, and all register alias values are outside this measured set.",
        "", "See `cases.csv`, `controls.csv`, `address_mode_comparisons.csv`, `size_comparisons.csv`, `workload_families.csv`, `measurements.jsonl`, `environment.json`, and `source_snapshot.json` for raw and derived evidence.",
    ]
    (run_dir / "report.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    if summary["valid_individual_cases"] == len(case_rows):
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
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("--retry-dir", type=Path)
    args = parser.parse_args()
    analyze(args.run_dir.resolve(), args.retry_dir.resolve() if args.retry_dir else None)

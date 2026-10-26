"""Compare M68000 whole-program measurements with approved and related cases."""

import hashlib
import json
import re
from collections import Counter
from statistics import median


SNAPSHOT_FIELDS = ("case_id", "ns_per_target", "target_count", "cpu_clocks",
                   "fingerprint", "disturbed")
RELATED_FAMILIES = (("add", "sub"), ("addi", "subi"),
                    ("addq", "subq"), ("adda", "suba"),
                    ("and", "or"), ("andi", "ori"), ("andi", "eori"),
                    ("ori", "eori"), ("neg", "not"))
DIAGNOSTIC_LIMIT_PERCENT = 5.0
SERIES_SPREAD_LIMIT_PERCENT = 10.0
ADDRESSING_FORMS = frozenset((
    "absolute_long", "absolute_short", "displacement", "indexed", "indirect",
    "postincrement", "predecrement", "immediate", "pc_displacement", "pc_indexed",
    "destination_absolute_long", "destination_absolute_short",
    "destination_displacement", "destination_indexed", "destination_indirect",
    "destination_predecrement",
))


def csv_bytes(rows):
    lines = [",".join(SNAPSHOT_FIELDS)]
    lines.extend(",".join(str(row[field]) for field in SNAPSHOT_FIELDS) for row in rows)
    return ("\n".join(lines) + "\n").encode("utf-8")


def load_snapshot(path):
    snapshot = json.loads(path.read_text(encoding="utf-8"))
    if snapshot.get("schema_version") != 3 or snapshot.get("kind") != "handwritten_m68000":
        raise ValueError(f"incompatible benchmark snapshot: {path}")
    rows = snapshot["rows"]
    if snapshot["case_count"] != len(rows) or len({row["case_id"] for row in rows}) != len(rows):
        raise ValueError(f"invalid benchmark snapshot case count: {path}")
    saved_csv = path.with_suffix(".csv").read_bytes()
    if hashlib.sha256(saved_csv).hexdigest() != snapshot["aggregate_csv_sha256"]:
        raise ValueError(f"benchmark snapshot CSV hash differs from JSON: {path}")
    if saved_csv != csv_bytes(rows):
        raise ValueError(f"benchmark snapshot CSV rows differ from JSON: {path}")
    if not set(snapshot.get("case_sources", {})).issubset({row["case_id"] for row in rows}):
        raise ValueError(f"benchmark snapshot contains unknown case sources: {path}")
    return snapshot


def snapshot_from_run(manifest, rows, aggregate_csv, manifest_bytes):
    selected = [{field: row[field] for field in SNAPSHOT_FIELDS} for row in rows]
    if aggregate_csv != csv_bytes(selected):
        raise ValueError("run aggregate CSV differs from selected rows")
    return {"schema_version": 3, "kind": "handwritten_m68000",
            "source_run": manifest["run_id"],
            "measurement": (f"minimum ns_per_target of {manifest['settings']['series']} "
                            f"{manifest['settings']['duration_ms']} ms whole-program series"),
            "host": manifest["host"], "settings": manifest["settings"],
            "case_count": len(selected),
            "manifest_sha256": hashlib.sha256(manifest_bytes).hexdigest(),
            "aggregate_csv_sha256": hashlib.sha256(aggregate_csv).hexdigest(),
            "rows": selected}


def write_snapshot(path, snapshot, aggregate_csv):
    csv_path = path.with_suffix(".csv")
    csv_temp = csv_path.with_suffix(".csv.tmp")
    json_temp = path.with_suffix(".json.tmp")
    csv_temp.write_bytes(aggregate_csv)
    json_temp.write_text(json.dumps(snapshot, indent=2) + "\n", encoding="utf-8")
    csv_temp.replace(csv_path)
    json_temp.replace(path)


def publish_current(path, snapshot, aggregate_csv):
    if path.is_file():
        previous = load_snapshot(path)
        latest = max([previous["source_run"], *previous.get("case_sources", {}).values()])
        if latest > snapshot["source_run"]:
            return False
    write_snapshot(path, snapshot, aggregate_csv)
    return True


def compare_baseline(baseline, current, selected_cases=None):
    old = {row["case_id"]: row for row in baseline["rows"]}
    new = {row["case_id"]: row for row in current["rows"]}
    host_match = baseline["host"] == current["host"]
    settings_match = baseline["settings"] == current["settings"]
    comparisons = []
    scope = old.keys() if selected_cases is None else selected_cases
    for case_id in sorted(set(scope) | new.keys()):
        before, after = old.get(case_id), new.get(case_id)
        result = {"case_id": case_id,
                  "baseline_ns_per_target": before["ns_per_target"] if before else None,
                  "current_ns_per_target": after["ns_per_target"] if after else None}
        if before is None:
            result["status"] = "new case"
        elif after is None:
            result["status"] = "missing case"
        elif not host_match or not settings_match:
            result["status"] = "incompatible host or settings"
        elif any(before[field] != after[field] for field in
                 ("fingerprint", "target_count", "cpu_clocks")):
            result["status"] = "incompatible workload"
        elif before["disturbed"] or after["disturbed"]:
            result["status"] = "disturbed measurement"
        else:
            result["status"] = "comparable"
            result["change_percent"] = 100 * (after["ns_per_target"] /
                                              before["ns_per_target"] - 1)
            result["flagged"] = result["change_percent"] >= DIAGNOSTIC_LIMIT_PERCENT
        comparisons.append(result)
    return {"baseline": baseline["source_run"], "current": current["source_run"],
            "diagnostic_limit_percent": DIAGNOSTIC_LIMIT_PERCENT,
            "status_counts": dict(Counter(row["status"] for row in comparisons)),
            "cases": comparisons}


def program_shape(cases_root, case_id, target_family=None):
    source = (cases_root / case_id / "program.s").read_text(encoding="utf-8")
    lines = []
    for line in source.splitlines():
        line = line.strip().lower()
        if not line or line.startswith(";"):
            continue
        if target_family:
            line = re.sub(r"^" + target_family + r"(?=\.|\s)", "target", line)
        lines.append(re.sub(r"\b([ad])[0-7]\b", r"\1#", line))
    return "\n".join(lines)


def compare_a7(rows, cases_root):
    groups = {}
    a7_count = sum("a7" in row["case_id"] for row in rows)
    if not a7_count:
        return {"method": "same instruction, program shape, target count and CPU clocks; "
                          "address and data register numbers normalized",
                "a7_case_count": 0, "comparisons": []}
    for row in rows:
        if row["disturbed"]:
            continue
        case_id = row["case_id"]
        family = case_id.split("/", 1)[0]
        key = (family, row["target_count"], row["cpu_clocks"],
               program_shape(cases_root, case_id))
        groups.setdefault(key, []).append(row)
    comparisons = []
    for group in groups.values():
        ordinary = [row for row in group if "a7" not in row["case_id"]]
        if not ordinary:
            continue
        expected = median(row["ns_per_target"] for row in ordinary)
        for row in group:
            if "a7" not in row["case_id"]:
                continue
            difference = 100 * (row["ns_per_target"] / expected - 1)
            comparisons.append({"case_id": row["case_id"],
                                "a7_ns_per_target": row["ns_per_target"],
                                "ordinary_ns_per_target": expected,
                                "ordinary_cases": sorted(peer["case_id"] for peer in ordinary),
                                "difference_percent": difference,
                                "flagged": abs(difference) >= DIAGNOSTIC_LIMIT_PERCENT})
    return {"method": "same instruction, program shape, target count and CPU clocks; "
                      "address and data register numbers normalized",
            "a7_case_count": a7_count,
            "comparisons": sorted(comparisons, key=lambda row: row["case_id"])}


def compare_related(rows, cases_root=None):
    cases = {row["case_id"]: row for row in rows}
    comparisons = []
    skipped = Counter()
    for left_family, right_family in RELATED_FAMILIES:
        left_prefix, right_prefix = left_family + "/", right_family + "/"
        for left_id, left in sorted(cases.items()):
            if not left_id.startswith(left_prefix):
                continue
            suffix = left_id[len(left_prefix):]
            if not suffix.startswith(left_family + "_"):
                continue
            right_id = right_prefix + right_family + suffix[len(left_family):]
            right = cases.get(right_id)
            if right is None:
                skipped["no paired case in this run"] += 1
                continue
            if left["disturbed"] or right["disturbed"]:
                skipped["disturbed measurement"] += 1
                continue
            if (left["target_count"], left["cpu_clocks"]) != (right["target_count"], right["cpu_clocks"]):
                skipped["different target count or CPU clocks"] += 1
                continue
            if cases_root and program_shape(cases_root, left_id, left_family) != \
                    program_shape(cases_root, right_id, right_family):
                skipped["different program shape"] += 1
                continue
            mean_ns = (left["ns_per_target"] + right["ns_per_target"]) / 2
            gap_ns = left["ns_per_target"] - right["ns_per_target"]
            size = suffix[len(left_family) + 1:len(left_family) + 2]
            result = {"left": left_id, "right": right_id, "family_pair":
                      f"{left_family}/{right_family}", "size": size,
                      "left_ns_per_target": left["ns_per_target"],
                      "right_ns_per_target": right["ns_per_target"],
                      "gap_ns_per_target": gap_ns,
                      "difference_percent": 100 * gap_ns / mean_ns}
            result["flagged"] = abs(result["difference_percent"]) >= DIAGNOSTIC_LIMIT_PERCENT
            comparisons.append(result)
    groups = {}
    other_pairs = []
    for row in comparisons:
        family = row["family_pair"].split("/")[0]
        prefix = f"{family}_{row['size']}_"
        case_name = row["left"].split("/")[1]
        form = case_name[len(prefix):] if case_name.startswith(prefix) else ""
        if form not in ADDRESSING_FORMS:
            other_pairs.append(row)
            continue
        direction = "destination" if (form.startswith("destination_") or
                                       family in ("addi", "addq")) else "source"
        mode = form.removeprefix("destination_")
        groups.setdefault((row["family_pair"], row["size"], direction), []).append((mode, row))
    mode_profiles = []
    for (pair, size, direction), entries in sorted(groups.items()):
        if len(entries) < 3:
            other_pairs.extend(row for _, row in entries)
            continue
        left_mean = sum(row["left_ns_per_target"] for _, row in entries) / len(entries)
        right_mean = sum(row["right_ns_per_target"] for _, row in entries) / len(entries)
        typical_gap = median(row["gap_ns_per_target"] for _, row in entries)
        forms = []
        for mode, row in sorted(entries):
            left_ns, right_ns = row["left_ns_per_target"], row["right_ns_per_target"]
            mode_effect = ((left_ns - left_mean) + (right_ns - right_mean)) / 2
            pair_mean = (left_ns + right_ns) / 2
            gap_deviation_pct = 100 * ((left_ns - right_ns) - typical_gap) / pair_mean
            forms.append({"mode": mode, "left": row["left"], "right": row["right"],
                          "left_ns_per_target": left_ns, "right_ns_per_target": right_ns,
                          "difference_percent": row["difference_percent"],
                          "average_mode_effect_ns": mode_effect,
                          "left_adjusted_ns": left_ns - mode_effect,
                          "right_adjusted_ns": right_ns - mode_effect,
                          "gap_deviation_percent": gap_deviation_pct,
                          "flagged": abs(gap_deviation_pct) >= DIAGNOSTIC_LIMIT_PERCENT})
            row["flagged"] = forms[-1]["flagged"]
            row["gap_deviation_percent"] = gap_deviation_pct
            row["typical_gap_ns_per_target"] = typical_gap
            row["form_count"] = len(entries)
        left_typical = median(form["left_adjusted_ns"] for form in forms)
        right_typical = median(form["right_adjusted_ns"] for form in forms)
        for form in forms:
            form["left_adjusted_deviation_percent"] = 100 * (
                form["left_adjusted_ns"] / left_typical - 1)
            form["right_adjusted_deviation_percent"] = 100 * (
                form["right_adjusted_ns"] / right_typical - 1)
        mode_profiles.append({"family_pair": pair, "size": size, "direction": direction,
                              "form_count": len(forms),
                              "left_mean_ns_per_target": left_mean,
                              "right_mean_ns_per_target": right_mean,
                              "left_typical_adjusted_ns": left_typical,
                              "right_typical_adjusted_ns": right_typical,
                              "typical_gap_ns_per_target": typical_gap,
                              "forms": forms})
    return {"diagnostic_limit_percent": DIAGNOSTIC_LIMIT_PERCENT,
            "method": "paired instruction families with matching case suffix, target count, CPU clocks and program shape; canonical mode gaps compared with the group median",
            "skipped": dict(skipped), "peer_comparisons": comparisons,
            "mode_profiles": mode_profiles, "other_pairs": other_pairs}


def report_lines(baseline_comparison, related, a7=None, anomalies=None, file_prefix=""):
    a7 = a7 or {"a7_case_count": 0, "comparisons": []}
    anomalies = anomalies or {"series_spread": [], "failures": [],
                            "load_disturbances": []}
    spreads = {row["case_id"]: row["range_pct"] for row in anomalies["series_spread"]}
    regressions = sorted((row for row in baseline_comparison["cases"] if row.get("flagged")),
                         key=lambda row: row["change_percent"], reverse=True)
    not_comparable = [row for row in baseline_comparison["cases"]
                      if row["status"] != "comparable"]
    peer_outliers = sorted((row for row in related["peer_comparisons"]
                           if row["flagged"] and
                           max(spreads.get(row["left"], 0),
                               spreads.get(row["right"], 0)) <
                           abs(row.get("gap_deviation_percent", row["difference_percent"]))),
                          key=lambda row: abs(row.get("gap_deviation_percent",
                                                      row["difference_percent"])),
                          reverse=True)
    a7_outliers = sorted((row for row in a7["comparisons"]
                          if row["flagged"] and
                          max([spreads.get(row["case_id"], 0)] +
                              [spreads.get(peer, 0) for peer in row["ordinary_cases"]]) <
                          abs(row["difference_percent"])),
                         key=lambda row: abs(row["difference_percent"]), reverse=True)
    unstable = sorted((row for row in anomalies["series_spread"]
                       if row["range_pct"] >= SERIES_SPREAD_LIMIT_PERCENT),
                      key=lambda row: row["range_pct"], reverse=True)
    issue_count = len(unstable) + len(anomalies["failures"]) + len(anomalies["load_disturbances"])
    comparable = baseline_comparison["status_counts"].get("comparable", 0)
    lines = ["## Findings", "",
             f"**{len(regressions)} baseline regressions, "
             f"{len(not_comparable)} baseline coverage issues, {len(peer_outliers)} "
             f"instruction/mode outliers, {len(a7_outliers)} A7 outliers, "
             f"{issue_count} measurement issues.**", "",
             f"Baseline: `{baseline_comparison['baseline']}`; "
             f"current: `{baseline_comparison['current']}`. "
             f"{comparable} cases are comparable.", ""]

    if regressions:
        lines.extend(["### Slower than the approved baseline", "",
                      "| Case | Baseline ns/target | Current ns/target | Change |",
                      "| --- | ---: | ---: | ---: |"])
        lines.extend(f"| `{row['case_id']}` | {row['baseline_ns_per_target']:.2f} | "
                     f"{row['current_ns_per_target']:.2f} | "
                     f"+{row['change_percent']:.2f}% |" for row in regressions)
        lines.append("")

    if not_comparable:
        lines.extend(["### Baseline cases that could not be compared", "",
                      "| Case | Reason |", "| --- | --- |"])
        lines.extend(f"| `{row['case_id']}` | {row['status']} |"
                     for row in not_comparable)
        lines.append("")

    if peer_outliers:
        lines.extend(["### Instruction and addressing-mode outliers", "",
                      "Times cover complete measured programs. A mode signal means the "
                      "instruction gap differs from the usual gap across matching modes. "
                      "Negative deviation means the first workload is faster than expected; "
                      "positive means slower.", "",
                      "| Case | Compared with | Times ns/target | Deviation | Evidence |",
                      "| --- | --- | ---: | ---: | --- |"])
        for row in peer_outliers:
            deviation = row.get("gap_deviation_percent", row["difference_percent"])
            if "gap_deviation_percent" in row:
                evidence = (f"gap vs median of {row['form_count']} modes; "
                            f"usual gap {row['typical_gap_ns_per_target']:+.2f} ns")
            else:
                evidence = "direct match; inspect program details before attributing the gap"
            noise = max(spreads.get(row["left"], 0), spreads.get(row["right"], 0))
            if noise >= SERIES_SPREAD_LIMIT_PERCENT:
                evidence += f"; series spread up to {noise:.1f}%"
            lines.append(f"| `{row['left']}` | `{row['right']}` | "
                         f"{row['left_ns_per_target']:.2f} / "
                         f"{row['right_ns_per_target']:.2f} | "
                         f"{deviation:+.1f}% | {evidence} |")
        lines.append("")

    if a7_outliers:
        lines.extend(["### A7 compared with A0-A6", "",
                      "The pairs have the same instruction, program shape, target count "
                      "and CPU clocks after normalizing register numbers. Byte stack "
                      "steps can access different addresses.", "",
                      "| A7 case | A0-A6 comparison | A7 ns/target | A0-A6 ns/target | Difference |",
                      "| --- | --- | ---: | ---: | ---: |"])
        for row in a7_outliers:
            peers = ", ".join(f"`{case}`" for case in row["ordinary_cases"])
            lines.append(f"| `{row['case_id']}` | {peers} | "
                         f"{row['a7_ns_per_target']:.2f} | "
                         f"{row['ordinary_ns_per_target']:.2f} | "
                         f"{row['difference_percent']:+.1f}% |")
        lines.append("")

    if anomalies["failures"] or anomalies["load_disturbances"]:
        lines.extend(["### Failed or load-disturbed measurements", ""])
        lines.extend(f"- `{row['case_id']}`: {row['finding']}"
                     for category in ("failures", "load_disturbances")
                     for row in anomalies[category])
        lines.append("")

    if unstable:
        lines.extend([f"<details><summary>{len(unstable)} cases with at least "
                      f"{SERIES_SPREAD_LIMIT_PERCENT:.0f}% series spread</summary>",
                      "", "| Case | Spread |", "| --- | ---: |"])
        lines.extend(f"| `{row['case_id']}` | {row['range_pct']:.1f}% |"
                     for row in unstable)
        lines.extend(["", "</details>", ""])

    if not regressions and not not_comparable and not peer_outliers and not a7_outliers and not issue_count:
        lines.extend(["No timing or measurement outliers were found.", ""])

    lines.extend([f"Within-run timing deviations of at least {DIAGNOSTIC_LIMIT_PERCENT:.0f}% "
                  "are listed when they exceed the observed series spread. "
                  "Baseline regressions use the same percentage limit. "
                  "They are review signals, not CPU correctness results. "
                  "Whole-program times include helper instructions.",
                  f"Details: `{file_prefix}baseline_comparison.json`, "
                  f"`{file_prefix}cross_case_anomalies.json`, "
                  f"`{file_prefix}a7_comparison.json`, and "
                  f"`{file_prefix}anomalies.json`.",
                  f"A7 coverage: {len(a7['comparisons'])}/{a7['a7_case_count']} "
                  "cases found a matching A0-A6 workload."])
    return lines


def publish_reports(directory, source_run, anomalies, baseline_comparison, related, a7,
                    case_sources=None):
    prefix = "m68k_benchmark_"
    case_sources = case_sources or {}
    report = ["# M68000 Benchmark Anomaly Report", "", f"Base full run: `{source_run}`"]
    if case_sources:
        runs = ", ".join(f"`{run}`" for run in sorted(set(case_sources.values())))
        report.append(f"Current snapshot also contains {len(case_sources)} refreshed cases "
                      f"from run(s) {runs}; per-case sources are in the current JSON snapshot.")
    report.extend(["", *report_lines(baseline_comparison, related, a7, anomalies, prefix), ""])
    files = {
        prefix + "anomalies.json": dict(anomalies, source_run=source_run),
        prefix + "baseline_comparison.json": baseline_comparison,
        prefix + "cross_case_anomalies.json": dict(related, source_run=source_run),
        prefix + "a7_comparison.json": dict(a7, source_run=source_run),
    }
    for name, content in files.items():
        path = directory / name
        temporary = path.with_suffix(path.suffix + ".tmp")
        temporary.write_text(json.dumps(content, indent=2) + "\n", encoding="utf-8")
        temporary.replace(path)
    path = directory / (prefix + "anomaly_report.md")
    temporary = path.with_suffix(".md.tmp")
    temporary.write_text("\n".join(report), encoding="utf-8")
    temporary.replace(path)

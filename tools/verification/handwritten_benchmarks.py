"""Measure the handwritten M68000 catalog and analyze stored runs."""

import hashlib
import json
import platform
import subprocess
import sys
import threading
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

import psutil

from repository import REPO_ROOT

CASES = REPO_ROOT / "benchmarks/m68k"
RESULTS = REPO_ROOT / "benchmarks/results"
LOAD_LIMIT_PERCENT = 20.0
SAMPLE_SECONDS = 0.02


def catalog():
    cases = {}
    for path in sorted(CASES.glob("*/*/case.json")):
        case_id = "/".join(path.relative_to(CASES).parts[:2])
        config = json.loads(path.read_text(encoding="utf-8"))
        cases[case_id] = config
    return cases


def select_cases(cases, selector=None):
    if selector is None:
        return list(cases)
    kind, value = selector
    if kind == "case":
        selected = [value] if value in cases else []
    elif kind == "mnemonic":
        selected = [case_id for case_id in cases
                    if case_id.split("/")[1].split("_")[0].upper() == value.upper()]
    elif kind == "opcode":
        if len(value) != 4:
            raise ValueError("opcode must be four hexadecimal digits")
        try:
            int(value, 16)
        except ValueError as error:
            raise ValueError("opcode must be four hexadecimal digits") from error
        selected = [case_id for case_id, config in cases.items()
                    if config["target_primary_opcode"].upper() == value.upper()]
    else:
        raise ValueError(f"unknown selector: {kind}")
    if not selected:
        raise ValueError(f"no benchmark cases match {kind} {value}")
    return selected


def executable():
    command = ["cargo", "bench", "-p", "test_runner", "--bench", "bench_instructions",
               "--no-run", "--message-format=json-render-diagnostics"]
    result = subprocess.run(command, cwd=REPO_ROOT, capture_output=True, text=True,
                            encoding="utf-8", errors="replace", check=True)
    binary = None
    for line in result.stdout.splitlines():
        message = json.loads(line)
        target = message.get("target", {})
        if message.get("reason") == "compiler-artifact" and target.get("name") == "bench_instructions":
            binary = message.get("executable")
    if not binary:
        raise ValueError("Cargo did not report the benchmark executable")
    return binary


def cpu_load(process, stop, active, active_series, samples):
    """Sample total busy time on the benchmark core less this process's CPU time."""
    child = psutil.Process(process.pid)
    try:
        previous_cores = psutil.cpu_times(True)
        previous_child = child.cpu_times()
        while not stop.wait(SAMPLE_SECONDS):
            if not active.is_set():
                previous_cores = psutil.cpu_times(True)
                previous_child = child.cpu_times()
                continue
            current_cores = psutil.cpu_times(True)
            current_child = child.cpu_times()
            child_busy = ((current_child.user + current_child.system)
                          - (previous_child.user + previous_child.system))
            for core, (previous_core, current_core) in enumerate(zip(previous_cores, current_cores)):
                total = sum(current_core) - sum(previous_core)
                idle = current_core.idle - previous_core.idle
                if total > 0:
                    samples.append({"series": active_series[0], "core": core, "busy_seconds": total - idle,
                                    "elapsed_seconds": total, "child_busy_seconds": child_busy,
                                    "core_count": len(current_cores)})
            previous_cores, previous_child = current_cores, current_child
    except (psutil.NoSuchProcess, psutil.AccessDenied, IndexError) as error:
        samples.append({"error": str(error)})


def attempt(binary, case_id, duration_ms, series, directory):
    directory.mkdir(parents=True)
    report_path = directory / "report.json"
    command = [binary, "--measure-case", case_id, str(duration_ms), str(series), str(report_path)]
    process = subprocess.Popen(command, cwd=REPO_ROOT, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True, encoding="utf-8", errors="replace")
    stop = threading.Event()
    active = threading.Event()
    active_series = [0]
    samples = []
    monitor = threading.Thread(target=cpu_load, args=(process, stop, active, active_series, samples), daemon=True)
    monitor.start()
    output_lines = []
    for line in process.stdout:
        if line.strip() == "SERIES_BEGIN":
            active_series[0] += 1
            active.set()
        elif line.strip() == "SERIES_END":
            active.clear()
        else:
            output_lines.append(line)
    stderr = process.stderr.read()
    process.wait()
    stop.set()
    monitor.join()
    output = ("".join(output_lines) + stderr).replace(str(REPO_ROOT), "<repo_path>")
    (directory / "execution.txt").write_bytes(output.encode("utf-8"))
    core = None
    if process.returncode == 0 and report_path.is_file():
        core = json.loads(report_path.read_text(encoding="utf-8")).get("affinity_core")
    selected = [sample for sample in samples if "core" in sample and
                (core is None or sample["core"] == core)]
    load_samples = []
    for series_number in range(1, active_series[0] + 1):
        series_samples = [sample for sample in selected if sample["series"] == series_number]
        total = sum(sample["elapsed_seconds"] for sample in series_samples)
        external = sum(sample["busy_seconds"] - sample["child_busy_seconds"] /
                       (sample["core_count"] if core is None else 1) for sample in series_samples)
        load_samples.append({"series": series_number, "observed_core": core,
                             "intervals": len(series_samples),
                             "external_busy_percent": max(0.0, min(100.0, 100.0 * external / total))
                             if total else None})
    observed = max((row["external_busy_percent"] for row in load_samples
                    if row["external_busy_percent"] is not None), default=None)
    disturbed = load_disturbed(load_samples, series)
    load_samples += [sample for sample in samples if "error" in sample]
    return {"report": str(report_path.relative_to(directory.parents[2])).replace("\\", "/"),
            "exit_code": process.returncode, "load_samples": load_samples,
            "observed_core": core, "external_busy_percent": observed, "disturbed": disturbed,
            "error": stderr.strip().replace(str(REPO_ROOT), "<repo_path>") if process.returncode else None}


def load_disturbed(load_samples, series):
    values = [sample["external_busy_percent"] for sample in load_samples if "series" in sample]
    return len(values) != series or any(value is None or value > LOAD_LIMIT_PERCENT for value in values)


def fingerprint(report):
    keys = ("case_id", "case_config", "benchmark_config", "assembler_sha256",
            "source_sha256", "assembly_source_sha256", "image_sha256")
    data = {key: report[key] for key in keys}
    return hashlib.sha256(json.dumps(data, sort_keys=True).encode()).hexdigest()


def write_json(path, data):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_bytes((json.dumps(data, indent=2, ensure_ascii=False) + "\n").encode("utf-8"))
    temporary.replace(path)


def measure(selector=None, duration_ms=None, series=None):
    cases = catalog()
    selected = select_cases(cases, selector)
    shared = json.loads((CASES / "config.json").read_text(encoding="utf-8"))
    duration_ms = shared["target_series_ms"] if duration_ms is None else duration_ms
    series = shared["series"] if series is None else series
    if duration_ms <= 0 or series <= 0:
        raise ValueError("duration-ms and series must be positive")
    binary = executable()
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    run_dir = RESULTS / run_id
    run_dir.mkdir(parents=True, exist_ok=False)
    manifest = {"schema_version": 1, "kind": "handwritten_m68000", "run_id": run_id,
                "status": "incomplete", "selected_cases": selected, "selector": selector,
                "full_catalog": selector is None, "settings": {"duration_ms": duration_ms,
                "series": series, "load_limit_percent": LOAD_LIMIT_PERCENT,
                "load_sample_seconds": SAMPLE_SECONDS},
                "host": {"os": platform.system(), "architecture": platform.machine(),
                         "processor": platform.processor()}, "cases": {}}
    write_json(run_dir / "manifest.json", manifest)
    try:
        for case_id in selected:
            print(f"[RUN] {case_id}", flush=True)
            attempts = []
            for index in range(2):
                directory = run_dir / "attempts" / case_id.replace("/", "__") / str(index + 1)
                record = attempt(binary, case_id, duration_ms, series, directory)
                attempts.append(record)
                if record["exit_code"] or not record["disturbed"]:
                    break
            chosen = attempts[-1]
            manifest["cases"][case_id] = {"attempts": attempts,
                                          "selected_attempt": len(attempts) - 1,
                                          "status": "failed" if chosen["exit_code"] else "measured",
                                          "disturbed": chosen["disturbed"]}
            write_json(run_dir / "manifest.json", manifest)
        manifest["status"] = "complete"
        write_json(run_dir / "manifest.json", manifest)
    finally:
        if manifest["status"] == "complete":
            analyze(run_dir)
    print(f"Run directory: {run_dir}")
    return 1 if any(row["status"] == "failed" for row in manifest["cases"].values()) else 0


def committed(path):
    relative = path.relative_to(REPO_ROOT).as_posix()
    record = subprocess.run(["git", "show", f"HEAD:{relative}"], cwd=REPO_ROOT,
                            capture_output=True)
    return record.returncode == 0 and record.stdout == path.read_bytes()


def previous_runs(run_dir, current):
    matches = []
    for path in sorted(RESULTS.glob("*/manifest.json"), reverse=True):
        if path.parent >= run_dir or not committed(path):
            continue
        manifest = json.loads(path.read_text(encoding="utf-8"))
        if manifest.get("status") == "complete" and manifest.get("full_catalog"):
            if manifest.get("host") == current["host"] and manifest.get("settings") == current["settings"]:
                matches.append((path.parent, manifest))
    return matches


def analyze(run_dir):
    run_dir = Path(run_dir).resolve()
    manifest = json.loads((run_dir / "manifest.json").read_text(encoding="utf-8"))
    if manifest["status"] != "complete":
        raise ValueError("cannot analyze an incomplete run")
    candidates = previous_runs(run_dir, manifest)
    rows, comparisons = [], []
    anomalies = {"series_spread": [], "load_disturbances": [], "failures": []}
    for case_id in manifest["selected_cases"]:
        case = manifest["cases"][case_id]
        if case["status"] == "failed":
            anomalies["failures"].append({"case_id": case_id, "finding": "measured execution failed"})
            comparisons.append({"case_id": case_id, "status": "current measurement failed"})
            continue
        chosen = case["attempts"][case.get("selected_attempt", len(case["attempts"]) - 1)]
        report = json.loads((run_dir / chosen["report"]).read_text(encoding="utf-8"))
        result = report["results"][0]
        disturbed = load_disturbed(chosen["load_samples"], manifest["settings"]["series"])
        row = {"case_id": case_id, "ns_per_target": result["min_ns_per_target"],
               "target_count": result["target_instructions_per_run"],
               "cpu_clocks": result["clocks_per_run"], "fingerprint": fingerprint(report),
               "disturbed": disturbed, "series": result["samples"]}
        rows.append(row)
        if disturbed:
            anomalies["load_disturbances"].append({"case_id": case_id,
                                                    "finding": "persistent external CPU load"})
        anomalies["series_spread"].append({"case_id": case_id,
                                           "range_pct": result["range_pct"]})
        comparison = {"case_id": case_id, "status": "no compatible committed full run"}
        for previous_dir, previous in candidates:
            old_case = previous["cases"].get(case_id)
            if not old_case or old_case["status"] != "measured":
                comparison["status"] = "missing previous case"
                continue
            old_attempt = old_case["attempts"][old_case.get("selected_attempt", len(old_case["attempts"]) - 1)]
            old_path = previous_dir / old_attempt["report"]
            if not committed(old_path):
                comparison["status"] = "previous case report is not committed"
                continue
            old_report = json.loads(old_path.read_text(encoding="utf-8"))
            old_result = old_report["results"][0]
            host_keys = ("host_os", "host_arch", "affinity_core", "priority_class")
            compatible = (fingerprint(old_report) == row["fingerprint"] and
                          old_result["target_instructions_per_run"] == row["target_count"] and
                          old_result["clocks_per_run"] == row["cpu_clocks"] and
                          all(old_report[key] == report[key] for key in host_keys))
            if not compatible:
                comparison["status"] = "incompatible workload or host settings"
                continue
            delta = 100 * (row["ns_per_target"] / old_result["min_ns_per_target"] - 1)
            comparison = {"case_id": case_id, "status": "compatible", "previous_run": previous_dir.name,
                          "previous_ns_per_target": old_result["min_ns_per_target"],
                          "current_ns_per_target": row["ns_per_target"], "change_percent": delta}
            break
        comparisons.append(comparison)
    write_json(run_dir / "aggregate.json", {"rows": rows, "failures": [case_id for case_id, case
               in manifest["cases"].items() if case["status"] == "failed"]})
    write_json(run_dir / "anomalies.json", anomalies)
    write_json(run_dir / "comparison.json", {"cases": comparisons})
    with (run_dir / "aggregate.csv").open("w", newline="", encoding="utf-8") as target:
        fields = ("case_id", "ns_per_target", "target_count", "cpu_clocks", "fingerprint", "disturbed")
        target.write(",".join(fields) + "\n")
        for row in rows:
            target.write(",".join(str(row[field]) for field in fields) + "\n")
    statuses = Counter(row["status"] for row in comparisons)
    spreads = [row["range_pct"] for row in anomalies["series_spread"]]
    lines = ["# M68000 Benchmark Run", "", f"Run: `{manifest['run_id']}`", "",
             f"Measured cases: {len(rows)} of {len(manifest['selected_cases'])}",
             f"Series: {manifest['settings']['series']}; duration: "
             f"{manifest['settings']['duration_ms']} ms of timed execution per series", "",
             "## Current-run anomalies", "",
             f"Failed cases: {len(anomalies['failures'])}",
             f"Persistent host-load disturbances: {len(anomalies['load_disturbances'])}",
             f"Largest observed series spread: {max(spreads):.2f}%" if spreads else
             "Series spread: no successful cases", ""]
    lines.extend(f"- {row['case_id']}: {row['finding']}" for category in
                 ("failures", "load_disturbances") for row in anomalies[category])
    lines.extend(["", "## Previous-run comparison", ""])
    lines.extend(f"- {status}: {count}" for status, count in sorted(statuses.items()))
    lines.extend(["", "Raw per-case timing changes and compatibility reasons are in `comparison.json`.",
                  "Series spread values are in `anomalies.json`; no spread threshold is applied.", ""])
    (run_dir / "report.md").write_bytes("\n".join(lines).encode("utf-8"))
    print(f"Analyzed {len(rows)} measured cases; "
          f"{len(anomalies['load_disturbances']) + len(anomalies['failures'])} anomaly findings; "
          f"compatible comparisons: {sum(row['status'] == 'compatible' for row in comparisons)}")


if __name__ == "__main__":
    try:
        if sys.argv[1:2] == ["analyze"] and len(sys.argv) == 3:
            analyze(sys.argv[2])
        elif sys.argv[1:2] == ["measure"]:
            arguments = sys.argv[2:]
            selector = None
            if arguments[:1] and arguments[0] in ("case", "mnemonic", "opcode"):
                selector, arguments = tuple(arguments[:2]), arguments[2:]
            values = {}
            if len(arguments) % 2:
                raise ValueError("expected duration-ms N and series N")
            for key, value in zip(arguments[::2], arguments[1::2]):
                if key not in ("duration-ms", "series") or key in values:
                    raise ValueError(f"unknown or repeated parameter: {key}")
                values[key] = int(value)
            sys.exit(measure(selector, values.get("duration-ms"), values.get("series")))
        else:
            raise ValueError("use measure [case ID|mnemonic NAME|opcode XXXX] "
                             "[duration-ms N] [series N], or analyze RUN_DIRECTORY")
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"Error: {error}", file=sys.stderr)
        sys.exit(1)

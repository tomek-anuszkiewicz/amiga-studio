"""Scope LLVM coverage and report unexecuted source locations for test review."""

import argparse
import json
from pathlib import Path
import sys
from impact import workspace_packages


def scope_report(document, roots):
    """Drop retired or unselected source files retained in cached LLVM objects."""
    removed = 0
    for entry in document["data"]:
        current = []
        for item in entry["files"]:
            path = Path(item["filename"]).resolve()
            if path.is_file() and any(path.is_relative_to(root) for root in roots):
                current.append(item)
            else:
                removed += 1
        entry["files"] = current
        filenames = {item["filename"] for item in current}
        entry["functions"] = [function for function in entry["functions"]
                              if function["filenames"][function["regions"][0][5]] in filenames]
        for name, previous in entry["totals"].items():
            totals = {key: sum(item["summary"][name][key] for item in current)
                      for key in previous if key != "percent"}
            totals["percent"] = 100 * totals["covered"] / totals["count"] if totals["count"] else 0
            entry["totals"][name] = totals
    return removed


def uncovered_spans(segments):
    """Read zero-count intervals from LLVM's merged per-file coverage segments."""
    spans = []
    # LLVM exports line, column, count, has_count, is_region_entry, is_gap_region.
    for segment, following in zip(segments, segments[1:]):
        start, end = tuple(segment[:2]), tuple(following[:2])
        if segment[3] and segment[2] == 0 and not segment[5] and start < end:
            if spans and spans[-1][1] == start:
                spans[-1] = (spans[-1][0], end)
            else:
                spans.append((start, end))
    return spans


def render_gaps(document, root):
    scope = document["verification_scope"]
    lines = ["# Ordinary Rust coverage gaps", "",
             "Test packages: " + ", ".join(scope["test_packages"]), "",
             "Scope: " + ("all ordinary workspace packages" if scope["full_workspace"] else "selected packages only") + ".",
             "Current build target and instrumented code only; test_runner, doctests and branch coverage are not measured.",
             "Use only with a successful verifier run whose source and index remained unchanged.",
             "Review assertions as well as execution. Missing instrumentation is not proof of coverage.", "",
             "Locations are line:column; source spans have an exclusive end. Function entries combine instantiations.",
             "Function-instance regions may execute in another instance; inspect the scenario before adding a test.", ""]
    entries = {}
    regions = {}
    files = []
    for entry in document["data"]:
        files.extend(entry["files"])
        for function in entry["functions"]:
            region = function["regions"][0]
            key = (function["filenames"][region[5]], region[0], region[1])
            entries[key] = entries.get(key, 0) + function["count"]
            for region in function["regions"]:
                # LLVM region kind 0 is code; expansion, skipped and gap regions differ.
                if region[4] == 0 and region[7] == 0:
                    filename = function["filenames"][region[5]]
                    regions.setdefault(filename, set()).add(tuple(region[:4]))
    gaps = 0
    for item in sorted(files, key=lambda item: item["filename"]):
        missing = {name: item["summary"][name]["count"] - item["summary"][name]["covered"]
                   for name in ("lines", "functions", "regions")}
        spans = uncovered_spans(item["segments"])
        instance_regions = sorted(regions.get(item["filename"], ()))
        functions = sorted((line, column) for (filename, line, column), count in entries.items()
                           if filename == item["filename"] and count == 0)
        if not any(missing.values()) and not spans and not functions and not instance_regions:
            continue
        gaps += 1
        path = Path(item["filename"]).resolve().relative_to(root).as_posix()
        lines += [f"## {path}", "", "Uncovered: " + ", ".join(f"{count} {name}" for name, count in missing.items()) + ".", ""]
        if functions:
            lines += ["Function entries: " + ", ".join(f"{line}:{column}" for line, column in functions), ""]
        if spans:
            lines += ["Zero-count source spans: " + ", ".join(
                f"{start[0]}:{start[1]}-{end[0]}:{end[1]}" for start, end in spans), ""]
        if instance_regions:
            lines += ["Zero-count function-instance regions: " + ", ".join(
                f"{line}:{column}-{end_line}:{end_column}" for line, column, end_line, end_column in instance_regions), ""]
    if not gaps:
        lines += ["No execution gaps reported within the measured scope.", ""]
    return "\n".join(lines)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepare", action="store_true", help="Remove the previous report before a fresh run")
    parser.add_argument("--package", action="append", help="Package whose ordinary tests ran; repeat for each package")
    args = parser.parse_args(argv)
    root = Path(__file__).resolve().parents[2]
    directory = root / ".agent/tmp/coverage"
    detailed = directory / "coverage.json"
    report = directory / "summary.json"
    gaps = directory / "gaps.md"
    if args.prepare:
        directory.mkdir(parents=True, exist_ok=True)
        for artifact in (detailed, report, gaps):
            artifact.unlink(missing_ok=True)
        return 0
    try:
        document = json.loads(detailed.read_text(encoding="utf-8"))
        _, roots = workspace_packages(root)
        names = set(roots.values()) - {"test_runner"}
        if args.package:
            if set(args.package) - names:
                raise ValueError("Coverage selection contains unknown ordinary packages")
            names = set(args.package)
        removed = scope_report(document, [path for path, name in roots.items() if name in names])
        data = document["data"]
        metrics = []
        for name in ("lines", "functions", "regions"):
            count = sum(entry["totals"][name]["count"] for entry in data)
            covered = sum(entry["totals"][name]["covered"] for entry in data)
            if not count:
                raise ValueError(f"No measured {name}")
            metrics.append(f"{name} {100 * covered / count:.2f}% ({covered}/{count})")
    except (OSError, ValueError, KeyError, TypeError, IndexError) as error:
        print(f"[ERROR] Ordinary crate coverage report unavailable: {error}")
        return 1
    scope = {"test_packages": sorted(names), "full_workspace": not args.package,
             "source_files": "current files in selected ordinary packages"}
    document["verification_scope"] = scope
    try:
        gap_text = render_gaps(document, root)
        summary = {**document, "data": [
            {"totals": entry["totals"], "files": [
                {"filename": item["filename"], "summary": item["summary"]} for item in entry["files"]]}
            for entry in data]}
        detailed.write_text(json.dumps(document) + "\n", encoding="utf-8")
        report.write_text(json.dumps(summary) + "\n", encoding="utf-8")
        gaps.write_text(gap_text, encoding="utf-8")
    except (OSError, ValueError, KeyError, TypeError, IndexError) as error:
        print(f"[ERROR] Cannot write coverage reports: {error}")
        return 1
    print(f"[INFO] Coverage test scope: {scope['test_packages']}; current source files only.")
    if removed:
        print(f"[INFO] Excluded {removed} retired or out-of-scope source files from cached coverage objects.")
    print(f"[INFO] Ordinary crate coverage: {'; '.join(metrics)}")
    print("[INFO] Report: .agent/tmp/coverage/summary.json (no percentage threshold)")
    print("[INFO] Coverage review: .agent/tmp/coverage/gaps.md; details: .agent/tmp/coverage/coverage.json")
    return 0


if __name__ == "__main__":
    sys.exit(main())

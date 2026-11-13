"""Prepare and summarize the ordinary-crate LLVM coverage report."""

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
        for name, previous in entry["totals"].items():
            totals = {key: sum(item["summary"][name][key] for item in current)
                      for key in previous if key != "percent"}
            totals["percent"] = 100 * totals["covered"] / totals["count"] if totals["count"] else 0
            entry["totals"][name] = totals
    return removed


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepare", action="store_true", help="Remove the previous report before a fresh run")
    parser.add_argument("--package", action="append", help="Package whose ordinary tests ran; repeat for each package")
    args = parser.parse_args(argv)
    root = Path(__file__).resolve().parents[2]
    report = root / ".agent/tmp/coverage/summary.json"
    if args.prepare:
        report.parent.mkdir(parents=True, exist_ok=True)
        report.unlink(missing_ok=True)
        return 0
    try:
        document = json.loads(report.read_text(encoding="utf-8"))
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
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"[ERROR] Ordinary crate coverage report unavailable: {error}")
        return 1
    scope = {"test_packages": sorted(names), "full_workspace": not args.package,
             "source_files": "current files in selected ordinary packages"}
    document["verification_scope"] = scope
    try:
        report.write_text(json.dumps(document) + "\n", encoding="utf-8")
    except OSError as error:
        print(f"[ERROR] Cannot record coverage scope: {error}")
        return 1
    print(f"[INFO] Coverage test scope: {scope['test_packages']}; current source files only.")
    if removed:
        print(f"[INFO] Excluded {removed} retired or out-of-scope source files from cached coverage objects.")
    print(f"[INFO] Ordinary crate coverage: {'; '.join(metrics)}")
    print("[INFO] Report: .agent/tmp/coverage/summary.json (no percentage threshold)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

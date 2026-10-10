"""Prepare and summarize the ordinary-crate LLVM coverage report."""

import argparse
import json
from pathlib import Path
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prepare", action="store_true", help="Remove the previous report before a fresh run")
    args = parser.parse_args()
    report = Path(__file__).resolve().parents[2] / ".agent/tmp/coverage/summary.json"
    if args.prepare:
        report.parent.mkdir(parents=True, exist_ok=True)
        report.unlink(missing_ok=True)
        return 0
    try:
        data = json.loads(report.read_text(encoding="utf-8"))["data"]
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
    print(f"[INFO] Ordinary crate coverage: {'; '.join(metrics)}")
    print("[INFO] Report: .agent/tmp/coverage/summary.json (no percentage threshold)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

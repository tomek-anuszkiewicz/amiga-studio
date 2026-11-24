"""Run the public function probe for every Cargo workspace package."""

import argparse
import json
from pathlib import Path
import tempfile

from impact import workspace_packages
from unused_public import probe_package
from unused_public_cargo import effective_build


def candidate_location(candidate):
    match candidate:
        case {"name": str(name), "module": str(module), "enclosing": list(enclosing),
              "name_span": {"path": str(path), "line": line}} if (
                  name and module and path and type(line) is int and line > 0
                  and not Path(path).is_absolute() and ".." not in Path(path).parts
                  and all(isinstance(part, str) and part for part in enclosing)):
            return "::".join([module, *enclosing, name]), f"{path}:{line}"
    raise ValueError("invalid candidate function or source location")


def build_failures(contexts):
    for name, context in contexts.items():
        match context:
            case {"baseline": {"state": str(baseline)} as original, "batch": {"state": str()}}:
                phase, build = ("baseline", original) if baseline != "succeeded" else effective_build(context)
                state = build["state"]
                expected = state in {"succeeded", "skipped"} or (state == "failed" and
                                                                 build.get("failure_kind") == "visibility")
                if phase != "baseline" and expected:
                    continue
                reason = build.get("reason", f"Cargo exit code {build['exit_code']}" if "exit_code" in build else "")
                detail = f" ({reason})" if reason else ""
                yield f"{name} {phase} {build['state']}{detail}", build.get("log")
            case _:
                yield f"invalid build result for {name}", None


def check_report(package, path, code):
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
        match report:
            case {"package": str(name), "state": str(state), "candidates": list(candidates),
                  "contexts": dict(contexts)} if name == package:
                locations = [candidate_location(candidate) for candidate in candidates]
            case _:
                raise ValueError("missing or mismatched package, state, candidates or contexts")
    except (OSError, ValueError) as error:
        print(f"[ERROR] {package}: cannot use public probe report (exit code {code}): {error}", flush=True)
        print(f"[INFO] Expected report: {path}", flush=True)
        return False

    for function, location in locations:
        print(f"[ERROR] {package}: candidate unused public function {function} at {location}", flush=True)
        print(f"[INFO] Candidate report: {path}", flush=True)

    failures = list(build_failures(contexts))
    if state != "completed" or code:
        reason = report.get("reason")
        detail = f": {reason}" if reason else ""
        print(f"[ERROR] {package}: public probe {state} (exit code {code}){detail}", flush=True)
        print(f"[INFO] Probe report: {path}", flush=True)
    elif not contexts or not completed_discovery(report):
        failures.append(("report lacks completed capture, discovery or build results", None))
    for failure, log in failures:
        print(f"[ERROR] {package}: {failure}", flush=True)
        print(f"[INFO] Diagnostic log: {log}" if log else f"[INFO] Probe report: {path}", flush=True)
    return state == "completed" and code == 0 and not failures and not locations


def completed_discovery(report):
    match report:
        case {"candidate": {"state": "captured"},
              "discovery": {"state": "completed", "pending_targets": [], "pending_consumer_targets": []}}:
            return True
    return False


def run_workspace(root, results):
    try:
        packages, _ = workspace_packages(root)
        results.mkdir(parents=True, exist_ok=True)
        sweep = Path(tempfile.mkdtemp(prefix="unused-public-workspace-", dir=results)).resolve()
    except (OSError, ValueError) as error:
        print(f"[ERROR] Cannot start workspace public probe: {error}", flush=True)
        return 1

    print(f"[INFO] Workspace public probe reports: {sweep}", flush=True)
    passed = True
    for package in sorted(item["name"] for item in packages):
        try:
            output = Path(tempfile.mkdtemp(prefix=f"{package}-", dir=sweep))
            code = probe_package(root, package, output, excluded_output=sweep)
        except KeyboardInterrupt:
            print(f"[ERROR] {package}: public probe interrupted by the user.", flush=True)
            return 130
        except (OSError, RuntimeError, ValueError) as error:
            print(f"[ERROR] {package}: cannot run public probe: {error}", flush=True)
            passed = False
            continue
        if not check_report(package, output / "report.json", code):
            passed = False
        if code == 130:
            return code
    if not passed:
        print("[INFO] Check callers and re-exports for each candidate and remove confirmed unused code. "
              "Resolve probe failures and rerun milestone verification.", flush=True)
    return int(not passed)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--results-dir", type=Path, default=Path(".agent/tmp/verify"))
    args = parser.parse_args(argv)
    return run_workspace(Path.cwd(), args.results_dir)


if __name__ == "__main__":
    raise SystemExit(main())

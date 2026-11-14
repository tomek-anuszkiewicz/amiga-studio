#!/usr/bin/env python3
"""Repository verification and diagnostic command entry point."""
import subprocess
import sys
import json

from cli import parse_args
from commands import select_phases
from execution import execute_command, print_info, result_label
from repository import REPO_ROOT, collect_changes, snapshot
from evidence import BatchEvidence, capture_inputs, execution_conditions, group_identity


if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def main(argv=None):
    parser, args = parse_args(sys.argv[1:] if argv is None else argv)
    if args is None:
        return 0
    before = None
    batch = None
    completion = args.command in ("commit", "milestone")
    if completion and not args.plan:
        try:
            before = snapshot(REPO_ROOT)
        except (RuntimeError, OSError) as error:
            print(f"{result_label(False)} Cannot capture source snapshot: {error}")
            return 1
    try:
        automatic_python = args.command == "test" and args.kind == "python" and not args.words
        if args.results_dir is not None and not args.results_dir.strip():
            raise ValueError("--results-dir requires a nonempty path")
        directory = args.results_dir or (f".agent/tmp/verification/{args.command}" if completion else None)
        results_dir = (REPO_ROOT / directory).resolve() if directory else None
        changes = collect_changes(REPO_ROOT) if automatic_python or completion else set()
        if completion and not args.plan:
            batch = BatchEvidence(REPO_ROOT, results_dir, args.command)
        explanations = []
        phases = select_phases(args, changes, explanations)
    except (RuntimeError, OSError, ValueError) as error:
        parser.error(str(error))
    for explanation in explanations:
        print(f"[INFO] {explanation}")
    if results_dir is not None:
        phases = [(phase, [(name, command,
                            {**overrides, "SINGLESTEP_RESULTS_DIR": str(results_dir / "singlestep")}
                            if "tools/verification/run_singlestep.py" in command else overrides)
                           for name, command, overrides in commands])
                  for phase, commands in phases]
    if args.command == "commit" and not any(path.startswith("tools/bootstrap/") for path in changes):
        print("[INFO] Bootstrap contract suites skipped: no tools/bootstrap/ changes.")
    if args.plan:
        print("[PLAN] Preview only; no checks or tests will run.")
        if results_dir is not None:
            print(f"[PLAN] Verifier logs: {results_dir / 'verify'}")
        if not any(commands for _, commands in phases):
            print("[INFO] No Python tool contracts selected: their inputs are unchanged."
                  if automatic_python else "[INFO] No checks or tests selected.")
        for phase, commands in phases:
            for name, command, overrides in commands:
                print(f"  {phase}: {name}\n    {subprocess.list2cmdline(command)}")
                if overrides:
                    print(f"    Environment: {overrides}")
        return 0
    if not any(commands for _, commands in phases):
        print("[INFO] No Python tool contracts selected: their inputs are unchanged."
                  if automatic_python else "[INFO] No checks or tests selected.")
        return 0
    produces_reports = args.command == "runner" and args.action in ("bench", "analyze", "refresh")
    if not produces_reports and before is None:
        try:
            before = snapshot(REPO_ROOT)
        except (RuntimeError, OSError) as error:
            print(f"{result_label(False)} Cannot capture source snapshot: {error}")
            return 1
    if batch is not None:
        return execute_batch(args, phases, batch, before)
    results = []
    blocked = False
    for phase, commands in phases:
        if phase != "Preflight" and blocked:
            print("[MISSING] Expensive checks deferred until preliminary blockers pass.")
            break
        if phase == "Coverage":
            print_info("Building/reusing Cargo tests WITH coverage instrumentation.")
        elif any(command[0] == "cargo" for _, command, _ in commands):
            print_info("Building/reusing Cargo artifacts WITHOUT coverage instrumentation.")
        for name, command, overrides in commands:
            print(f"[RUN] {name}", flush=True)
            ok, detail, elapsed = execute_command(command, overrides, log_level=args.log_level,
                                                  log_dir=results_dir / "verify" if results_dir else None)
            results.append(ok)
            blocked = blocked or phase in ("Preflight", "Build checks") and not ok
            print(f"{result_label(ok)} {name} ({elapsed:.2f}s): {detail}", flush=True)
            if phase == "Coverage" and not ok:
                break
            if args.command == "runner" and args.action in ("bench", "profile") and not ok:
                break
    try:
        unchanged = produces_reports or before == snapshot(REPO_ROOT)
    except (RuntimeError, OSError):
        unchanged = False
    if not unchanged:
        print(f"{result_label(False)} Source or index changed during verification. Verify the current snapshot again.")
        results.append(False)
    print(f"{result_label(all(results))} {args.command} verification")
    return 0 if all(results) else 1



def execute_batch(args, phases, batch, before):
    """Execute the full selected profile, retaining only matching successful groups."""
    batch.directory.mkdir(parents=True, exist_ok=True)
    lock = batch.directory / "active.lock"
    try:
        with lock.open("x", encoding="utf-8") as handle:
            handle.write(batch.attempt)
    except FileExistsError:
        print(f"{result_label(False)} Batch is active or interrupted; use a fresh evidence directory if its owner stopped.")
        return 1
    try:
        batch.load()
        inputs = capture_inputs(REPO_ROOT)
        input_state = capture_inputs(REPO_ROOT, contents=False)
        conditions = execution_conditions(REPO_ROOT)
        source = snapshot(REPO_ROOT)
        if source != before:
            raise ValueError("Source or index changed during selection")
        groups = [(phase, "Coverage" if phase == "Coverage" else items[0][0], items)
                  for phase, commands in phases
                  for items in ([commands] if phase == "Coverage" else [[item] for item in commands]) if items]
        batch.record["required"] = [name for _, name, _ in groups]
        batch.record["attempt"] = batch.attempt
        batch.record["outcome"] = "running"
        batch.record["results"] = {}
        manifest = batch.directory / "verify" / batch.attempt / "inputs.json"
        manifest.parent.mkdir(parents=True, exist_ok=True)
        manifest.write_text(json.dumps(inputs, indent=2) + "\n", encoding="utf-8")
        batch.record["inputs"] = manifest.relative_to(batch.directory).as_posix()
        batch.record["conditions"] = conditions
        batch.save()
        results = batch.record["results"]
        blocked = False
        for phase, name, commands in groups:
            identity = group_identity(name, commands, inputs, conditions)
            reason = batch.reason(name, identity)
            if phase != "Preflight" and blocked:
                results[name] = {"status": "missing", "reason": "preliminary blocker"}
                print(f"[MISSING] {name}: preliminary blocker")
                continue
            if snapshot(REPO_ROOT) != source:
                raise ValueError("Source or index changed during verification")
            current_state = capture_inputs(REPO_ROOT, contents=False)
            if current_state != input_state and capture_inputs(REPO_ROOT) != inputs:
                raise ValueError("Source or index changed during verification")
            input_state = current_state
            if reason is None:
                item = batch.record["groups"][name]
                results[name] = {"status": "reused", "attempt": item["attempt"]}
                print(f"[REUSED] {name}: matching inputs and artifacts; attempt {item['attempt']}")
                continue
            old = batch.record["groups"].get(name)
            label = "INVALIDATED" if old and old.get("status") == "passed" else "MISSING"
            print(f"[{label}] {name}: {reason}")
            directory = batch.start(name, identity, commands, source)
            success, elapsed = True, 0
            for title, command, overrides in commands:
                print(f"[RUN] {title}", flush=True)
                ok, detail, seconds = execute_command(command, overrides, log_level=args.log_level, log_dir=directory)
                elapsed += seconds
                success = success and ok
                print(f"{result_label(ok)} {title} ({seconds:.2f}s): {detail}", flush=True)
                if phase == "Coverage" and not ok:
                    break
            if snapshot(REPO_ROOT) != source:
                raise ValueError("Source or index changed during verification")
            current_state = capture_inputs(REPO_ROOT, contents=False)
            if current_state != input_state and capture_inputs(REPO_ROOT) != inputs:
                raise ValueError("Source or index changed during verification")
            input_state = current_state
            success = batch.finish(name, directory, success, elapsed)
            results[name] = {"status": "executed" if success else "failed", "reason": reason,
                             "attempt": batch.attempt}
            blocked = blocked or phase in ("Preflight", "Build checks") and not success
        if snapshot(REPO_ROOT) != source or capture_inputs(REPO_ROOT) != inputs or execution_conditions(REPO_ROOT) != conditions:
            raise ValueError("Source, index or execution conditions changed during verification")
        success = all(item["status"] in ("executed", "reused") for item in results.values())
        batch.record["outcome"] = "passed" if success else "failed"
        batch.record["source"] = source
        batch.save()
        print(f"{result_label(success)} {args.command} verification; evidence: {batch.path}")
        return 0 if success else 1
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        for item in batch.record["groups"].values():
            if item.get("attempt") == batch.attempt:
                item["status"] = "invalidated"
        batch.record["outcome"] = "failed"
        batch.save()
        print(f"{result_label(False)} {error}")
        return 1
    finally:
        lock.unlink()


if __name__ == "__main__":
    sys.exit(main())

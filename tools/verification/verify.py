#!/usr/bin/env python3
"""Repository verification and diagnostic command entry point."""
import subprocess
import sys

from cli import parse_args
from commands import select_phases
from execution import execute_command, print_info, result_label
from repository import REPO_ROOT, collect_changes, snapshot


if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")


def main(argv=None):
    parser, args = parse_args(sys.argv[1:] if argv is None else argv)
    if args is None:
        return 0
    try:
        changes = collect_changes(REPO_ROOT) if args.command in ("commit", "milestone") else set()
        phases = select_phases(args, changes)
    except (RuntimeError, OSError, ValueError) as error:
        parser.error(str(error))
    if args.plan:
        print("[PLAN] Preview only; no checks or tests will run.")
        for phase, commands in phases:
            for name, command, overrides in commands:
                print(f"  {phase}: {name}\n    {subprocess.list2cmdline(command)}")
                if overrides:
                    print(f"    Environment: {overrides}")
        return 0
    try:
        before = snapshot(REPO_ROOT)
    except (RuntimeError, OSError) as error:
        print(f"{result_label(False)} Cannot capture source snapshot: {error}")
        return 1
    results = []
    for phase, commands in phases:
        if phase == "Coverage":
            print_info("Building/reusing Cargo tests WITH coverage instrumentation.")
        elif any(command[0] == "cargo" for _, command, _ in commands):
            print_info("Building/reusing Cargo artifacts WITHOUT coverage instrumentation.")
        for name, command, overrides in commands:
            print(f"[RUN] {name}", flush=True)
            ok, detail, elapsed = execute_command(command, overrides, log_level=args.log_level)
            results.append(ok)
            print(f"{result_label(ok)} {name} ({elapsed:.2f}s): {detail}", flush=True)
            if args.command == "runner" and args.action == "profile" and not ok:
                break
    try:
        unchanged = before == snapshot(REPO_ROOT)
    except (RuntimeError, OSError):
        unchanged = False
    if not unchanged:
        print(f"{result_label(False)} Source or index changed during verification. Verify the current snapshot again.")
        results.append(False)
    print(f"{result_label(all(results))} {args.command} verification")
    return 0 if all(results) else 1


if __name__ == "__main__":
    sys.exit(main())

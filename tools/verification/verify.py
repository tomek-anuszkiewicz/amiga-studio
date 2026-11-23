#!/usr/bin/env python3
"""Run explicitly selected repository checks and report their actual outcomes."""
import subprocess
import sys
import io
from contextlib import redirect_stderr
import http.client
import re
import tomllib
import urllib.request
import time
from uuid import uuid4

from case_report import CaseReport, run_reported
from cli import parse_args
from commands import select_phases
from execution import COMMAND_LOG, execute_command, print_info, result_label
from repository import REPO_ROOT, collect_changes
from timing_report import measurement, publish

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

STABLE_MANIFEST = "https://static.rust-lang.org/dist/channel-rust-stable.toml"

def stable_update_notice(root):
    """Describe Stable freshness without making it a verification requirement."""
    lines = []
    try:
        active = subprocess.check_output(["rustc", "--version"], cwd=root, text=True,
                                         encoding="utf-8", stderr=subprocess.STDOUT, timeout=5).strip()
        lines.append(f"[INFO] Active verification compiler: {active}")
    except (OSError, subprocess.SubprocessError, UnicodeError) as error:
        lines.append(f"[INFO] Active verification compiler unavailable: {error}")
    try:
        installed = subprocess.check_output(["rustup", "run", "stable", "rustc", "--version"],
                                            cwd=root, text=True, encoding="utf-8",
                                            stderr=subprocess.STDOUT, timeout=5).strip()
        lines.append(f"[INFO] Installed Stable compiler: {installed}")
        with urllib.request.urlopen(STABLE_MANIFEST, timeout=5) as response:
            manifest = tomllib.loads(response.read().decode("utf-8"))
        latest = manifest["pkg"]["rust"]["version"]
        local_match = re.fullmatch(r"rustc (\d+)\.(\d+)\.(\d+)(?: \([^\r\n]+\))?", installed)
        remote_match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)(?: \([^\r\n]+\))?", latest)
        if local_match is None or remote_match is None:
            raise ValueError("Invalid Stable version")
        local_version = tuple(map(int, local_match.groups()))
        remote_version = tuple(map(int, remote_match.groups()))
        version = ".".join(map(str, remote_version))
        if remote_version > local_version:
            lines.append(f"[INFO] Stable update available: {installed.split()[1]} -> {version}. "
                         "Run: rustup update stable")
        else:
            lines.append(f"[INFO] Stable is up to date (latest release: {version}).")
    except (OSError, subprocess.SubprocessError, http.client.HTTPException, ValueError, KeyError, TypeError) as error:
        lines.append(f"[INFO] Could not check Stable updates: {error}")
    return "\n".join(lines) + "\n"



def main(argv=None):
    started = time.perf_counter()
    argv = list(sys.argv[1:] if argv is None else argv)
    preview = not argv or any(value in argv for value in ("--plan", "--help", "-h"))
    if preview:
        return select_and_run(argv, started)
    return run_reported("Verifier", [sys.executable, str(REPO_ROOT / "tools/verification/verify.py"), *argv],
                        lambda report: select_and_run(argv, started, report), cwd=REPO_ROOT)


def standalone_english(args):
    return args.command == "check" and args.name == "english"


def select_and_run(argv, started, archive=None):
    if archive:
        archive.phase("argument selection")
    if archive:
        original = sys.stderr
        with redirect_stderr(io.StringIO()) as diagnostics:
            try:
                parser, args = parse_args(argv)
            except SystemExit:
                archive.event("selection_error", detail=diagnostics.getvalue(), started=0)
                raise
            finally:
                original.write(diagnostics.getvalue())
    else:
        parser, args = parse_args(argv)
    if args is None:
        return 0
    try:
        return run_verification(parser, args, started, archive)
    except OSError as error:
        if archive:
            archive.event("invocation_error", detail=str(error))
        print(f"{result_label(False)} Verification I/O error: {error}")
        return 2 if standalone_english(args) else 1
    except KeyboardInterrupt:
        if archive:
            archive.event("interrupted")
        print(f"{result_label(False)} Verification interrupted.")
        return 130


def run_verification(parser, args, started, archive=None):
    """Execute the requested checks without freezing inputs or reusing verdicts."""
    if archive:
        archive.phase("work selection")
    completion = args.command in ("commit", "milestone")
    automatic_python = args.command == "test" and args.kind == "python" and not args.words
    try:
        if args.results_dir is not None and not args.results_dir.strip():
            raise ValueError("--results-dir requires a nonempty path")
        results_dir = (REPO_ROOT / args.results_dir).resolve() if args.results_dir else None
        changes = collect_changes(REPO_ROOT) if automatic_python or completion else set()
        explanations = []
        if completion:
            explanations.append("Selected paths: " + ", ".join(sorted(changes)))
        phases = select_phases(args, changes, explanations)
    except (RuntimeError, OSError, ValueError) as error:
        if archive:
            archive.event("selection_error", detail=str(error), started=0)
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
        for phase, commands in phases:
            for name, command, overrides in commands:
                print(f"  {phase}: {name}\n    {subprocess.list2cmdline(command)}")
                if overrides:
                    print(f"    Environment: {overrides}")
        return 0
    if not any(commands for _, commands in phases):
        if archive:
            archive.event("empty_selection", started=0)
            archive.event("counts", started=0, completed=0)
        print("[INFO] No Python tool contracts selected: their inputs are unchanged."
              if automatic_python else "[INFO] No checks or tests selected.")
        return 0
    if completion:
        print(stable_update_notice(REPO_ROOT), end="")
    if archive:
        archive.phase("execution")
        for phase, commands in phases:
            for name, command, overrides in commands:
                archive.event("command_selected", phase=phase, name=name, command=command, overrides=overrides)
    try:
        return execute_diagnostic(args, phases, results_dir, started, archive)
    finally:
        if archive:
            archive.merge_children()


def retain_timing(archive, root, timing):
    if "path" in timing:
        artifact = archive.copy(root / timing["path"], "test-runtime-report.md")
        archive.event("command_timing", artifact=archive.link(artifact), sha256=timing["sha256"])



def reserve_group(archive, phase, name, commands):
    if not archive or not archive.reference():
        return None
    argv = commands[0][1] if len(commands) == 1 else [name]
    group = CaseReport.reserve("Verification command group", argv, REPO_ROOT, archive.child_environment())
    if group.reference():
        archive.event("group_started", phase=phase, group=name, invocation_id=group.id,
                      artifact=archive.link(group.path))
    group.phase(phase, group=name)
    return group



def group_command(report, command, overrides, index, log_level, log_dir):
    context = overrides
    if report:
        context = report.child_environment(overrides)
        log = report.attachment(f"command-{index + 1}.log")
        if log:
            context[COMMAND_LOG] = str(log)
        report.event("command_started", command=command, artifact=report.link(log))
    code, detail, seconds = execute_command(command, context, log_level=log_level, log_dir=log_dir)
    if report:
        report.event("command_finished", command=command, outcome="passed" if code == 0 else "failed",
                     detail=detail, seconds=seconds)
        if log and not log.is_file():
            # Tests may replace the executor. A real launch error is already returned by it.
            if detail.startswith("Cannot launch command"):
                report.record_error("Command log is unavailable")
    return code, detail, seconds



def finish_group(report, code):
    if report:
        report.merge_children()
        report.finish(code)



def execute_diagnostic(args, phases, results_dir, started, archive=None):
    own_report = args.command == "runner" and args.action in ("bench", "analyze", "refresh")
    native_english_status = standalone_english(args)
    error_code = 2 if native_english_status else 1
    directory = None if own_report else REPO_ROOT / ".agent/tmp/verification/diagnostics" / uuid4().hex
    phase_timings = [
        [measurement(phase, name, name, "missing", commands=[command])
         for name, command, _ in commands]
        for phase, commands in phases
    ]
    timings = [row for rows in phase_timings for row in rows]
    active = None
    exit_code = 0
    blocked = False
    try:
        active = None
        for (phase, commands), rows in zip(phases, phase_timings):
            if phase != "Preflight" and blocked:
                print("[MISSING] Expensive checks deferred until preliminary blockers pass.")
                continue
            if phase == "Coverage":
                print_info("Building/reusing Cargo tests WITH coverage instrumentation.")
            elif any(command[0] == "cargo" for _, command, _ in commands):
                print_info("Building/reusing Cargo artifacts WITHOUT coverage instrumentation.")
            for (name, command, overrides), row in zip(commands, rows):
                active = row
                print(f"[RUN] {name}", flush=True)
                group = reserve_group(archive, phase, name, [(name, command, overrides)])
                code = None
                try:
                    native_code, detail, elapsed = group_command(
                        group, command, overrides, 0, args.log_level, results_dir / "verify" if results_dir else None)
                    ok = native_code == 0
                    code = native_code if native_english_status else int(not ok)
                except KeyboardInterrupt:
                    code = 130
                    raise
                finally:
                    finish_group(group, code)
                row.update(status="executed" if ok else "failed", seconds=elapsed)
                active = None
                if not ok:
                    exit_code = code
                blocked = blocked or phase in ("Preflight", "Build checks") and not ok
                print(f"{result_label(ok)} {name} ({elapsed:.2f}s): {detail}", flush=True)
                if not ok and (phase == "Coverage" or args.command == "runner" and args.action in ("bench", "profile")):
                    break
    except (KeyboardInterrupt, OSError, RuntimeError) as error:
        if own_report:
            raise
        if isinstance(error, KeyboardInterrupt):
            exit_code = 130
            status = "interrupted"
            print(f"{result_label(False)} Verification interrupted; repeat the same command.")
        else:
            exit_code = error_code
            status = "failed"
            print(f"{result_label(False)} Diagnostic execution failed: {error}")
        if active is None:
            timings.append(measurement("Verification", "Diagnostic", "Diagnostic", status))
        else:
            active["status"] = status
    if directory is not None:
        try:
            published = publish(REPO_ROOT, REPO_ROOT, directory, args.command, timings, time.perf_counter() - started)
            if archive:
                retain_timing(archive, REPO_ROOT, published)
        except (OSError, RuntimeError) as error:
            print(f"{result_label(False)} Cannot write timing report: {error}")
            if exit_code != 130:
                exit_code = error_code
    print(f"{result_label(exit_code == 0)} {args.command} verification")
    return exit_code



if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Repository verification and diagnostic command entry point."""
import subprocess
import sys
import json
import io
from contextlib import redirect_stderr
import http.client
import re
import tomllib
import urllib.request
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from uuid import uuid4

from case_report import CaseReport, run_reported
from cli import parse_args
from commands import independent_contract, select_phases
from execution import COMMAND_LOG, execute_command, print_info, result_label
from repository import REPO_ROOT, collect_changes, snapshot, resolve_base, divergent_integration, candidate_tree
from evidence import BatchEvidence, capture_inputs, execution_conditions, group_identity
from batch_lock import VerificationLock
from timing_report import measurement, publish, omissions


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
    if preview or (argv and argv[0] == "integrate"):
        return select_and_run(argv, started)
    return run_reported("Verifier", [sys.executable, str(REPO_ROOT / "tools/verification/verify.py"), *argv],
                        lambda report: select_and_run(argv, started, report), cwd=REPO_ROOT)


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
    if args.command == "integrate":
        from integration import dispatch
        try:
            return dispatch(REPO_ROOT, args)
        except (OSError, ValueError, KeyError, TypeError, RuntimeError, subprocess.SubprocessError) as error:
            print(f"{result_label(False)} {error}")
            return 1
    if args.plan:
        return run_verification(parser, args, started, archive)
    if archive:
        archive.phase("entry-point lock")
    try:
        with VerificationLock(REPO_ROOT):
            return run_verification(parser, args, started, archive)
    except BlockingIOError as error:
        if archive:
            archive.event("lock_rejected", detail=str(error), started=0)
        print(f"{result_label(False)} {error}")
        return 1
    except OSError as error:
        if archive:
            archive.event("invocation_error", detail=str(error))
        print(f"{result_label(False)} Verification I/O error: {error}")
        return 1
    except KeyboardInterrupt:
        if archive:
            archive.event("interrupted")
        print(f"{result_label(False)} Verification interrupted; repeat the same command.")
        return 130


def run_verification(parser, args, started, archive=None):
    """Select and execute work under the entry-point lock, or preview it without one."""
    if archive:
        archive.phase("work selection")
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
        base = getattr(args, "integration_base", None)
        if base is not None:
            if not completion:
                raise ValueError("--integration-base requires a completion profile")
            base = resolve_base(REPO_ROOT, base)
        original_source = args.integration_source
        if original_source is not None:
            if base is None:
                raise ValueError("--integration-source requires --integration-base and a completion profile")
            original_source = resolve_base(REPO_ROOT, original_source)
        changes = (collect_changes(REPO_ROOT, base) if base else collect_changes(REPO_ROOT)) if automatic_python or completion else set()
        args.integration_divergent = bool(base and divergent_integration(REPO_ROOT, base, original_source))
        if completion and not args.plan:
            batch = BatchEvidence(REPO_ROOT, results_dir, args.command, base, original_source)
        explanations = []
        if completion:
            explanations.append("Comparison base: " + (base or "HEAD"))
            if original_source:
                explanations.append("Original integration source: " + original_source)
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
        if archive:
            archive.event("empty_selection", started=0)
            archive.event("counts", started=0, completed=0)
        print("[INFO] No Python tool contracts selected: their inputs are unchanged."
                  if automatic_python else "[INFO] No checks or tests selected.")
        return 0
    if archive:
        archive.phase("execution")
        for phase, commands in phases:
            for name, command, overrides in commands:
                archive.event("command_selected", phase=phase, name=name, command=command, overrides=overrides)
    try:
        if batch is not None:
            return execute_batch(args, phases, batch, before, started, archive)
        return execute_diagnostic(args, phases, results_dir, started, archive)
    finally:
        if archive:
            if batch is not None:
                retain_timing(archive, batch.directory, batch.record.get("timing_report", {}))
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
    ok, detail, seconds = execute_command(command, context, log_level=log_level, log_dir=log_dir)
    if report:
        report.event("command_finished", command=command, outcome="passed" if ok else "failed",
                     detail=detail, seconds=seconds)
        if log and not log.is_file():
            # Tests may replace the executor. A real launch error is already returned by it.
            if detail.startswith("Cannot launch command"):
                report.record_error("Command log is unavailable")
    return ok, detail, seconds


def finish_group(report, code):
    if report:
        report.merge_children()
        report.finish(code)


def reused_group(archive, phase, name, item):
    if not archive:
        return
    evidence = item.get("case_report")
    path = None
    if evidence and archive.reference():
        candidate = (archive.primary / evidence["path"]).resolve()
        if candidate.is_relative_to(archive.primary / ".agent/task-archives") and candidate.is_file():
            path = candidate
    archive.event("group_reused", phase=phase, group=name, prior_attempt=item["attempt"], new_attempts=0,
                  invocation_id=evidence.get("invocation_id") if evidence else None, artifact=archive.link(path))


def execute_diagnostic(args, phases, results_dir, started, archive=None):
    own_report = args.command == "runner" and args.action in ("bench", "analyze", "refresh")
    directory = None if own_report else REPO_ROOT / ".agent/tmp/verification/diagnostics" / uuid4().hex
    phase_timings = [
        [measurement(phase, name, name, "missing", commands=[command])
         for name, command, _ in commands]
        for phase, commands in phases
    ]
    timings = [row for rows in phase_timings for row in rows]
    stability = measurement("Stability", "Source and index", "Source and index", "missing")
    active = stability
    exit_code = 0
    blocked = False
    try:
        before = None if own_report else snapshot(REPO_ROOT)
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
                    ok, detail, elapsed = group_command(group, command, overrides, 0, args.log_level,
                                                        results_dir / "verify" if results_dir else None)
                    code = 0 if ok else 1
                except KeyboardInterrupt:
                    code = 130
                    raise
                finally:
                    finish_group(group, code)
                row.update(status="executed" if ok else "failed", seconds=elapsed)
                active = None
                if not ok:
                    exit_code = 1
                blocked = blocked or phase in ("Preflight", "Build checks") and not ok
                print(f"{result_label(ok)} {name} ({elapsed:.2f}s): {detail}", flush=True)
                if not ok and (phase == "Coverage" or args.command == "runner" and args.action in ("bench", "profile")):
                    break
        active = stability
        try:
            unchanged = own_report or before == snapshot(REPO_ROOT)
        except (RuntimeError, OSError):
            unchanged = False
        if not unchanged:
            print(f"{result_label(False)} Source or index changed during verification. Verify the current snapshot again.")
            exit_code = 1
            stability["status"] = "failed"
            timings.append(stability)
    except (KeyboardInterrupt, OSError, RuntimeError) as error:
        if own_report:
            raise
        if isinstance(error, KeyboardInterrupt):
            exit_code = 130
            status = "interrupted"
            print(f"{result_label(False)} Verification interrupted; repeat the same command.")
        else:
            exit_code = 1
            status = "failed"
            label = "Cannot capture source snapshot" if active is stability else "Diagnostic execution failed"
            print(f"{result_label(False)} {label}: {error}")
        if active is None:
            timings.append(measurement("Verification", "Diagnostic", "Diagnostic", status))
        else:
            active["status"] = status
            if active is stability:
                timings.append(stability)
    if directory is not None:
        try:
            published = publish(REPO_ROOT, REPO_ROOT, directory, args.command, timings, time.perf_counter() - started)
            if archive:
                retain_timing(archive, REPO_ROOT, published)
        except (OSError, RuntimeError) as error:
            print(f"{result_label(False)} Cannot write timing report: {error}")
            if exit_code == 0:
                exit_code = 1
    print(f"{result_label(exit_code == 0)} {args.command} verification")
    return exit_code


def execution_waves(groups):
    """Keep builds and harness writers separate from the audited concurrent work."""
    preflight = [group for group in groups if group[0] == "Preflight"]
    if preflight:
        yield preflight
    for group in groups:
        if group[0] == "Build checks":
            yield [group]
    remaining = [group for group in groups if group[0] not in {"Preflight", "Build checks"}]
    # Multiple coverage bundles would share counters: retain serial execution.
    coverage = [group for group in remaining if group[0] == "Coverage"]
    parallel = [group for group in remaining if independent_contract(group[1], group[2])]
    if len(coverage) == 1:
        parallel = coverage + parallel
    if parallel:
        yield parallel
    for group in remaining:
        if group not in parallel:
            yield [group]


def execute_group(phase, name, commands, args, directory, timings, report=None):
    """Workers update their own timing rows; the coordinator saves evidence."""
    success, elapsed = True, 0
    code = None
    try:
        for index, (title, command, overrides) in enumerate(commands):
            print(f"[RUN] {title}", flush=True)
            ok, detail, seconds = group_command(report, command, overrides, index, args.log_level, directory)
            elapsed += seconds
            timings[index] = measurement(phase, name, title, "executed" if ok else "failed", seconds,
                                         commands=[command])
            success = success and ok
            print(f"{result_label(ok)} {title} ({seconds:.2f}s): {detail}", flush=True)
            if phase == "Coverage" and not ok:
                break
        code = 0 if success else 1
    except KeyboardInterrupt:
        code = 130
        raise
    finally:
        finish_group(report, code)
    return success, elapsed


def execute_batch(args, phases, batch, before, started, archive=None):
    """Execute the full selected profile, retaining only matching successful groups."""
    batch.directory.mkdir(parents=True, exist_ok=True)
    groups = [(phase, "Coverage" if phase == "Coverage" else items[0][0], items)
              for phase, commands in phases
              for items in ([commands] if phase == "Coverage" else [[item] for item in commands]) if items]
    group_timings = {
        name: [measurement(phase, name, title, "missing", commands=[command])
               for title, command, _ in commands]
        for phase, name, commands in groups
    }

    def attempt_timings(terminal=None):
        rows = omissions(phases)
        for group_rows in group_timings.values():
            rows.extend(group_rows)
        if terminal:
            rows.append(measurement("Verification", "Completion", "Complete the attempt", terminal))
        return rows

    attempt_started = False
    try:
        batch.load()
        batch.record.update(attempt=batch.attempt, outcome="running", results={})
        for key in ("required", "scope", "inputs", "conditions", "source", "tree", "timing_report"):
            batch.record.pop(key, None)
        manifest = batch.attempt_directory / "inputs.json"
        manifest.parent.mkdir(parents=True)
        attempt_started = True
        input_state = {}
        inputs = capture_inputs(REPO_ROOT, state=input_state)
        conditions = execution_conditions(REPO_ROOT)
        source = snapshot(REPO_ROOT)
        if source != before:
            raise ValueError("Source or index changed during selection")
        batch.record["required"] = [name for _, name, _ in groups]
        base = getattr(args, "integration_base", None)
        batch.record["scope"] = sorted(collect_changes(REPO_ROOT, base) if base else collect_changes(REPO_ROOT))
        manifest.write_text(json.dumps(inputs, indent=2) + "\n", encoding="utf-8")
        batch.record["inputs"] = manifest.relative_to(batch.directory).as_posix()
        batch.record["conditions"] = conditions
        batch.save()
        if args.command == "commit":
            notice = stable_update_notice(REPO_ROOT)
            print(notice, end="", flush=True)
            try:
                (manifest.parent / "stable-update.log").write_text(notice, encoding="utf-8")
            except OSError as error:
                print(f"[INFO] Could not save Stable update notice: {error}")
        results = batch.record["results"]
        blocked = False
        def guard_inputs():
            nonlocal input_state
            if snapshot(REPO_ROOT) != source:
                raise ValueError("Source or index changed during verification")
            current_state = capture_inputs(REPO_ROOT, contents=False)
            if current_state != input_state and capture_inputs(REPO_ROOT) != inputs:
                raise ValueError("Source or index changed during verification")
            input_state = current_state

        pool = ThreadPoolExecutor(max_workers=2)
        pending = {}
        try:
            for wave in execution_waves(groups):
                if wave[0][0] != "Preflight" and blocked:
                    for _, name, _ in wave:
                        results[name] = {"status": "missing", "reason": "preliminary blocker"}
                        print(f"[MISSING] {name}: preliminary blocker")
                    continue
                guard_inputs()
                pending = {}
                for phase, name, commands in wave:
                    identity = group_identity(name, commands, inputs, conditions)
                    reason = batch.reason(name, identity)
                    if reason is None:
                        item = batch.record["groups"][name]
                        results[name] = {"status": "reused", "attempt": item["attempt"]}
                        group_timings[name] = [measurement(
                            phase, name, name, "reused", previous_seconds=item.get("elapsed_seconds"),
                            commands=[command for _, command, _ in commands])]
                        reused_group(archive, phase, name, item)
                        print(f"[REUSED] {name}: matching inputs and artifacts; attempt {item['attempt']}")
                        continue
                    old = batch.record["groups"].get(name)
                    label = "INVALIDATED" if old and old.get("status") == "passed" else "MISSING"
                    print(f"[{label}] {name}: {reason}")
                    directory = batch.start(name, identity, commands, source)
                    report = reserve_group(archive, phase, name, commands)
                    if report and report.reference():
                        batch.record["groups"][name]["case_report"] = report.reference()
                    future = pool.submit(
                        execute_group, phase, name, commands, args, directory, group_timings[name], report)
                    pending[future] = (phase, name, directory, reason)
                for future in as_completed(pending):
                    phase, name, directory, reason = pending[future]
                    success, elapsed = future.result()
                    success = batch.finish(name, directory, success, elapsed)
                    results[name] = {"status": "executed" if success else "failed", "reason": reason,
                                     "attempt": batch.attempt}
                    blocked = blocked or phase in ("Preflight", "Build checks") and not success
                # Inspect inputs only after all writers in this wave have stopped.
                guard_inputs()
        finally:
            for future in pending:
                future.cancel()
            pool.shutdown(wait=True, cancel_futures=True)
        if snapshot(REPO_ROOT) != source or capture_inputs(REPO_ROOT) != inputs or execution_conditions(REPO_ROOT) != conditions:
            raise ValueError("Source, index or execution conditions changed during verification")
        success = all(item["status"] in ("executed", "reused") for item in results.values())
        batch.record["source"] = source
        batch.record["tree"] = candidate_tree(REPO_ROOT)
        success = write_batch_timing(batch, args.command, attempt_timings(), started) and success
        batch.record["outcome"] = "passed" if success else "failed"
        batch.save()
        print(f"{result_label(success)} {args.command} verification; evidence: {batch.path}")
        return 0 if success else 1
    except KeyboardInterrupt:
        if attempt_started and "timing_report" not in batch.record:
            write_batch_timing(batch, args.command, attempt_timings("interrupted"), started)
        batch.record["outcome"] = "interrupted"
        batch.save()
        print(f"{result_label(False)} Verification interrupted; repeat the same command and results directory.")
        return 130
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        for item in batch.record["groups"].values():
            if item.get("attempt") == batch.attempt:
                item["status"] = "invalidated"
        batch.record["outcome"] = "failed"
        if attempt_started and "timing_report" not in batch.record:
            write_batch_timing(batch, args.command, attempt_timings("failed"), started)
        batch.save()
        print(f"{result_label(False)} {error}")
        return 1


def write_batch_timing(batch, profile, timings, started):
    try:
        batch.record["timing_report"] = publish(REPO_ROOT, batch.directory, batch.attempt_directory,
                                                 profile, timings, time.perf_counter() - started)
    except (OSError, RuntimeError) as error:
        batch.record["timing_report"] = {"error": str(error)}
        print(f"{result_label(False)} Cannot write timing report: {error}")
        return False
    return True


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Repository verification and diagnostic command entry point."""
import subprocess
import sys
import json
import http.client
import re
import tomllib
import urllib.request
import time

from cli import parse_args
from commands import select_phases
from execution import execute_command, print_info, result_label
from repository import REPO_ROOT, collect_changes, snapshot, resolve_base, divergent_integration, candidate_tree
from evidence import BatchEvidence, capture_inputs, execution_conditions, group_identity
from batch_lock import BatchLock
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
    parser, args = parse_args(sys.argv[1:] if argv is None else argv)
    if args is None:
        return 0
    if args.command == "integrate":
        from integration import dispatch
        try:
            return dispatch(REPO_ROOT, args)
        except (OSError, ValueError, KeyError, TypeError, RuntimeError, subprocess.SubprocessError) as error:
            print(f"{result_label(False)} {error}")
            return 1
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
        return execute_batch(args, phases, batch, before, started)
    results = []
    timings = []
    blocked = False
    for phase, commands in phases:
        if phase != "Preflight" and blocked:
            print("[MISSING] Expensive checks deferred until preliminary blockers pass.")
            timings.extend(measurement(phase, name, name, "missing") for name, _, _ in commands)
            continue
        if phase == "Coverage":
            print_info("Building/reusing Cargo tests WITH coverage instrumentation.")
        elif any(command[0] == "cargo" for _, command, _ in commands):
            print_info("Building/reusing Cargo artifacts WITHOUT coverage instrumentation.")
        for index, (name, command, overrides) in enumerate(commands):
            print(f"[RUN] {name}", flush=True)
            ok, detail, elapsed = execute_command(command, overrides, log_level=args.log_level,
                                                  log_dir=results_dir / "verify" if results_dir else None)
            results.append(ok)
            timings.append(measurement(phase, name, name, "executed" if ok else "failed", elapsed,
                                       commands=[command]))
            blocked = blocked or phase in ("Preflight", "Build checks") and not ok
            print(f"{result_label(ok)} {name} ({elapsed:.2f}s): {detail}", flush=True)
            if phase == "Coverage" and not ok:
                timings.extend(measurement(phase, title, title, "missing")
                               for title, _, _ in commands[index + 1:])
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
        timings.append(measurement("Stability", "Source and index", "Source and index", "failed"))
    if not produces_reports:
        try:
            publish(REPO_ROOT, args.command, timings, time.perf_counter() - started)
        except OSError as error:
            print(f"{result_label(False)} Cannot write timing report: {error}")
            results.append(False)
    print(f"{result_label(all(results))} {args.command} verification")
    return 0 if all(results) else 1



def execute_batch(args, phases, batch, before, started):
    """Execute the full selected profile, retaining only matching successful groups."""
    batch.directory.mkdir(parents=True, exist_ok=True)
    timings = omissions(phases)
    lock = BatchLock(batch.directory)
    try:
        lock.acquire()
    except (OSError, RuntimeError) as error:
        print(f"{result_label(False)} {error}")
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
        base = getattr(args, "integration_base", None)
        batch.record["scope"] = sorted(collect_changes(REPO_ROOT, base) if base else collect_changes(REPO_ROOT))
        batch.record["attempt"] = batch.attempt
        batch.record["outcome"] = "running"
        batch.record["results"] = {}
        manifest = batch.directory / "verify" / batch.attempt / "inputs.json"
        manifest.parent.mkdir(parents=True, exist_ok=True)
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
        for phase, name, commands in groups:
            identity = group_identity(name, commands, inputs, conditions)
            reason = batch.reason(name, identity)
            if phase != "Preflight" and blocked:
                results[name] = {"status": "missing", "reason": "preliminary blocker"}
                timings.append(measurement(phase, name, name, "missing"))
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
                timings.append(measurement(phase, name, name, "reused",
                                           previous_seconds=item.get("elapsed_seconds"),
                                           commands=[command for _, command, _ in commands]))
                print(f"[REUSED] {name}: matching inputs and artifacts; attempt {item['attempt']}")
                continue
            old = batch.record["groups"].get(name)
            label = "INVALIDATED" if old and old.get("status") == "passed" else "MISSING"
            print(f"[{label}] {name}: {reason}")
            directory = batch.start(name, identity, commands, source)
            success, elapsed = True, 0
            for index, (title, command, overrides) in enumerate(commands):
                print(f"[RUN] {title}", flush=True)
                ok, detail, seconds = execute_command(command, lock.command_environment(overrides),
                                                      log_level=args.log_level, log_dir=directory, job=lock.job)
                elapsed += seconds
                timings.append(measurement(phase, name, title, "executed" if ok else "failed", seconds,
                                           commands=[command]))
                success = success and ok
                print(f"{result_label(ok)} {title} ({seconds:.2f}s): {detail}", flush=True)
                if phase == "Coverage" and not ok:
                    timings.extend(measurement(phase, name, title, "missing")
                                   for title, _, _ in commands[index + 1:])
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
        batch.record["source"] = source
        try:
            batch.record["timing_report"] = publish(REPO_ROOT, args.command, timings,
                                                     time.perf_counter() - started)
        except OSError as error:
            batch.record["timing_report"] = {"error": str(error)}
            print(f"{result_label(False)} Cannot write timing report: {error}")
            success = False
        batch.record["outcome"] = "passed" if success else "failed"
        batch.record["tree"] = candidate_tree(REPO_ROOT)
        batch.save()
        lock.complete()
        print(f"{result_label(success)} {args.command} verification; evidence: {batch.path}")
        return 0 if success else 1
    except KeyboardInterrupt:
        batch.record["outcome"] = "interrupted"
        batch.save()
        print(f"{result_label(False)} Verification interrupted; repeat the same command after its commands stop.")
        return 130
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        for item in batch.record["groups"].values():
            if item.get("attempt") == batch.attempt:
                item["status"] = "invalidated"
        batch.record["outcome"] = "failed"
        batch.save()
        print(f"{result_label(False)} {error}")
        return 1
    finally:
        lock.release()


if __name__ == "__main__":
    sys.exit(main())

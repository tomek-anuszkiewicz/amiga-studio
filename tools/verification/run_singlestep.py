"""Build and launch the aggregate SingleStep test; Rust finalizes its own reports."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import uuid

from repository import REPO_ROOT


def build_artifact(profile):
    command = ["cargo", "test", "--profile", profile, "-p", "test_runner",
               "--test", "test_singlestep", "--no-run", "--message-format=json-render-diagnostics"]
    executable = None
    with subprocess.Popen(command, cwd=REPO_ROOT, stdout=subprocess.PIPE, text=True,
                          encoding="utf-8", errors="replace") as process:
        for line in process.stdout:
            message = json.loads(line)
            if message.get("reason") == "compiler-message":
                rendered = message["message"].get("rendered")
                if rendered:
                    print(rendered, end="", file=sys.stderr)
            elif message.get("reason") == "compiler-artifact":
                target = message.get("target", {})
                selected = target.get("name") == "test_singlestep" and "test" in target.get("kind", [])
                cpu_library = target.get("name") == "cpu" and "lib" in target.get("kind", [])
                if selected or cpu_library:
                    settings = message.get("profile", {})
                    if profile == "verification" and not (
                        settings.get("opt_level") == "3"
                        and settings.get("debug_assertions") is True
                        and settings.get("overflow_checks") is True
                    ):
                        raise RuntimeError("Verification requires optimization level 3, debug assertions "
                                           "and overflow checks; Cargo configuration overrides changed them")
                    if selected:
                        executable = message.get("executable")
        code = process.wait()
    if not code and not executable:
        raise RuntimeError("Cargo did not report the test_singlestep executable")
    return code, executable


def completion(directory, run_id, exit_code):
    metadata = json.loads((directory / "run.json").read_text(encoding="utf-8"))
    timing_path = directory / "timing.json"
    timing = json.loads(timing_path.read_text(encoding="utf-8")) if timing_path.exists() else {}
    complete = (metadata.get("id") == run_id
                and metadata.get("status") in ("passed", "failed")
                and metadata.get("selected_tests", 0) > 0
                and metadata.get("completed_tests") == metadata.get("selected_tests")
                and timing.get("run_id") == run_id)
    if not complete:
        metadata.update(id=run_id, status="incomplete")
        timing = {}
    elif exit_code or timing.get("report_exit_code"):
        metadata["status"] = "failed"
    metadata["exit_code"] = exit_code
    (directory / "run.json").write_text(json.dumps(metadata, indent=2), encoding="utf-8")
    return metadata, timing


def execute_tests(executable, options, directory):
    run_id = uuid.uuid4().hex
    env = dict(os.environ, SINGLESTEP_RESULTS_DIR=str(directory), SINGLESTEP_RUN_ID=run_id,
               SINGLESTEP_PROFILE=options.profile, SINGLESTEP_CASE=options.case or "",
               SINGLESTEP_EXACT="1" if options.exact else "0")
    if options.full:
        env.update(SINGLESTEP_FULL="1", SINGLESTEP_LIMIT="all")
    elif options.sample is not None:
        env.update(SINGLESTEP_FULL="0", SINGLESTEP_LIMIT=str(options.sample))
    metadata = dict(id=run_id, status="running", profile=options.profile,
                    selected_tests=0, completed_tests=0, exit_code=None)
    (directory / "run.json").write_text(json.dumps(metadata, indent=2), encoding="utf-8")
    log_path = directory / "execution.log"
    started = time.perf_counter()
    with log_path.open("w", encoding="utf-8") as log:
        code = subprocess.call([executable, "--quiet", "--nocapture"], cwd=REPO_ROOT, env=env,
                               stdout=log, stderr=subprocess.STDOUT)
    process_seconds = time.perf_counter() - started
    metadata, timing = completion(directory, run_id, code)
    label = "PASS" if metadata["status"] == "passed" else "FAIL"
    print(f"[{label}] SingleStep groups: {metadata['completed_tests']}/{metadata['selected_tests']} "
          f"completed; status {metadata['status']}.", flush=True)
    if timing:
        timing.update(process_seconds=process_seconds, test_exit_code=code)
        (directory / "timing.json").write_text(json.dumps(timing, indent=2), encoding="utf-8")
        print(f"[INFO] SingleStep execution {timing['execution_seconds']:.3f}s; finalization "
              f"{timing['finalization_seconds']:.3f}s; pipeline {timing['pipeline_seconds']:.3f}s.",
              flush=True)
    else:
        print("[FAIL] No completed Rust report for this execution.", flush=True)
    try:
        location = log_path.relative_to(REPO_ROOT)
    except ValueError:
        location = log_path
    print(f"[LOG] {location}", flush=True)
    if os.environ.get("VERIFICATION_LOG_LEVEL") == "full":
        with log_path.open(encoding="utf-8") as log:
            shutil.copyfileobj(log, sys.stdout)
    return code or (0 if metadata["status"] == "passed" else 1)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=("test", "verification", "release"), default="verification")
    parser.add_argument("--case")
    parser.add_argument("--exact", action="store_true", help="Match the group name exactly; requires --case")
    scope = parser.add_mutually_exclusive_group()
    scope.add_argument("--full", action="store_true")
    scope.add_argument("--sample", type=int)
    options = parser.parse_args(argv)
    if options.sample is not None and options.sample <= 0:
        parser.error("Sample count must be positive")
    if options.exact and not options.case:
        parser.error("--exact requires --case")
    directory = Path(os.environ.get("SINGLESTEP_RESULTS_DIR", REPO_ROOT / ".agent/tmp/singlestep")).resolve()
    directory.mkdir(parents=True, exist_ok=True)
    lock_path = directory / "run.lock"
    try:
        lock = lock_path.open("x", encoding="utf-8")
    except FileExistsError:
        print("Error: this results directory is owned by another run (run.lock).", file=sys.stderr)
        return 1
    try:
        with lock:
            lock.write(str(os.getpid()))
            lock.flush()
            started = time.perf_counter()
            code, executable = build_artifact(options.profile)
            if code:
                return code
            print(f"[INFO] Build/reuse {time.perf_counter() - started:.3f}s "
                  f"(profile {options.profile}); selected tests retain all comparisons.", flush=True)
            return execute_tests(executable, options, directory)
    finally:
        lock_path.unlink()


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, RuntimeError) as error:
        print(f"Error: {error}", file=sys.stderr)
        sys.exit(1)

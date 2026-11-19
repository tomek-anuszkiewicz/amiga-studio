"""Build and launch the aggregate SingleStep test; Rust finalizes its own reports."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import uuid

from case_report import ReportArgumentParser, run_reported
from repository import REPO_ROOT


def build_artifact(profile, log=None, archive=None):
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
                    if log is not None:
                        try:
                            log.write(rendered)
                            log.flush()
                        except OSError as error:
                            archive.record_error(error)
                            log = None
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


def execute_tests(executable, options, directory, archive=None):
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
    output = log_path
    command = [executable, "--quiet", "--nocapture"]
    if archive is not None:
        native_log = archive.attachment("singlestep-execution.log")
        if native_log:
            try:
                native_log.touch(exist_ok=False)
                output = native_log
            except OSError as error:
                archive.record_error(error)
        index = archive.attachment("groups.md")
        if index:
            env["SINGLESTEP_CASE_REPORT_DIR"] = str(index.parent)
            archive.event("vector_groups", artifact=archive.link(index))
        if output != log_path:
            archive.event("execution_log", artifact=archive.link(output))
        archive.phase("execution", command=command, native_run_id=run_id)
    started = time.perf_counter()
    with output.open("w", encoding="utf-8") as log:
        code = subprocess.call(command, cwd=REPO_ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
    if output != log_path:
        shutil.copyfile(output, log_path)
    process_seconds = time.perf_counter() - started
    metadata, timing = completion(directory, run_id, code)
    if archive is not None and "SINGLESTEP_CASE_REPORT_DIR" in env:
        collect_vector_counts(archive, Path(env["SINGLESTEP_CASE_REPORT_DIR"]))
        archive.event("native_exit", code=code, signal=-code if code < 0 else None, status=metadata["status"])
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


def collect_vector_counts(archive, directory):
    counts = dict(selected=0, started=0, completed=0, passed=0, failed=0)
    try:
        for report in directory.glob("test_*.jsonl"):
            with report.open(encoding="utf-8") as stream:
                for line in stream:
                    row = json.loads(line)
                    if row["event"] == "selection":
                        counts["selected"] += len(row["vectors"])
                    elif row["event"] == "case_started":
                        counts["started"] += 1
                    elif row["event"] == "case_finished":
                        counts["completed"] += 1
                        counts[row["outcome"]] += 1
        if (directory / "incomplete").is_file():
            archive.record_error("SingleStep could not write all native case records")
    except (OSError, ValueError, KeyError) as error:
        archive.record_error(error)
    archive.event("counts", **counts, aborted=counts["started"] - counts["completed"],
                  never_started=counts["selected"] - counts["started"])


def run(argv, archive):
    archive.phase("argument selection")
    parser = ReportArgumentParser(archive, description=__doc__)
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
    archive.phase("results lock", selection=vars(options))
    directory = Path(os.environ.get("SINGLESTEP_RESULTS_DIR", REPO_ROOT / ".agent/tmp/singlestep")).resolve()
    directory.mkdir(parents=True, exist_ok=True)
    lock_path = directory / "run.lock"
    try:
        lock = lock_path.open("x", encoding="utf-8")
    except FileExistsError:
        detail = "This results directory is owned by another run (run.lock)."
        archive.event("lock_rejected", detail=detail)
        print(f"Error: {detail}", file=sys.stderr)
        return 1
    try:
        with lock:
            lock.write(str(os.getpid()))
            lock.flush()
            started = time.perf_counter()
            archive.phase("build", profile=options.profile)
            build_log = archive.attachment("singlestep-build.log")
            log = None
            if build_log:
                try:
                    log = build_log.open("x", encoding="utf-8")
                except OSError as error:
                    archive.record_error(error)
                else:
                    archive.event("build_log", artifact=archive.link(build_log))
            if log is not None:
                with log:
                    code, executable = build_artifact(options.profile, log, archive)
            else:
                code, executable = build_artifact(options.profile)
            if code:
                return code
            print(f"[INFO] Build/reuse {time.perf_counter() - started:.3f}s "
                  f"(profile {options.profile}); selected tests retain all comparisons.", flush=True)
            return execute_tests(executable, options, directory, archive)
    finally:
        lock_path.unlink()


def main(argv=None):
    args = sys.argv[1:] if argv is None else argv
    return run_reported("SingleStep", [sys.executable, str(Path(__file__).resolve()), *args],
                        lambda report: run(args, report), cwd=REPO_ROOT)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("Error: SingleStep execution interrupted", file=sys.stderr)
        sys.exit(130)
    except (OSError, ValueError, RuntimeError) as error:
        print(f"Error: {error}", file=sys.stderr)
        sys.exit(1)

"""Run ordinary verification unittest cases in separate processes."""

import argparse
from collections import Counter, deque
from dataclasses import asdict, dataclass
import importlib
import json
import os
from pathlib import Path
import signal
import sys
import tempfile
import time
import unittest

from worker_processes import CLEANUP_POLL_SECONDS, WorkerProcesses


RUNNER = Path(__file__).resolve()
PROJECT_ROOT = RUNNER.parents[2]
OUTCOME_KINDS = ("error", "failure", "unexpected success", "expected failure", "skipped")
FAILURES = {"error", "failure", "unexpected success"}


@dataclass(frozen=True)
class Case:
    id: str
    module: str
    class_name: str
    method: str
    serial_reason: str | None


def worker_capacity():
    process_count = getattr(os, "process_cpu_count", os.cpu_count)
    logical_count = process_count() or os.cpu_count()
    affinity_count = process_affinity_count()
    if affinity_count:
        logical_count = min(logical_count or affinity_count, affinity_count)
    logical_count = logical_count or 1
    reserve = max(2, (logical_count + 4) // 5)
    return logical_count, reserve, max(1, logical_count - reserve)


def process_affinity_count():
    if hasattr(os, "sched_getaffinity"):
        try:
            return len(os.sched_getaffinity(0))
        except OSError:
            return None
    if os.name == "nt":
        try:
            import psutil
        except ImportError:
            return None
        try:
            return len(psutil.Process().cpu_affinity())
        except (psutil.Error, NotImplementedError, OSError):
            return None
    return None


class CaseLoader(unittest.TestLoader):
    def loadTestsFromModule(self, module, *args, **kwargs):
        if hasattr(module, "load_tests"):
            self.errors.append(f"{module.__name__} defines load_tests. "
                               "The verification runner requires ordinary test cases.")
            return self.suiteClass()
        return super().loadTestsFromModule(module, *args, **kwargs)


def tests_in(suite):
    for test in suite:
        if isinstance(test, unittest.TestSuite):
            yield from tests_in(test)
        else:
            yield test


def discover(directory, pattern, filters):
    loader = CaseLoader()
    if filters:
        loader.testNamePatterns = [word if "*" in word else f"*{word}*" for word in filters]
    cases = {}
    unmarked = object()
    for test in tests_in(loader.discover(str(directory), pattern=pattern)):
        if isinstance(test, unittest.loader._FailedTest):
            # Its original import error is already recorded by the loader.
            continue
        test_class = type(test)
        name = test.id()
        method = name.rsplit(".", 1)[-1]
        class_path = f"{test_class.__module__}.{test_class.__qualname__}"
        if name != f"{class_path}.{method}" or not hasattr(test_class, method):
            loader.errors.append(f"Unsupported test case: {name}")
            continue
        if name in cases:
            loader.errors.append(f"Duplicate test case: {name}")
            continue
        method_reason = getattr(getattr(test_class, method), "serial_reason", unmarked)
        class_reason = getattr(test_class, "serial_reason", unmarked)
        if any(reason is not unmarked and (not isinstance(reason, str) or not reason.strip())
               for reason in (method_reason, class_reason)):
            loader.errors.append(f"{name} needs a non-empty serial reason.")
            continue
        reason = method_reason if method_reason is not unmarked else class_reason
        cases[name] = Case(name, test_class.__module__, test_class.__qualname__, method,
                           None if reason is unmarked else reason)
    return {"cases": [asdict(case) for case in cases.values()], "errors": loader.errors}


class CaseResult(unittest.TestResult):
    def __init__(self):
        super().__init__()
        self.passed = False

    def addSuccess(self, test):
        super().addSuccess(test)
        self.passed = True


def run_case(case):
    test_class = importlib.import_module(case.module)
    for part in case.class_name.split("."):
        test_class = getattr(test_class, part)
    test = test_class(case.method)
    if test.id() != case.id:
        raise ValueError(f"Expected {case.id}, loaded {test.id()}.")
    result = CaseResult()
    started = time.perf_counter()
    unittest.TestSuite([test]).run(result)
    elapsed = time.perf_counter() - started
    entries = (result.errors, result.failures, [(test, "") for test in result.unexpectedSuccesses],
               result.expectedFailures, result.skipped)
    diagnostics = [
        {"id": case.id, "label": observed.id(), "kind": kind, "detail": detail}
        for kind, items in zip(OUTCOME_KINDS, entries, strict=True)
        for observed, detail in items
    ]
    if diagnostics:
        outcome = diagnostics[0]["kind"]
    elif result.passed and result.testsRun == 1:
        outcome = "passed"
    else:
        raise ValueError(f"No unittest outcome for {case.id}.")
    return {"id": case.id, "seconds": elapsed, "outcome": outcome, "diagnostics": diagnostics,
            "serial_reason": case.serial_reason}


def run_worker(request_path):
    if not sys.stdin.buffer.read(1):
        return 1
    request = json.loads(request_path.read_text(encoding="utf-8"))
    sys.path[:0] = [request["directory"], request["caller"], str(PROJECT_ROOT)]
    if request["case"] is None:
        result = discover(request["directory"], request["pattern"], request["filters"])
    else:
        result = run_case(Case(**request["case"]))
    request_path.with_name("result.json").write_text(json.dumps(result), encoding="utf-8")
    return 0


@dataclass(frozen=True)
class Work:
    root: Path
    directory: Path
    caller: Path
    pattern: str
    filters: list[str] | None

    def start(self, processes, number, case=None):
        root = self.root / str(number)
        cwd = root / "cwd"
        temporary = root / "temp"
        cwd.mkdir(parents=True)
        temporary.mkdir()
        request = root / "request.json"
        request.write_text(json.dumps({
            "directory": str(self.directory), "caller": str(self.caller),
            "pattern": self.pattern, "filters": self.filters,
            "case": asdict(case) if case else None,
        }), encoding="utf-8")
        # Git processes must use the fixture's repository and local configuration.
        environment = {key: value for key, value in os.environ.items() if not key.upper().startswith("GIT_")}
        environment.update(
            TMP=str(temporary), TEMP=str(temporary), TMPDIR=str(temporary),
            PYTHONDONTWRITEBYTECODE="1", PYTHONIOENCODING="utf-8",
            GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1",
            GIT_CEILING_DIRECTORIES=str(self.root),
        )
        with (root / "worker.log").open("wb") as log:
            return processes.start([sys.executable, str(RUNNER), "--worker", str(request)],
                                   log, environment, cwd=cwd)

    def result(self, number):
        return json.loads((self.root / str(number) / "result.json").read_text(encoding="utf-8"))

    def output(self, number):
        path = self.root / str(number) / "worker.log"
        if path.is_file():
            return path.read_text(encoding="utf-8", errors="replace")
        return ""


def run_discovery(work, errors):
    with WorkerProcesses() as processes:
        process = work.start(processes, 0)
        while process.poll() is None:
            time.sleep(CLEANUP_POLL_SECONDS)
    if process.returncode == 130:
        raise KeyboardInterrupt
    if process.returncode:
        errors.append(f"Discovery worker exited with status {process.returncode}.")
        return []
    try:
        manifest = work.result(0)
        cases = [Case(**case) for case in manifest["cases"]]
        errors.extend(manifest["errors"])
        return cases
    except (KeyError, OSError, TypeError, ValueError) as error:
        errors.append(f"Discovery worker did not return valid results: {error}")
        return []


def collect_case(work, number, case, code, results, errors):
    if code:
        errors.append(f"{case.id}: worker exited with status {code}.")
        return
    try:
        result = work.result(number)
        if (result["id"] != case.id or result["outcome"] not in (*OUTCOME_KINDS, "passed")
                or not isinstance(result["seconds"], (int, float))
                or not isinstance(result["diagnostics"], list)):
            raise ValueError("result does not match the selected case")
    except (KeyError, OSError, TypeError, ValueError) as error:
        errors.append(f"{case.id}: worker did not return a valid result: {error}")
        return
    results[case.id] = result


def run_phase(work, entries, capacity, results, errors):
    pending = deque(entries)
    with WorkerProcesses() as processes:
        active = {}
        while pending or active:
            while pending and len(active) < capacity:
                number, case = pending.popleft()
                active[number] = case, work.start(processes, number, case)
            interrupted = False
            for number, (case, process) in tuple(active.items()):
                code = process.poll()
                if code is None:
                    continue
                processes.finish(process)
                del active[number]
                collect_case(work, number, case, code, results, errors)
                interrupted |= code == 130
            if interrupted:
                raise KeyboardInterrupt
            if active and (not pending or len(active) == capacity):
                time.sleep(CLEANUP_POLL_SECONDS)


def print_output(label, output):
    if output:
        print(f"Output from {label}:")
        print(output, end="" if output.endswith("\n") else "\n")


def summarize(work, selected, results, errors, interrupted, quiet):
    cases = [results[case.id] for _, case in selected if case.id in results]
    diagnostics = [row for case in cases for row in case["diagnostics"]]
    missing = [case.id for _, case in selected if case.id not in results]
    if missing:
        errors.append("Tests without a completion result: " + ", ".join(missing))
    if interrupted:
        errors.append("Test execution interrupted.")

    print_output("discovery worker", work.output(0))
    for number, case in selected:
        print_output(case.id, work.output(number))
    for row in diagnostics:
        if row["kind"] in FAILURES or not quiet:
            print(f"{row['kind'].upper()}: {row['label']}")
            if row["detail"]:
                print(row["detail"], end="" if row["detail"].endswith("\n") else "\n")
    for problem in errors:
        print(f"[ERROR] {problem}")
    if not quiet:
        for case in cases:
            print(f"{case['id']} ({case['seconds']:.3f}s)")

    counts = Counter(row["kind"] for row in diagnostics)
    if interrupted:
        code = 130
    elif errors or any(case["outcome"] in FAILURES for case in cases):
        code = 1
    else:
        code = 0 if cases else 5
    return code, cases, diagnostics, missing, counts


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if argv[:1] == ["--worker"]:
        return run_worker(Path(argv[1]))
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("-s", "--start-directory", type=Path, default=RUNNER.with_name("tests"))
    parser.add_argument("-p", "--pattern", default="test*.py")
    parser.add_argument("-k", dest="filters", action="append")
    parser.add_argument("-q", "--quiet", action="store_true")
    parser.add_argument("--workers", type=int)
    parser.add_argument("--report", type=Path, help="Write test IDs, durations and outcomes as JSON")
    args = parser.parse_args(argv)
    if args.workers is not None and args.workers < 1:
        parser.error("--workers must be positive")

    started = time.perf_counter()
    logical_count, reserve, default = worker_capacity()
    selected, parallel, serial, errors = [], [], {}, []
    results = {}
    interrupted = False
    workers = 0
    with tempfile.TemporaryDirectory(prefix="verify-tests-") as temporary:
        work = Work(Path(temporary), args.start_directory.resolve(), Path.cwd(), args.pattern, args.filters)
        try:
            selected = list(enumerate(run_discovery(work, errors), 1))
            parallel = [entry for entry in selected if entry[1].serial_reason is None]
            serial = {case.id: case.serial_reason for _, case in selected if case.serial_reason is not None}
            workers = min(args.workers or default, len(parallel))
            print(f"[INFO] Verification tests: {len(selected)} selected, "
                  f"{len(parallel)} parallel, {len(serial)} serial. "
                  f"Parallel workers: {workers} ({logical_count} logical CPUs, {reserve} reserved).",
                  flush=True)
            for name, reason in serial.items():
                print(f"[INFO] Serial test {name}: {reason}")
            run_phase(work, parallel, workers, results, errors)
            run_phase(work, (entry for entry in selected if entry[1].id in serial), 1, results, errors)
        except KeyboardInterrupt:
            interrupted = True
        except OSError as error:
            errors.append(f"Cannot run verification tests: {error}")
        code, cases, diagnostics, missing, counts = summarize(work, selected, results, errors, interrupted, args.quiet)
    elapsed = time.perf_counter() - started
    print(f"Ran {len(cases)} tests in {elapsed:.3f}s")
    summary = ", ".join(f"{kind}={number}" for kind, number in sorted(counts.items()) if number)
    label = {0: "OK", 5: "NO TESTS RAN", 130: "INTERRUPTED"}.get(code, "FAILED")
    print(f"{label} ({summary})" if summary else label)
    if args.report:
        args.report.write_text(json.dumps({
            "exit_code": code, "logical_cpu_count": logical_count, "reserved_cpu_count": reserve, "workers": workers,
            "seconds": elapsed, "selected": [case.id for _, case in selected], "cases": cases,
            "parallel_count": len(parallel), "serial_count": len(serial), "serial": serial,
            "diagnostics": diagnostics, "missing": missing, "errors": errors,
        }, indent=2) + "\n", encoding="utf-8")
    return code


def interrupt(signum, frame):
    raise KeyboardInterrupt


if __name__ == "__main__":
    if os.name != "nt":
        signal.signal(signal.SIGTERM, interrupt)
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        print("[ERROR] Test execution interrupted.")
        raise SystemExit(130)
    except (OSError, ImportError, ValueError) as error:
        print(f"[ERROR] {error}")
        raise SystemExit(1)

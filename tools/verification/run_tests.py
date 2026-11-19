"""Run ordinary verification unittest cases in separate processes."""

from collections import Counter, deque
from contextlib import contextmanager
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

from case_report import CONTEXT, ReportArgumentParser, run_reported
from repository import git
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


def discover(directory, pattern, filters, exact=()):
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
    if exact:
        missing = [name for name in exact if name not in cases]
        if missing:
            loader.errors.append("Unknown unittest IDs: " + ", ".join(missing))
        cases = {name: case for name, case in cases.items() if name in exact}
    return {"cases": [asdict(case) for case in cases.values()], "errors": loader.errors}


class CaseResult(unittest.TestResult):
    def __init__(self, log):
        super().__init__()
        self.passed = False
        self.log = log
        self.subtests = []
        self.active_subtests = []

    def output_offset(self):
        if self.log is None:
            return None
        try:
            sys.stdout.flush()
            sys.stderr.flush()
            return self.log.stat().st_size
        except OSError:
            return None

    @contextmanager
    def timed_subtest(self, test, native, *args, **kwargs):
        frame = {"output_start": self.output_offset()}
        self.active_subtests.append(frame)
        try:
            with native(*args, **kwargs):
                active = test._subtest
                if active is not None:
                    frame.update(id=active.id(), outcome="unavailable")
                started = time.perf_counter()
                try:
                    yield
                finally:
                    frame["seconds"] = time.perf_counter() - started
        finally:
            self.active_subtests.pop()
            if "id" in frame:
                frame["output_end"] = self.output_offset()
                self.subtests.append(frame)

    def addSubTest(self, test, subtest, err):
        super().addSubTest(test, subtest, err)
        frame = self.active_subtests[-1]
        if err is None:
            frame.update(id=subtest.id(), outcome="passed")
        else:
            kind = "failure" if issubclass(err[0], test.failureException) else "error"
            frame.update(id=subtest.id(), outcome=kind, detail=self._exc_info_to_string(err, subtest))

    def addSkip(self, test, reason):
        super().addSkip(test, reason)
        if self.active_subtests:
            self.active_subtests[-1].update(id=test.id(), outcome="skipped", detail=reason)

    def addSuccess(self, test):
        super().addSuccess(test)
        self.passed = True


def run_case(case, log=None):
    test_class = importlib.import_module(case.module)
    for part in case.class_name.split("."):
        test_class = getattr(test_class, part)
    test = test_class(case.method)
    if test.id() != case.id:
        raise ValueError(f"Expected {case.id}, loaded {test.id()}.")
    result = CaseResult(log)
    native_subtest = test.subTest
    test.subTest = lambda *args, **kwargs: result.timed_subtest(test, native_subtest, *args, **kwargs)
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
            "serial_reason": case.serial_reason, "subtests": result.subtests}


def run_worker(request_path):
    if not sys.stdin.buffer.read(1):
        return 1
    request = json.loads(request_path.read_text(encoding="utf-8"))
    sys.path[:0] = [request["directory"], request["caller"], str(PROJECT_ROOT)]
    local = request_path.parent
    progress = Path(request["progress"])
    if request["case"] is None:
        result = discover(request["directory"], request["pattern"], request["filters"], request["exact"])
    else:
        reference = {"id": request["case"]["id"], "attempt": 1}
        persist_worker_record(local, progress, "started.json", reference)
        result = run_case(Case(**request["case"]), progress / "worker.log")
        sys.stdout.flush()
        sys.stderr.flush()
    persist_worker_record(local, progress, "result.json", result)
    return 0


def persist_worker_record(local, progress, name, record):
    write_worker_record(local / name, record)
    if progress != local:
        try:
            write_worker_record(progress / name, record)
        except OSError as error:
            (local / "archive-error").write_text(str(error), encoding="utf-8")


def write_worker_record(path, record):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(record), encoding="utf-8")
    temporary.replace(path)


@dataclass(frozen=True)
class Work:
    root: Path
    directory: Path
    caller: Path
    pattern: str
    filters: list[str] | None
    exact: list[str]
    report: object

    def progress(self, number):
        name = f"worker-{number}"
        return self.report.attachment(name) or self.root / str(number)

    def start(self, processes, number, case=None):
        root = self.root / str(number)
        cwd = root / "cwd"
        temporary = root / "temp"
        cwd.mkdir(parents=True)
        temporary.mkdir()
        progress = self.progress(number)
        try:
            progress.mkdir(exist_ok=True)
            log = (progress / "worker.log").open("wb")
        except OSError as error:
            self.report.record_error(error)
            progress = root
            log = (root / "worker.log").open("wb")
        request = root / "request.json"
        request.write_text(json.dumps({
            "directory": str(self.directory), "caller": str(self.caller),
            "pattern": self.pattern, "filters": self.filters, "exact": self.exact,
            "case": asdict(case) if case else None, "progress": str(progress),
        }), encoding="utf-8")
        # Git processes must use the fixture's repository and local configuration.
        environment = {key: value for key, value in os.environ.items() if not key.upper().startswith("GIT_")}
        environment.update(
            TMP=str(temporary), TEMP=str(temporary), TMPDIR=str(temporary),
            PYTHONDONTWRITEBYTECODE="1", PYTHONIOENCODING="utf-8",
            GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1",
            GIT_CEILING_DIRECTORIES=str(self.root),
        )
        for variable in CONTEXT.values():
            environment.pop(variable, None)
        environment["PYTHONUNBUFFERED"] = "1"
        with log:
            return processes.start([sys.executable, str(RUNNER), "--worker", str(request)],
                                   log, environment, cwd=cwd)

    def observed_path(self, number):
        request = self.root / str(number) / "request.json"
        if request.is_file():
            return Path(json.loads(request.read_text(encoding="utf-8"))["progress"])
        return self.root / str(number)

    def result(self, number):
        local = self.root / str(number)
        error = local / "archive-error"
        if error.is_file():
            self.report.record_error(error.read_text(encoding="utf-8"))
        return json.loads((local / "result.json").read_text(encoding="utf-8"))

    def started(self, number):
        return (self.root / str(number) / "started.json").is_file()

    def output(self, number):
        path = self.observed_path(number) / "worker.log"
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
        detail = f"{case.id}: worker exited with status {code}."
        errors.append(detail)
        work.report.event("worker_exit", id=case.id, exit_code=code, detail=detail)
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
    output = work.observed_path(number) / "worker.log"
    archive_link = work.report.link(output) if output.parent.parent == work.report.files else None
    work.report.event("case_finished", **result, suite=case.module, attempt=1,
                      stack="see diagnostics" if result["diagnostics"] else None, artifact=archive_link)


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


def run_suite(argv, archive):
    archive.phase("argument selection")
    parser = ReportArgumentParser(archive, description=__doc__)
    parser.add_argument("-s", "--start-directory", type=Path, default=RUNNER.with_name("tests"))
    parser.add_argument("-p", "--pattern", default="test*.py")
    parser.add_argument("-k", dest="filters", action="append")
    parser.add_argument("-q", "--quiet", action="store_true")
    parser.add_argument("--workers", type=int)
    parser.add_argument("--report", type=Path, help="Write test IDs, durations and outcomes as JSON")
    parser.add_argument("exact", metavar="TEST_ID", nargs="*", help="Select full discovered unittest IDs")
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
        work = Work(Path(temporary), args.start_directory.resolve(), Path.cwd(), args.pattern,
                    args.filters, args.exact, archive)
        archive.phase("discovery", directory=str(work.directory), pattern=args.pattern,
                      filters=args.filters, exact=args.exact)
        discovery_log = archive.attachment("worker-0")
        archive.event("discovery_worker", artifact=archive.link(discovery_log / "worker.log")
                      if discovery_log else None)
        try:
            selected = list(enumerate(run_discovery(work, errors), 1))
            if selected:
                archive.phase("execution")
                archive.event("recovery", detail="Each selected worker directory can contain "
                              "started.json, result.json and the native worker log. A worker "
                              "without a start was never started. The result records the actual "
                              "outcome even when the launcher could not collect it.")
            for number, case in selected:
                progress = archive.attachment(f"worker-{number}")
                archive.event("case_selected", id=case.id, suite=case.module, attempt=1,
                              artifact=archive.link(progress))
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
        for number, case in selected:
            if case.id in missing:
                progress = work.observed_path(number)
                started = work.started(number)
                archive.event("case_unfinished", id=case.id, suite=case.module, attempt=1,
                              outcome="aborted" if started else "never started", seconds=None,
                              stack=None, artifact=archive.link(progress / "worker.log")
                              if progress.parent == archive.files else None)
        if errors:
            archive.event("execution_errors", errors=errors)
        archive.event("counts", selected=len(selected), completed=len(cases),
                      started=sum(work.started(number) for number, _ in selected),
                      aborted=sum(work.started(number) for number, case in selected if case.id in missing),
                      never_started=sum(not work.started(number) for number, case in selected if case.id in missing))
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


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    if argv[:1] == ["--worker"]:
        return run_worker(Path(argv[1]))
    cwd = Path.cwd()
    try:
        git(cwd, "rev-parse", "--show-toplevel")
    except RuntimeError:
        cwd = PROJECT_ROOT
    return run_reported("unittest", [sys.executable, str(RUNNER), *argv],
                        lambda report: run_suite(argv, report), cwd=cwd)


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

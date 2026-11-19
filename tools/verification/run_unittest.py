"""Run standard unittest discovery or exact IDs and retain native case evidence."""

from functools import partial
import io
from pathlib import Path
import runpy
import sys
import time
import unittest

from case_report import run_reported


class OutputCopy(io.TextIOBase):
    def __init__(self, raw, copy, report):
        self.raw = raw
        self.copy = copy
        self.report = report

    def write(self, text):
        if self.copy:
            try:
                self.copy.write(text)
                self.copy.flush()
            except OSError as error:
                self.report.record_error(error)
                self.copy = None
        return self.raw.write(text)

    def flush(self):
        self.raw.flush()

    def fileno(self):
        return self.raw.fileno()

    def isatty(self):
        return self.raw.isatty()

    @property
    def encoding(self):
        return self.raw.encoding

    @property
    def errors(self):
        return self.raw.errors

    @property
    def buffer(self):
        return self.raw.buffer


class ReportingResult(unittest.TextTestResult):
    def __init__(self, report, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.report = report
        self.active = None
        self.counts = dict(started=0, completed=0, skipped=0)

    def startTest(self, test):
        super().startTest(test)
        self.counts["started"] += 1
        file = self.report.attachment(f"unittest-{self.counts['started']}.log")
        log = None
        if file:
            try:
                log = file.open("x", encoding="utf-8")
            except OSError as error:
                self.report.record_error(error)
        identity = dict(suite=test.id().rsplit(".", 1)[0], id=test.id(), attempt=1)
        self.report.event("case_started", **identity, artifact=self.report.link(file))
        if log:
            self.stdout, self.stderr = sys.stdout, sys.stderr
            sys.stdout, sys.stderr = (OutputCopy(self.stdout, log, self.report),
                                      OutputCopy(self.stderr, log, self.report))
        self.active = dict(identity=identity, path=file, log=log, started=time.perf_counter(),
                           outcome="unavailable", detail=None, stack=None, outer_exception=sys.exc_info()[1])

    def stopTest(self, test):
        # unittest calls stopTest during exception unwinding.
        exception = sys.exc_info()[1]
        interrupted = isinstance(exception, KeyboardInterrupt) and exception is not self.active["outer_exception"]
        elapsed = None if interrupted else time.perf_counter() - self.active["started"]
        self.finish_case(elapsed)
        super().stopTest(test)

    def finish_case(self, elapsed):
        active, self.active = self.active, None
        outcome = active["outcome"] if elapsed is not None else "aborted"
        if active["log"]:
            sys.stdout, sys.stderr = self.stdout, self.stderr
            active["log"].close()
        self.counts["completed" if elapsed is not None else "aborted"] = (
            self.counts.get("completed" if elapsed is not None else "aborted", 0) + 1)
        if outcome == "skipped":
            self.counts["skipped"] += 1
        self.report.event("case_finished" if elapsed is not None else "case_unfinished",
                          **active["identity"], outcome=outcome, detail=active["detail"], seconds=elapsed,
                          stack=active["stack"], artifact=self.report.link(active["path"]))

    def set_outcome(self, test, outcome, detail=None, stack=None):
        if self.active is not None and test.id() == self.active["identity"]["id"]:
            self.active.update(outcome=outcome, detail=detail, stack=stack)
        else:
            self.report.event("fixture_outcome", id=test.id(), outcome=outcome, detail=detail, stack=stack)

    def addSuccess(self, test):
        super().addSuccess(test)
        self.set_outcome(test, "passed")

    def addError(self, test, error):
        super().addError(test, error)
        self.set_outcome(test, "error", self.errors[-1][1], "see traceback")

    def addFailure(self, test, error):
        super().addFailure(test, error)
        self.set_outcome(test, "failure", self.failures[-1][1], "see traceback")

    def addSkip(self, test, reason):
        super().addSkip(test, reason)
        self.set_outcome(test, "skipped", reason)

    def addExpectedFailure(self, test, error):
        super().addExpectedFailure(test, error)
        self.set_outcome(test, "expected failure", self.expectedFailures[-1][1], "see traceback")

    def addUnexpectedSuccess(self, test):
        super().addUnexpectedSuccess(test)
        self.set_outcome(test, "unexpected success")

    def addSubTest(self, test, subtest, error):
        super().addSubTest(test, subtest, error)
        detail = self._exc_info_to_string(error, test) if error else None
        outcome = "failed" if error else "passed"
        if error:
            self.set_outcome(test, "failed", detail, "see traceback")
        self.report.event("subtest_finished", id=subtest.id(), parent_id=test.id(), outcome=outcome,
                          seconds=None, detail=detail, stack="see traceback" if error else None)


def cases_in(suite):
    if isinstance(suite, unittest.TestSuite):
        for test in suite:
            yield from cases_in(test)
    else:
        yield suite


class ReportingRunner(unittest.TextTestRunner):
    def __init__(self, report, *args, resultclass=ReportingResult, **kwargs):
        self.report = report
        super().__init__(*args, resultclass=partial(resultclass, report), **kwargs)

    def _makeResult(self):
        self.result = super()._makeResult()
        return self.result

    def run(self, test):
        selected = [item.id() for item in cases_in(test)]
        self.report.phase("test execution")
        for case in selected:
            self.report.event("case_selected", id=case, attempt=1)
        self.result = None
        try:
            return super().run(test)
        finally:
            if self.result:
                if self.result.active:
                    self.result.finish_case(None)
                counts = self.result.counts
                self.report.event("counts", selected=len(selected),
                                  never_started=len(selected) - counts["started"], **counts)


def execute(argv, report, cwd):
    report.phase("discovery")
    runner = partial(ReportingRunner, report)
    if argv[:1] == ["--architecture"]:
        # The protected checker owns its filters and assertions. Its standard
        # TextTestRunner is replaced only in this isolated launcher process.
        checker = cwd / "tools/verification/checks/check_architecture.py"
        native, original = unittest.TextTestRunner, sys.argv
        unittest.TextTestRunner = runner
        sys.argv = [str(checker), *argv[1:]]
        try:
            runpy.run_path(str(checker), run_name="__main__")
        finally:
            unittest.TextTestRunner, sys.argv = native, original
        return 0
    class BoundRunner(ReportingRunner):
        def __init__(self, *args, **kwargs):
            super().__init__(report, *args, **kwargs)

    sys.path.insert(0, str(cwd))
    unittest.main(module=None, argv=["unittest", *argv], testRunner=BoundRunner)


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    cwd = Path.cwd()
    return run_reported("Python unittest", [sys.executable, str(Path(__file__).resolve()), *argv],
                        lambda report: execute(argv, report, cwd), cwd=cwd)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(130)

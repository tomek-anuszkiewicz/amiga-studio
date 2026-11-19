"""Stream original Cargo and nextest test results into one case report."""

from collections import Counter, defaultdict
import json
from pathlib import Path
import re
import subprocess
import sys

from case_report import run_reported


NEXTEST_ENV = "NEXTEST_EXPERIMENTAL_LIBTEST_JSON"


def is_rust_test(command):
    return (command[:2] == ["cargo", "test"] and "--no-run" not in command
            or command[:3] in (["cargo", "llvm-cov", "nextest"], ["cargo", "nextest", "run"]))


RETRY_STATUS = re.compile(
    r"\s*TRY (\d+) (\S+) \[\s*(\d+(?:\.\d+)?)s\]\s+\([^)]*\)\s+(\S+) (.+)"
)
OUTCOMES = {"ok": "passed", "failed": "failed", "ignored": "skipped", "timeout": "failed"}


class NextestCases:
    def __init__(self, report, log):
        self.report = report
        self.log = log
        self.counts = Counter()
        self.pending = defaultdict(set)
        self.suites = {}
        self.retries = {}
        self.native_line = 0
        self.running = False

    def consume(self, line):
        self.native_line += 1
        try:
            event = json.loads(line)
        except ValueError:
            if match := RETRY_STATUS.fullmatch(line.rstrip()):
                attempt, status, seconds, binary, case = match.groups()
                self.retries[binary, case, int(attempt)] = (status, float(seconds), self.native_line)
            return
        if not isinstance(event, dict):
            return
        if event.get("type") == "suite":
            self.consume_suite(event)
        elif event.get("type") == "test":
            self.consume_case(event)

    def consume_suite(self, event):
        status = event.get("event")
        metadata = event.get("nextest")
        if status == "started" and not self.running:
            self.report.phase("test execution")
            self.running = True
        if metadata is None:
            return
        crate, binary, kind = (metadata[key] for key in ("crate", "test_binary", "kind"))
        suite = f"{crate}::{binary}"
        if "stress_index" in metadata:
            suite += f"@stress-{metadata['stress_index']}"
        if status == "started":
            if kind in {"lib", "proc-macro"}:
                display = crate
            elif kind == "test":
                display = f"{crate}::{binary}"
            else:
                display = f"{crate}::{kind}/{binary}"
            self.suites[suite] = (display, event["test_count"])
        elif status in {"ok", "failed"}:
            _, selected = self.suites.get(suite, (None, None))
            pending = self.pending.get(suite)
            if not pending:
                return
            ignored = event["ignored"]
            resolved = event["passed"] + event["failed"]
            # Some nextest suites emit starts for ignored cases without emitting
            # their individual ignored events. The ignored summary can repeat in
            # later blocks, so only reconcile names observed in this block.
            if len(pending) <= ignored and selected == resolved + ignored:
                del self.pending[suite]
                for case in sorted(pending):
                    self.finish_case(suite, case, 1, "skipped", "suite reported ignored")
            elif ignored == 0:
                del self.pending[suite]
                for case in sorted(pending):
                    self.abort(suite, case)
            else:
                # Nextest may emit a partial block before sending results for
                # the same binary. Keep its unresolved names for the next block.
                self.report.event("suite_counts", suite=suite, ignored=ignored,
                                  unresolved=len(pending), artifact=self.report.link(self.log))

    def consume_case(self, event):
        status = event.get("event")
        name = event.get("name")
        if not isinstance(name, str) or "$" not in name:
            return
        suite, case = name.split("$", 1)
        pending = self.pending[suite]
        if status == "started":
            pending.add(case)
            # A structured start can represent an ignored test. Keep the
            # observation immediately, and count a start once execution is known.
            self.report.event("case_observed", suite=suite, id=case, native_reason=status)
            return
        if status not in OUTCOMES:
            return
        attempt = 1
        if case not in pending and (retry := re.fullmatch(r"(.+)#([2-9]|[1-9]\d+)", case)):
            base, ordinal = retry.groups()
            if base in pending:
                case, attempt = base, int(ordinal)
        pending.discard(case)
        binary, _ = self.suites.get(suite, (suite, None))
        for previous in range(1, attempt):
            status_line = self.retries.pop((binary, case, previous), None)
            if status_line is None:
                self.report.record_error(f"Nextest retry timing is unavailable for {suite}${case}")
            reason, seconds, native_line = status_line or ("nextest retried the failed attempt", None, None)
            self.finish_case(suite, case, previous, "failed", reason, seconds, native_line=native_line)
        self.retries.pop((binary, case, attempt), None)
        self.finish_case(suite, case, attempt, OUTCOMES[status], event.get("reason", status),
                         event.get("exec_time"), event.get("stdout"))

    def finish_case(self, suite, case, attempt, outcome, reason, seconds=None, stdout=None, native_line=None):
        if outcome == "skipped":
            self.counts["skipped"] += 1
        else:
            self.counts["started"] += 1
            self.counts["completed"] += 1
            self.report.event("case_started", suite=suite, id=case, attempt=attempt)
        retry_location = {"native_output_line": native_line} if native_line is not None else {}
        self.report.event("case_finished", suite=suite, id=case, attempt=attempt,
                          outcome=outcome, native_reason=reason, seconds=seconds, detail=stdout, **retry_location,
                          stack="see native output" if stdout else None,
                          artifact=self.report.link(self.log))

    def abort(self, suite, case):
        self.counts["started"] += 1
        self.counts["aborted"] += 1
        self.report.event("case_started", suite=suite, id=case, attempt=1)
        self.report.event("case_unfinished", suite=suite, id=case, attempt=1,
                          outcome="aborted", seconds=None, stack=None,
                          artifact=self.report.link(self.log))

    def finish(self):
        for suite, pending in sorted(self.pending.items()):
            if suite in self.suites and pending:
                # A terminated nextest process may omit the suite's ignored count.
                self.counts["unresolved"] += len(pending)
                for case in sorted(pending):
                    self.report.event("case_unfinished", suite=suite, id=case, attempt=1,
                                      outcome="unavailable", seconds=None, artifact=self.report.link(self.log))
                self.report.record_error(f"Nextest could not identify outcomes in {suite}")
            else:
                for case in sorted(pending):
                    self.abort(suite, case)
        self.report.event("counts", started=self.counts["started"], completed=self.counts["completed"],
                          skipped=self.counts["skipped"], aborted=self.counts["aborted"],
                          unresolved=self.counts["unresolved"])


RUNNING = re.compile(r"\s*(?:Running|Doc-tests)\s+(.+?)\s*$")
TEST_EXECUTION = re.compile(r"^running [1-9]\d* tests?$")
CASE = re.compile(r"^test (.+?) \.\.\. (ok|FAILED|ignored)(?:, (.*))?$")
START = re.compile(r"^test (.+?) \.\.\.(?: (.*))?$")
CARGO_SUMMARY = re.compile(
    r"^test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured;"
)


class CargoCases:
    """Stable libtest reports per-case names and outcomes and aggregate timing."""

    def __init__(self, report, log):
        self.report = report
        self.log = log
        self.suite = "libtest"
        self.counts = Counter()
        self.named = Counter()
        self.failure = None
        self.diagnostic = []
        self.pending = None
        self.running = False

    def start_execution(self):
        if not self.running:
            self.report.phase("test execution")
            self.running = True

    def flush_diagnostic(self):
        if self.failure:
            detail = "".join(self.diagnostic).strip()
            self.report.event("case_diagnostic", suite=self.suite, id=self.failure, attempt=1,
                              detail=detail, stack="see native output" if "stack backtrace:" in detail else None,
                              artifact=self.report.link(self.log))
            self.failure = None
            self.diagnostic = []

    def consume(self, line):
        if match := RUNNING.fullmatch(line.rstrip()):
            self.flush_diagnostic()
            self.abort_pending()
            self.suite = match[1]
            return
        if match := re.fullmatch(r"---- (.+) stdout ----", line.rstrip()):
            self.flush_diagnostic()
            self.failure = match[1]
            return
        if self.failure:
            if line.strip() == "failures:" or line.startswith("test result:"):
                self.flush_diagnostic()
            else:
                self.diagnostic.append(line)
        if TEST_EXECUTION.fullmatch(line.rstrip()):
            self.start_execution()
        elif match := CARGO_SUMMARY.match(line):
            self.reconcile_summary(*(int(value) for value in match.groups()))
        elif match := CASE.fullmatch(line.rstrip()):
            self.start_execution()
            if self.pending != match[1]:
                self.abort_pending()
            self.complete(*match.groups())
            self.pending = None
        elif match := START.fullmatch(line.rstrip()):
            self.start_execution()
            self.abort_pending()
            self.pending = match[1]
            self.report.event("case_started", suite=self.suite, id=self.pending, attempt=1)
        elif self.pending and line.strip() in {"ok", "FAILED", "ignored"}:
            self.complete(self.pending, line.strip(), None)
            self.pending = None

    def reconcile_summary(self, passed, failed, ignored, measured):
        completed = passed + failed + measured
        missing = {
            "completed": max(0, completed - self.named["completed"]),
            "skipped": max(0, ignored - self.named["skipped"]),
        }
        if any(missing.values()):
            self.counts.update(missing)
            self.report.event("suite_counts", suite=self.suite, passed=passed, failed=failed,
                              skipped=ignored, measured=measured, unnamed=missing,
                              artifact=self.report.link(self.log))
            self.report.record_error(f"Cargo omitted individual case results in {self.suite}")
        self.named.clear()
        self.pending = None

    def complete(self, case, result, reason):
        outcome = {"ok": "passed", "FAILED": "failed", "ignored": "skipped"}[result]
        count = "skipped" if result == "ignored" else "completed"
        self.counts[count] += 1
        self.named[count] += 1
        self.report.event("case_finished", suite=self.suite, id=case, attempt=1, outcome=outcome,
                          native_reason=reason or result, seconds=None, stack=None,
                          artifact=self.report.link(self.log))

    def abort_pending(self):
        if self.pending is None:
            return
        self.counts["aborted"] += 1
        self.report.event("case_unfinished", suite=self.suite, id=self.pending, attempt=1,
                          outcome="aborted", seconds=None, stack=None,
                          artifact=self.report.link(self.log))
        self.report.record_error(f"Cargo did not report an outcome for {self.pending} in {self.suite}")
        self.pending = None

    def finish(self):
        self.flush_diagnostic()
        self.abort_pending()
        self.report.event("counts", started=self.counts["completed"] + self.counts["aborted"],
                          completed=self.counts["completed"], skipped=self.counts["skipped"],
                          aborted=self.counts["aborted"], unobserved_starts="unavailable")


def libtest_reporting_options(options):
    # Libtest options that take a separate argument can consume a flag-shaped
    # value. Skip over those values and leave literal filters after -- alone.
    value_options = {"--color", "--format", "--logfile", "--shuffle-seed", "--skip", "--test-threads", "-Z"}
    args = iter(options)
    native = []
    filters = []
    formats = []
    for option in args:
        if option == "--":
            filters = [option, *args]
            break
        if option in {"--quiet", "-q"}:
            continue
        name, equal, value = option.partition("=")
        if name not in value_options:
            native.append(option)
            continue
        if not equal:
            value = next(args, None)
            if value is None:
                return native + [option], formats  # Leave the error to libtest.
        if name == "--format":
            if value == "terse":
                value = "pretty"
            formats.append(value)
        native.extend([f"{name}={value}"] if equal else [name, value])
    if not formats:
        native += ["--format", "pretty"]
    return native + filters, formats


def rust_command(command):
    if command[:2] == ["cargo", "test"]:
        # Keep Cargo's build, targets, selection and native exit contract.
        build, separator, options = command, None, []
        if "--" in command:
            index = command.index("--")
            build, separator, options = command[:index], "--", command[index + 1:]
        native, formats = libtest_reporting_options(options)
        return build + [separator or "--", *native], False, formats
    at = command.index("--") if "--" in command else len(command)
    options = ["--message-format", "libtest-json-plus", "--message-format-version", "0.1"]
    return command[:at] + options + command[at:], True, []


def run(command, report):
    if not is_rust_test(command):
        report.phase("selection")
        report.event("selection_error", detail="Use cargo test, cargo nextest run or cargo llvm-cov nextest")
        print("Use cargo test, cargo nextest run or cargo llvm-cov nextest", file=sys.stderr)
        return 2
    native, nextest, libtest_formats = rust_command(command)
    env = report.child_environment()
    if nextest:
        env[NEXTEST_ENV] = "1"
    log = report.attachment("native-output.log")
    reader = NextestCases(report, log) if nextest else CargoCases(report, log)
    report.phase("build and test selection", command=native, artifact=report.link(log))
    for format_name in sorted(set(libtest_formats) & {"json", "junit"}):
        report.record_error(f"Cargo case parsing is unavailable for libtest format {format_name}")
    stream = None
    if log:
        try:
            stream = log.open("x", encoding="utf-8", newline="\n")
        except OSError as error:
            report.record_error(error)
    try:
        with subprocess.Popen(native, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                              env=env, text=True, encoding="utf-8", errors="replace") as process:
            for line in process.stdout:
                if stream:
                    try:
                        stream.write(line)
                        stream.flush()
                    except OSError as error:
                        report.record_error(error)
                        stream.close()
                        stream = None
                reader.consume(line)
                if not nextest or not line.startswith("{"):
                    print(line, end="", flush=True)
                elif '"event":"failed"' in line or '"event":"timeout"' in line:
                    event = json.loads(line)
                    print(f"[FAIL] {event.get('name', 'nextest case')}", flush=True)
                    if event.get("stdout"):
                        print(event["stdout"], flush=True)
            code = process.wait()
        return code
    finally:
        if stream:
            stream.close()
        reader.finish()


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    command = argv[1:] if argv[:1] == ["--"] else argv
    return run_reported("Rust native test cases", [sys.executable, str(Path(__file__).resolve()), *argv],
                        lambda report: run(command, report), cwd=Path.cwd())


if __name__ == "__main__":
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(130)
    except (OSError, ValueError) as error:
        print(f"Error: {error}", file=sys.stderr)
        sys.exit(1)

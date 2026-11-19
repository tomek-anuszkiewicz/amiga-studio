"""Durable case evidence shared by repository test and measurement launchers."""

import argparse
from datetime import datetime, timezone
import html
import json
import os
from pathlib import Path
import re
import shutil
import stat
import sys
import time
from urllib.parse import quote
from uuid import uuid4

from repository import REPO_ROOT, git


CONTEXT = {
    "task_id": "AMIGA_TEST_TASK_ID",
    "execution_id": "AMIGA_TEST_EXECUTION_ID",
    "work_item": "AMIGA_TEST_WORK_ITEM_ID",
    "round": "AMIGA_TEST_ROUND",
    "parent_invocation_id": "AMIGA_TEST_PARENT_INVOCATION_ID",
}


def utc_now():
    return datetime.now(timezone.utc)


def timestamp(instant):
    return instant.isoformat(timespec="milliseconds").replace("+00:00", "Z")


def identifier(value):
    if (not re.fullmatch(r"[A-Za-z0-9_-][A-Za-z0-9_.-]*", value) or value.endswith(".")
            or re.fullmatch(r"(?i:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\..*)?", value)):
        raise ValueError(f"Invalid archive identifier: {value!r}")
    return value


def primary_checkout(cwd):
    """Git lists the primary checkout first, including for linked worktrees."""
    first = git(cwd, "worktree", "list", "--porcelain", "-z").split(b"\0", 1)[0]
    prefix = b"worktree "
    if not first.startswith(prefix):
        raise ValueError("Git did not identify a primary checkout")
    return Path(os.fsdecode(first[len(prefix):])).resolve()


def inline(value):
    return "<code>" + html.escape(str(value)).replace("\n", "&#10;") + "</code>"


def prepare_archive(primary, archive):
    directory = primary
    for component in archive.relative_to(primary).parts:
        directory /= component
        try:
            directory.mkdir()
        except FileExistsError:
            pass
        state = directory.lstat()
        # Resolve only existing directories. Windows non-strict resolution of a
        # missing descendant can change representation during concurrent creation.
        if (stat.S_ISLNK(state.st_mode)
                or getattr(state, "st_file_attributes", 0) & stat.FILE_ATTRIBUTE_REPARSE_POINT
                or directory.resolve(strict=True) != directory):
            raise ValueError("Test report archive cannot use a redirected path")
        if not stat.S_ISDIR(state.st_mode):
            raise NotADirectoryError(directory)


class CaseReport:
    def __init__(self):
        self.id = None
        self.path = None
        self.journal = None
        self.files = None
        self.metadata = {}
        self.started = time.perf_counter()
        self.current_phase = "startup"
        self.incomplete = False
        self._warned = False

    @classmethod
    def reserve(cls, producer, argv, cwd=REPO_ROOT, environment=None):
        report = cls()
        try:
            report._reserve(producer, list(argv), cwd, os.environ if environment is None else environment)
        except (OSError, RuntimeError, ValueError) as error:
            report.record_error(error)
        return report

    def _reserve(self, producer, argv, cwd, environment):
        instant = utc_now()
        values = {key: environment.get(name) for key, name in CONTEXT.items()}
        if bool(values["task_id"]) != bool(values["execution_id"]):
            raise ValueError("Test report context requires both task and execution IDs")
        values["task_id"] = identifier(values["task_id"] or "standalone")
        values["execution_id"] = identifier(values["execution_id"] or uuid4().hex)
        self.primary = primary_checkout(cwd)
        archive = self.primary / ".agent/task-archives" / values["task_id"] / values["execution_id"] / "test-runs"
        try:
            git(self.primary, "check-ignore", "--quiet", "--", str(archive / "probe"))
        except RuntimeError as error:
            raise ValueError("The primary checkout must ignore the case report archive") from error
        prepare_archive(self.primary, archive)
        revision = git(cwd, "rev-parse", "HEAD").decode("ascii").strip()
        start = timestamp(instant)
        for _ in range(5):
            invocation_id = uuid4().hex
            stem = start.replace(":", "-") + "_" + invocation_id
            path = archive / (stem + ".md")
            try:
                with path.open("x", encoding="utf-8", newline="\n") as stream:
                    stream.write(
                        "# Test and benchmark invocation\n\n"
                        f"[Native event journal]({stem}.jsonl)\n\n"
                        "A missing finish record leaves the invocation unfinished. "
                        "A case start without a result has an unavailable duration and outcome.\n\n"
                    )
            except FileExistsError:
                continue
            self.id = invocation_id
            self.path = path
            self.journal = path.with_suffix(".jsonl")
            self.files = archive / (stem + "-files")
            break
        else:
            raise FileExistsError("Could not reserve a unique report after five collisions")
        print(f"[REPORT] {self.path}", flush=True)
        self.metadata = dict(**values, invocation_id=self.id, producer=producer, argv=argv,
                             revision=revision, started_utc=start, elapsed_unit="seconds")
        self.files.mkdir()
        self.journal.touch(exist_ok=False)
        self.event("invocation", **self.metadata)

    def record_error(self, error):
        self.incomplete = True
        if self._warned:
            return
        self._warned = True
        print(f"[ERROR] Case report evidence is incomplete: {error}", file=sys.stderr, flush=True)
        if self.path:
            try:
                with self.path.open("a", encoding="utf-8", newline="\n") as stream:
                    stream.write(f"\n**Evidence incomplete.** {inline(error)}\n")
            except OSError:
                pass

    def attachment(self, name):
        if self.files is None or self.incomplete:
            return None
        return self.files / identifier(name)

    def copy(self, source, name):
        target = self.attachment(name)
        if target is not None:
            try:
                shutil.copyfile(source, target)
                return target
            except OSError as error:
                self.record_error(error)
        return None

    def link(self, path):
        return Path(os.path.relpath(path, self.path.parent)).as_posix() if path is not None else None

    def child_environment(self, environment=None):
        result = dict(os.environ if environment is None else environment)
        if self.id is not None and self.metadata:
            for key, variable in CONTEXT.items():
                value = self.id if key == "parent_invocation_id" else self.metadata.get(key)
                if value is not None:
                    result[variable] = value
        return result

    def children(self):
        """Read child headers without opening their potentially large case streams."""
        if self.path is None:
            return []
        reports = []
        for journal in sorted(self.path.parent.glob("*.jsonl")):
            try:
                with journal.open(encoding="utf-8") as stream:
                    header = json.loads(stream.readline())
                if header.get("parent_invocation_id") == self.id:
                    reports.append((journal.with_suffix(".md"), header))
            except (OSError, ValueError):
                continue
        return reports

    def reference(self):
        if self.path is None:
            return None
        return {"invocation_id": self.id, "path": self.path.relative_to(self.primary).as_posix()}

    def merge_children(self):
        counts = {"started": 0, "completed": 0}
        for path, header in self.children():
            count = None
            observed = {"started": 0, "completed": 0}
            terminal = None
            try:
                with path.with_suffix(".jsonl").open(encoding="utf-8") as stream:
                    for line in stream:
                        row = json.loads(line)
                        if row["event"] == "counts":
                            count = {key: row.get(key, 0) for key in counts}
                        elif row["event"] == "case_started":
                            observed["started"] += 1
                        elif row["event"] == "case_finished" and row.get("outcome") != "skipped":
                            observed["completed"] += 1
                        elif row["event"] == "finished":
                            terminal = row
            except (OSError, ValueError, KeyError) as error:
                self.record_error(error)
            if count is None:
                observed["started"] = max(observed["started"], observed["completed"])
                count = observed
            if terminal is None or terminal.get("evidence") == "incomplete":
                self.record_error(f"Child case evidence is incomplete: {path.name}")
            self.event("child_report", producer=header["producer"], invocation_id=header["invocation_id"],
                       artifact=self.link(path), exit_code=terminal["exit_code"] if terminal else None, **count)
            for key, value in count.items():
                counts[key] += value
        self.event("counts", **counts)
        return counts

    def phase(self, name, **details):
        self.current_phase = name
        self.event("phase", phase=name, **details)

    def event(self, kind, **details):
        if self.journal is None:
            return
        record = {"event": kind, **details}
        try:
            native = json.dumps(record, ensure_ascii=False, allow_nan=False)
            with self.journal.open("a", encoding="utf-8", newline="\n") as stream:
                stream.write(native + "\n")
            with self.path.open("a", encoding="utf-8", newline="\n") as stream:
                fields = []
                for key, value in details.items():
                    if value is None:
                        value = "unavailable"
                    elif key == "artifact":
                        fields.append(f"[Native evidence]({quote(value, safe='/')})")
                        continue
                    elif isinstance(value, (dict, list, tuple)):
                        value = json.dumps(value, ensure_ascii=False)
                    fields.append(f"{key.replace('_', ' ')} {inline(value)}")
                stream.write(f"- **{kind.replace('_', ' ').capitalize()}**. " + ". ".join(fields) + "\n")
        except (OSError, ValueError) as error:
            self.record_error(error)

    def finish(self, exit_code):
        self.event("finished", phase=self.current_phase, exit_code=exit_code,
                   signal=-exit_code if isinstance(exit_code, int) and exit_code < 0 else None,
                   ended_utc=timestamp(utc_now()), seconds=time.perf_counter() - self.started,
                   evidence="incomplete" if self.incomplete else "recorded")


class ReportArgumentParser(argparse.ArgumentParser):
    def __init__(self, report, **options):
        super().__init__(**options)
        self.report = report

    def error(self, message):
        self.report.event("selection_error", detail=message)
        super().error(message)


def run_reported(producer, argv, run, cwd=REPO_ROOT):
    report = CaseReport.reserve(producer, argv, cwd)
    try:
        code = run(report)
    except SystemExit as error:
        report.finish(error.code)
        raise
    except KeyboardInterrupt:
        report.event("interrupted")
        report.finish(130)
        raise
    except Exception as error:
        report.event("invocation_error", phase=report.current_phase, detail=str(error),
                     native_exit_code=getattr(error, "returncode", None), stack=None)
        report.finish(1)
        raise
    report.finish(code)
    return code

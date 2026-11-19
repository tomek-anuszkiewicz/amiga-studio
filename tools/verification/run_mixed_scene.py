"""Run the mixed scene benchmark and archive its original measurement."""

import json
import re
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

from case_report import run_reported
from repository import REPO_ROOT


def find_executable(output):
    executable = None
    for line in output.splitlines():
        row = json.loads(line)
        if row.get("reason") == "compiler-message" and row["message"].get("rendered"):
            print(row["message"]["rendered"], end="", file=sys.stderr)
        target = row.get("target", {})
        if target.get("name") == "mixed_scene" and "example" in target.get("kind", []):
            if row.get("reason") == "compiler-artifact":
                executable = row["executable"]
    if executable is None:
        raise ValueError("Cargo did not report the mixed_scene executable")
    return executable


def read_metrics(output):
    # Native stdout consists of per-series console lines followed by one JSON object.
    start = output.find("\n{\n")
    if start == -1:
        return None
    value, _ = json.JSONDecoder().raw_decode(output[start + 1:])
    if value.get("mode") != "bench":
        return None
    return value


def run(arguments, archive):
    archive.phase("build", arguments=arguments)
    command = ["cargo", "build", "--release", "-p", "machine_loop", "--example", "mixed_scene",
               "--message-format=json-render-diagnostics"]
    build = subprocess.run(command, cwd=REPO_ROOT, capture_output=True, text=True, encoding="utf-8",
                           errors="replace")
    if build.stderr:
        print(build.stderr, end="", file=sys.stderr)
    build_log = archive.attachment("cargo-build.log")
    if build_log:
        try:
            build_log.write_text(build.stdout + build.stderr, encoding="utf-8")
            archive.event("build_log", artifact=archive.link(build_log))
        except OSError as error:
            archive.record_error(error)
    if build.returncode:
        return build.returncode
    command = [find_executable(build.stdout), "bench", *arguments]
    identity = dict(suite="machine_loop", id="mixed_scene", attempt=1)
    archive.phase("measurement", command=command)
    with tempfile.TemporaryDirectory(prefix="mixed-scene-") as temporary:
        output = archive.attachment("mixed-scene-execution.log")
        if output:
            try:
                output.touch(exist_ok=False)
            except OSError as error:
                archive.record_error(error)
                output = None
        log = output or Path(temporary) / "execution.log"
        if output:
            archive.event("execution_log", artifact=archive.link(output))
        info_only = "--help" in arguments
        if not info_only:
            archive.event("case_selected", **identity)
            archive.event("case_started", **identity)
        with log.open("w", encoding="utf-8") as stream:
            started = time.perf_counter()
            code = subprocess.call(command, cwd=REPO_ROOT, stdout=stream, stderr=subprocess.STDOUT)
            elapsed = time.perf_counter() - started
        with log.open(encoding="utf-8", errors="replace") as stream:
            shutil.copyfileobj(stream, sys.stdout)
        if info_only:
            return code
        text = log.read_text(encoding="utf-8", errors="replace")
        try:
            metrics = read_metrics(text)
        except ValueError as error:
            archive.record_error(error)
            metrics = None
        detail = next((line for line in text.splitlines() if line.startswith("Error:")), None)
        series = [
            {"series": int(number), "fields": int(fields), "seconds": float(seconds),
             "milliseconds_per_field": float(milliseconds), "fields_per_second": float(rate)}
            for number, fields, seconds, milliseconds, rate in re.findall(
                r"^series=(\d+) fields=(\d+) seconds=([^ ]+) ms_per_field=([^ ]+) fields_per_second=(\S+)$",
                text, re.MULTILINE)
        ]
        archive.event("case_finished", **identity, exit_code=code, seconds=elapsed,
                      outcome="failed" if code else "passed", detail=detail, stack=None,
                      artifact=archive.link(output), metric_units={"series_seconds": "seconds",
                      "milliseconds_per_field": "ms per field", "fields_per_second": "fields per second"},
                      native=metrics, displayed_series=series)
        archive.event("counts", selected=1, started=1, completed=1)
        if metrics is None and not code:
            archive.record_error("The original benchmark emitted no readable native metrics")
    return code


def main():
    argv = sys.argv[1:]
    return run_reported("Mixed scene benchmark", [sys.executable, str(Path(__file__).resolve()), *argv],
                        lambda report: run(argv, report))


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(130)
    except (OSError, ValueError) as error:
        print(f"Error: {error}", file=sys.stderr)
        sys.exit(1)

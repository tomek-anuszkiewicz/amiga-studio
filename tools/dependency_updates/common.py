"""Bound individual probes and collect their report findings."""

from collections import Counter, defaultdict
from dataclasses import dataclass, field, replace
from functools import lru_cache
import http.client
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib
from typing import Callable
from urllib.error import HTTPError
from urllib.request import Request, urlopen


class ProbeError(Exception):
    """A probe could not establish its result."""


DATA_ERRORS = (OSError, ValueError, LookupError, TypeError, AttributeError, ProbeError)
TRIPLE = re.compile(r"(?<![\w.])v?(\d+)\.(\d+)\.(\d+)(?![\w.-])")


def triplet(value):
    """Read a three-part stable version from a tool or release label."""
    match = TRIPLE.search(value)
    if not match:
        raise ProbeError("the source did not report a recognizable stable version")
    return tuple(map(int, match.groups()))


def version_text(version):
    return ".".join(map(str, version))


class Probes:
    def __init__(self, root):
        self.root = root
        self.scratch = root / ".agent/tmp"
        self.http = lru_cache(maxsize=None)(self._http)

    def run(self, command, *, timeout=8, env=None, input_text=None, cwd=None, combine=False):
        environment = {**os.environ, "RUSTUP_AUTO_INSTALL": "0", "PYTHONDONTWRITEBYTECODE": "1",
                       "GIT_OPTIONAL_LOCKS": "0"}
        for key, value in (env or {}).items():
            if value is None:
                environment.pop(key, None)
            else:
                environment[key] = value
        try:
            result = subprocess.run(
                command, cwd=cwd or self.root, env=environment, encoding="utf-8", errors="replace",
                input=input_text, stdout=subprocess.PIPE, timeout=timeout, check=True,
                stderr=subprocess.STDOUT if combine else subprocess.PIPE,
            )
        except FileNotFoundError:
            raise ProbeError(f"{Path(command[0]).name} is unavailable") from None
        except subprocess.TimeoutExpired:
            raise ProbeError(f"command timed out after {timeout} seconds") from None
        except subprocess.CalledProcessError as error:
            raise ProbeError(f"command exited with status {error.returncode}") from None
        except OSError:
            raise ProbeError("the command could not run") from None
        return result.stdout.rstrip()

    @staticmethod
    def _http(url, accept="application/json"):
        headers = {"Accept": accept, "User-Agent": "Amiga-development-dependency-check"}
        try:
            with urlopen(Request(url, headers=headers), timeout=7) as response:
                data = response.read(24 * 1024 * 1024 + 1)
                if len(data) > 24 * 1024 * 1024:
                    raise ProbeError("source response exceeded the size limit")
                return json.loads(data) if accept.endswith("json") else data.decode("utf-8")
        except HTTPError as error:
            raise ProbeError(f"source returned HTTP {error.code}") from None
        except (http.client.HTTPException, OSError, ValueError):
            raise ProbeError("source did not return readable data before the network limit") from None

    def tracked_files(self):
        blocked = {"ref_src", "reference", "node_modules", "site-packages", ".venv", "venv",
                   "target", "graphify-out", ".cache", "__pycache__", ".agent", "books",
                   "dist", "build", "amigatestkit", "m68k-asm"}
        return {
            name for name in self.run(["git", "ls-files", "-z"]).split("\0")
            if name and not name.lower().startswith(("artifacts/", "benchmarks/results/"))
            and blocked.isdisjoint(part.lower() for part in Path(name).parts[:-1])
        }

    def toml(self, path):
        return tomllib.loads((self.root / path).read_text(encoding="utf-8"))

    def json(self, path):
        return json.loads((self.root / path).read_text(encoding="utf-8"))


@dataclass(frozen=True)
class Finding:
    status: str
    item: str
    observed: str | None = None
    declared: str | None = None
    candidate: str | None = None
    source: str | None = None
    context: str | None = None
    detail: str | None = None
    within_requirements: bool = False


@dataclass
class Area:
    name: str
    progress: Callable[[str], None] | None = None
    counts: Counter = field(default_factory=Counter)
    findings: list[Finding] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)

    def announce(self, message):
        if self.progress is not None:
            self.progress(message)

    def record(self, status, item, **details):
        self.counts[status] += 1
        if status != "current":
            self.findings.append(Finding(status, item, **details))

    def incomplete(self, item, source, reason, **details):
        self.record("inconclusive", item, source=source, detail=str(reason), **details)

    def compare(self, item, observed, candidate, source, *, category="update", observed_text=None, **details):
        if candidate > observed:
            self.record(category, item, observed=observed_text or version_text(observed),
                        candidate=version_text(candidate), source=source, **details)
        else:
            self.record("current", item)


GROUPS = {
    "update": ("Updates available", "32"),
    "within requirements": ("Updates within current requirements", "32"),
    "compatibility": ("Check compatibility before updating", "33"),
    "pin review": ("Requires changing project requirements", "33"),
    "baseline difference": ("Historical version differences", "33"),
    "drift": ("Installation or lockfile differences", "33"),
    "missing": ("Missing", "31"),
    "optional missing": ("Missing optional tools or packages", "33"),
    "inconclusive": ("Could not check", "31"),
}


def supports_color(out):
    if not out.isatty() or "NO_COLOR" in os.environ or os.environ.get("TERM") == "dumb":
        return False
    if os.name != "nt":
        return True

    import ctypes
    import msvcrt

    try:
        handle = ctypes.c_void_p(msvcrt.get_osfhandle(out.fileno()))
        mode = ctypes.c_ulong()
        kernel = ctypes.windll.kernel32
        return bool(kernel.GetConsoleMode(handle, ctypes.byref(mode))
                    and kernel.SetConsoleMode(handle, mode.value | 0x0004))
    except (AttributeError, OSError, ValueError):
        return False


def report_text(value):
    text = "" if value is None else str(value)
    text = re.sub(r"\b[a-z][a-z0-9+.-]*://[^\s]+", "[source omitted]", text, flags=re.I)
    text = "".join(character if character.isprintable() else " " for character in text)
    if re.fullmatch(r"sha256:[0-9a-f]{64}", text):
        return text[7:19]
    return text


class Report:
    def __init__(self, out):
        self.out = out
        self.color = supports_color(out)
        self.partial = False
        self.has_findings = False

    def style(self, text, color):
        return f"\033[{color}m{text}\033[0m" if self.color else text

    def write(self, text):
        print(text, file=self.out, flush=True)

    def start_area(self, name):
        self.write(f"\n{self.style(f'Checking {name}', '1;36')}...")

    def write_area(self, area):
        self.partial |= bool(area.counts["inconclusive"])
        self.has_findings |= bool(area.findings)
        findings = defaultdict(set)
        for finding in area.findings:
            findings[replace(finding, context=None)].add(finding.context)
        groups = defaultdict(list)
        contexts = defaultdict(set)
        for finding, origins in findings.items():
            status = finding.status
            if finding.within_requirements:
                status = "within requirements"
            elif status in {"candidate", "review"}:
                status = "compatibility"
            groups[status].append((finding, origins))
            contexts[finding.item].update(origins)

        lines = [f"  {report_text(note)}" for note in area.notes]
        for status, (label, color) in GROUPS.items():
            if not groups[status]:
                continue
            lines.append(f"  {self.style(label, color)}")
            for finding, origins in groups[status]:
                item = report_text(finding.item)
                if origins != contexts[finding.item] and origins - {None}:
                    scope = ", ".join(report_text(origin) for origin in sorted(origins - {None}))
                    item += f" ({scope})"
                observed = report_text(finding.observed)
                candidate = report_text(finding.candidate)
                change = f"{observed} -> {candidate}" if observed and candidate else observed or candidate
                description = f"{item} {change}" if change else item
                if finding.detail:
                    description += f" ({report_text(finding.detail)})"
                lines.append(f"    {description}")
        if not findings:
            lines.append("  No findings.")
        self.write("\n".join(lines))

    def finish(self):
        if self.partial:
            message = self.style("Partial report. See the checks that could not finish above.", "33")
        elif self.has_findings:
            message = self.style("Report complete.", "32")
        else:
            message = self.style("Report complete. No updates or installation differences found.", "32")
        self.write(f"\n{message}")

"""Compare Python declarations and the selected interpreter's installed packages."""

from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from functools import lru_cache
from importlib import metadata
import json
import os
from pathlib import Path, PurePosixPath
import re
import shlex
import sys

from .common import DATA_ERRORS, ProbeError, triplet


PYTHON_RELEASES = "https://www.python.org/api/v2/downloads/release/?is_published=true&pre_release=false"
PYPI = "https://pypi.org/simple"


def packaging_api():
    try:
        from packaging.requirements import Requirement
        from packaging.utils import canonicalize_name
        from packaging.version import Version
    except ImportError:
        try:
            from pip._vendor.packaging.requirements import Requirement
            from pip._vendor.packaging.utils import canonicalize_name
            from pip._vendor.packaging.version import Version
        except ImportError:
            raise ProbeError("PEP 440 and PEP 508 helpers are unavailable in this interpreter") from None
    return Requirement, canonicalize_name, Version


def check_python_runtime(area, probe):
    info = sys.version_info
    current = info[:3]
    version = sys.version.split()[0]
    if sys.prefix != sys.base_prefix:
        prefix = Path(sys.prefix)
        name = prefix.parent.name if prefix.name in {".venv", "venv"} else prefix.name
        area.notes.append(f"Selected Python environment: virtual environment {name}.")
    else:
        area.notes.append("Selected Python environment: base environment.")
    try:
        if info.releaselevel != "final":
            raise ProbeError("the selected interpreter is a prerelease")
        records = probe.http(PYTHON_RELEASES)
        stable = [triplet(row["name"]) for row in records
                  if re.fullmatch(r"Python \d+\.\d+\.\d+", row["name"])
                  and row["pre_release"] is False and row["is_published"] is True]
        same = [candidate for candidate in stable if candidate[:2] == current[:2]]
        if not stable or not same:
            raise ProbeError("Python.org did not list this release series")
        area.compare("Python", current, max(same), "Python.org releases", detail="selected release series")
        if max(stable)[:2] > current[:2]:
            area.compare("Python", current, max(stable), "Python.org releases",
                         category="review", detail="newer release series")
    except DATA_ERRORS as error:
        reason = error if isinstance(error, ProbeError) else "invalid Python.org release data"
        area.incomplete("Python", "Python.org releases", reason, observed=version)


def pip_index(probe, name, Version):
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", name):
        raise ProbeError("invalid PyPI project name")
    # Pin the registry used for ordinary PyPI declarations and suppress pip caches.
    env = {key: None for key in os.environ if key.startswith("PIP_")}
    env["PIP_CONFIG_FILE"] = os.devnull
    command = [sys.executable, "-B", "-m", "pip", "--isolated", "--disable-pip-version-check",
               "--no-cache-dir", "index", "versions", name, "--index-url", PYPI,
               "--timeout", "5", "--retries", "0"]
    try:
        data = json.loads(probe.run([*command, "--json"], timeout=12, env=env))
        versions = data["versions"]
    except ProbeError as original:
        # Older pip releases expose the same candidates through the text command.
        try:
            output = probe.run(command, timeout=12, env=env)
        except ProbeError:
            raise original
        match = re.search(r"^Available versions:\s*(.+)$", output, re.M)
        if not match:
            raise ProbeError("pip did not report available versions")
        versions = match.group(1).split(", ")
    if not isinstance(versions, list) or not versions:
        raise ProbeError("pip returned no releases for this interpreter")
    stable = {Version(value.strip()) for value in versions}
    stable = {value for value in stable if not value.is_prerelease and not value.is_devrelease}
    if not stable:
        raise ProbeError("pip returned no stable releases for this interpreter")
    return sorted(stable, reverse=True)


@dataclass
class PythonInputs:
    roots: list
    constraints: dict
    issues: list
    public_index: bool = True


def read_requirements(probe, tracked, start, Requirement, normalize):
    result = PythonInputs([], defaultdict(list), [])

    def visit(path, constraints=False, stack=()):
        if path not in tracked or path in stack:
            result.issues.append((path, "a requirements include is untracked, excluded or cyclic"))
            return
        try:
            raw = (probe.root / path).read_text(encoding="utf-8-sig")
        except (OSError, UnicodeError):
            result.issues.append((path, "the requirements file could not be read"))
            return
        raw = re.sub(r"\\\r?\n", "", raw)
        for line in raw.splitlines():
            line = re.split(r"(^|\s+)#", line, maxsplit=1)[0].strip()
            if not line:
                continue
            include = re.fullmatch(r"(-r|--requirement|-c|--constraint)(?:\s+|=)(.+)", line)
            if include:
                kind, value = include.groups()
                try:
                    filename, = shlex.split(value)
                    target = (probe.root / PurePosixPath(path).parent / filename).resolve()
                    relative = target.relative_to(probe.root).as_posix()
                except (OSError, ValueError):
                    result.issues.append((path, "a requirements include could not be read inside the checkout"))
                else:
                    visit(relative, constraints or kind in {"-c", "--constraint"}, (*stack, path))
                continue
            if line.startswith("-"):
                result.public_index = False
                result.issues.append((path, "requirements file options need a separate source or policy check"))
                continue
            try:
                requirement = Requirement(line)
            except ValueError:
                result.issues.append((path, "a requirement uses unsupported syntax"))
                continue
            if requirement.marker and not requirement.marker.evaluate():
                continue
            if constraints:
                result.constraints[normalize(requirement.name)].append((path, requirement))
            else:
                result.roots.append((path, requirement))

    visit(start)
    return result


def candidate_rows(area, item, installed, specifiers, releases, *, context=None, declared=None):
    # Project pins supply a release baseline even when the local installation drifts.
    exact = {part.version for specifier in specifiers for part in specifier
             if part.operator == "==" and "*" not in part.version}
    pin = type(releases[0])(next(iter(exact))) if len(exact) == 1 else None
    baseline = pin if pin is not None else installed
    newer = [version for version in releases if baseline is None or version > baseline]
    if not newer:
        area.record("current", item)
        return
    allowed = next((v for v in newer if all(v in specifier for specifier in specifiers)), None)
    observed = str(installed) if installed is not None else "not installed"
    if allowed is not None:
        update = baseline is not None
        area.record("update" if update else "candidate", item, observed=observed,
                    declared=declared, candidate=str(allowed), source=PYPI, context=context,
                    within_requirements=update and declared is not None)
    latest = newer[0]
    if latest != allowed:
        use_pin = pin is not None and pin != installed
        area.record("pin review", item, observed=str(pin) if use_pin else observed, declared=declared,
                    candidate=str(latest), source=PYPI, context=context, detail="project pin" if use_pin else None)


def check_python_packages(area, probe, tracked):
    Requirement, normalize, Version = packaging_api()
    sources = sorted(path for path in tracked if PurePosixPath(path).name == "requirements.txt")
    inputs = {path: read_requirements(probe, tracked, path, Requirement, normalize) for path in sources}

    @lru_cache(maxsize=None)
    def distribution(name):
        try:
            return metadata.distribution(name)
        except metadata.PackageNotFoundError:
            return None

    observations = []
    for start, declared in inputs.items():
        owner = PurePosixPath(start).parent.name or "checkout root"
        for path, issue in declared.issues:
            area.incomplete(f"Python requirements ({owner})", "requirements files", issue, declared=path)
        root_requirements = defaultdict(list)
        for path, requirement in declared.roots:
            root_requirements[normalize(requirement.name)].append((path, requirement))
        for path, requirement in dict.fromkeys(declared.roots):
            name = normalize(requirement.name)
            context = owner
            if path != start:
                context += f", via {PurePosixPath(path).name}"
            constraints = [*declared.constraints[name],
                           *((owner, req) for owner, req in root_requirements[name]
                             if (owner, req) != (path, requirement))]
            specs = [requirement.specifier, *(req.specifier for _, req in constraints)]
            text = ", ".join([str(requirement), *(f"{req} ({owner})" for owner, req in constraints)])
            if requirement.url or any(req.url for _, req in constraints) or not declared.public_index:
                area.incomplete(name, "requirements files", "the package source needs a separate check",
                                declared=requirement.name, context=context)
                continue
            dist = distribution(name)
            try:
                installed = Version(dist.version) if dist is not None else None
            except ValueError:
                area.incomplete(name, "installed Python metadata", "invalid installed version",
                                declared=text, context=context)
                continue
            if installed is None:
                area.record("missing", name, observed="not installed", declared=text, source="selected interpreter",
                            context=context)
            elif not all(spec.contains(installed, prereleases=True) for spec in specs):
                area.record("drift", name, observed=str(installed), declared=text, source="selected interpreter",
                            context=context, detail="outside the installed workflow's requirements")
            observations.append((name, context, installed, specs, text))

    try:
        pip_version = probe.run([sys.executable, "-B", "-m", "pip", "--version"]).split()[1]
        installed_pip = Version(pip_version)
        observations.append(("pip", None, installed_pip, (), None))
    except DATA_ERRORS as error:
        area.incomplete("pip", "selected interpreter",
                        error if isinstance(error, ProbeError) else "could not parse the pip version")

    def lookup(name):
        try:
            return pip_index(probe, name, Version)
        except DATA_ERRORS as error:
            return error if isinstance(error, ProbeError) else ProbeError("invalid PyPI version response")

    names = sorted({row[0] for row in observations})
    area.announce("  Checking Python package releases on PyPI...")
    with ThreadPoolExecutor(max_workers=6) as workers:
        releases = dict(zip(names, workers.map(lookup, names)))
    for name, context, installed, specs, declared in observations:
        available = releases[name]
        if isinstance(available, ProbeError):
            area.incomplete(name, PYPI, available,
                            observed=str(installed) if installed is not None else "not installed",
                            declared=declared, context=context)
        else:
            candidate_rows(area, name, installed, specs, available, context=context,
                           declared=declared)

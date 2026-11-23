"""Read Cargo declarations, registry releases and the resolver's dry run."""

from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
import fnmatch
import json
import os
from pathlib import Path, PurePosixPath
import re
from tempfile import TemporaryDirectory

from .common import DATA_ERRORS, ProbeError


CRATES_IO = "registry+https://github.com/rust-lang/crates.io-index"
CRATES_INDEX = "https://index.crates.io"
REGISTRY_SOURCES = {CRATES_IO, "sparse+https://index.crates.io/"}
DEPENDENCY_SECTIONS = ("dependencies", "dev-dependencies", "build-dependencies")
VERSION = r"(\d+(?:\.\d+){0,2})(?:\.([*xX]))?"


def stable_crate_version(value):
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)(?:\+[-\w.]+)?", value)
    return tuple(map(int, match.groups())) if match else None


def cargo_allows(requirement, version):
    """Evaluate Cargo's stable requirement operators without considering prereleases."""
    for clause in requirement.split(","):
        if clause.strip() in {"*", "x", "X"}:
            continue
        match = re.fullmatch(rf"\s*(>=|<=|[=><~^])?\s*{VERSION}\s*", clause)
        if not match:
            raise ProbeError("Cargo requirement uses an unsupported version expression")
        operator, number, wildcard = match.groups()
        prefix = tuple(map(int, number.split(".")))
        floor = prefix + (0,) * (3 - len(prefix))
        if wildcard or operator == "=":
            allowed = version[:len(prefix)] == prefix
        elif operator in {">", "<", ">=", "<="}:
            actual = version[:len(prefix)]
            allowed = {">": actual > prefix, "<": actual < prefix,
                       ">=": actual >= prefix, "<=": actual <= prefix}[operator]
        else:
            if operator == "~":
                index = min(1, len(prefix) - 1)
            else:
                index = next((i for i, part in enumerate(prefix) if part), len(prefix) - 1)
            ceiling = prefix[:index] + (prefix[index] + 1,) + (0,) * (2 - index)
            allowed = floor <= version < ceiling
        if not allowed:
            return False
    return True


@dataclass(frozen=True)
class Declaration:
    path: str
    member: str
    name: str
    requirement: str
    source: str | None


def dependency_tables(manifest):
    for section in (manifest, *manifest.get("target", {}).values()):
        for group in DEPENDENCY_SECTIONS:
            yield section.get(group, {})


def declarations(probe, tracked):
    manifests = {name: probe.toml(name) for name in sorted(tracked)
                 if PurePosixPath(name).name == "Cargo.toml"}
    root = manifests["Cargo.toml"]
    if root.get("patch") or root.get("replace"):
        raise ProbeError("the workspace changes Cargo package sources through patches or replacements")
    workspace = root.get("workspace", {})
    inherited = workspace.get("dependencies", {})
    names = set(workspace.get("members", ()))
    excluded = workspace.get("exclude", ())
    members = {"Cargo.toml"}
    for path in manifests:
        directory = PurePosixPath(path).parent.as_posix()
        if (any(fnmatch.fnmatchcase(directory, pattern) for pattern in names)
                and not any(fnmatch.fnmatchcase(directory, pattern) for pattern in excluded)):
            members.add(path)
    # Cargo also includes path dependencies under the workspace root.
    pending = list(members)
    while pending:
        path = pending.pop()
        for group in dependency_tables(manifests[path]):
            for alias, value in group.items():
                if not isinstance(value, dict):
                    continue
                if value.get("workspace"):
                    value = inherited[alias]
                if isinstance(value, dict) and "path" in value:
                    base = "." if group[alias].get("workspace") else PurePosixPath(path).parent
                    local = (probe.root / base / value["path"] / "Cargo.toml").resolve()
                    if local.is_relative_to(probe.root):
                        relative = local.relative_to(probe.root).as_posix()
                        directory = PurePosixPath(relative).parent.as_posix()
                        if (relative in manifests and relative not in members
                                and not any(fnmatch.fnmatchcase(directory, pattern) for pattern in excluded)):
                            members.add(relative)
                            pending.append(relative)
    for path in sorted(members):
        manifest = manifests[path]
        if "package" not in manifest:
            continue
        member = manifest["package"]["name"]
        for group in dependency_tables(manifest):
            for alias, value in group.items():
                if isinstance(value, dict) and value.get("workspace"):
                    value = inherited[alias]
                if isinstance(value, str):
                    yield Declaration(path, member, alias, value, CRATES_IO)
                    continue
                package = value.get("package", alias)
                requirement = value.get("version", "unspecified")
                if "path" in value:
                    if "git" not in value and "registry" not in value:
                        continue
                    source = None
                elif "git" in value or "registry-index" in value or value.get("registry", "crates-io") != "crates-io":
                    source = None
                else:
                    source = CRATES_IO
                yield Declaration(path, member, package, requirement, source)


def registry_release(probe, name):
    name = name.lower()
    if not re.fullmatch(r"[a-z0-9_-]+", name):
        raise ProbeError("invalid crates.io package name")
    index_path = (f"{len(name)}/{name}" if len(name) < 3 else f"3/{name[0]}/{name}" if len(name) == 3
                  else f"{name[:2]}/{name[2:4]}/{name}")
    url = f"{CRATES_INDEX}/{index_path}"
    versions = [json.loads(line) for line in probe.http(url, "text/plain").splitlines()]
    if not versions or any(row["name"].lower() != name for row in versions):
        raise ProbeError("crates.io returned no matching package data")
    candidates = [stable_crate_version(row["vers"]) for row in versions if row["yanked"] is False]
    stable = [candidate for candidate in candidates if candidate is not None]
    if not stable:
        raise ProbeError("crates.io listed no stable non-yanked releases")
    return max(stable)


def locked_for(declaration, packages):
    candidates = [p for p in packages if p["name"] == declaration.name and p.get("source") in REGISTRY_SOURCES]
    if not candidates:
        return []
    parent = next((p for p in packages if p["name"] == declaration.member and "source" not in p), None)
    if parent is None:
        return []
    links = parent.get("dependencies", ())
    return [p for p in candidates if any(
        link in links for link in (p["name"], f'{p["name"]} {p["version"]}',
                                    f'{p["name"]} {p["version"]} ({p["source"]})')
    ) and ((version := stable_crate_version(p["version"])) is not None
           and cargo_allows(declaration.requirement, version))]


def cargo_configuration(probe):
    home = Path(os.environ["CARGO_HOME"]) if "CARGO_HOME" in os.environ else Path.home() / ".cargo"
    if not home.is_absolute():
        home = probe.root / home
    workspace_folders = tuple(parent / ".cargo" for parent in (*probe.root.parents, probe.root))
    home_config = False
    for folder in dict.fromkeys((home, *workspace_folders)):
        for filename in ("config", "config.toml"):
            path = folder / filename
            if path.is_file():
                conf = probe.toml(path)
                if conf.get("include"):
                    raise ProbeError("Cargo includes extra configuration files; registry identity cannot be guaranteed")
                if conf.get("source") or conf.get("registries") or conf.get("patch") or conf.get("paths"):
                    raise ProbeError("Cargo source overrides are configured; registry identity cannot be guaranteed")
                if folder == home and folder not in workspace_folders:
                    home_config = bool(conf)
                break
    if any(key.startswith(("CARGO_SOURCE_", "CARGO_REGISTRIES_", "CARGO_REGISTRY_"))
           and key != "CARGO_REGISTRY_GLOBAL_MIN_PUBLISH_AGE" for key in os.environ):
        raise ProbeError("Cargo registry environment overrides are configured")
    return home_config


def cargo_dry_run(probe, tracked):
    source_files = [probe.root / name for name in tracked if PurePosixPath(name).name in {"Cargo.toml", "Cargo.lock"}]
    before = [file.read_bytes() for file in source_files]
    probe.scratch.mkdir(parents=True, exist_ok=True)
    try:
        with TemporaryDirectory(prefix="cargo-update-", dir=probe.scratch) as scratch:
            output = probe.run(["cargo", "+stable", "update", "--dry-run", "--color", "never"],
                               timeout=60, combine=True, env={
                                   "CARGO_HOME": scratch, "CARGO_NET_RETRY": "0",
                                   "CARGO_HTTP_TIMEOUT": "10", "CARGO_TERM_COLOR": "never",
                               })
    finally:
        if [file.read_bytes() for file in source_files] != before:
            raise ProbeError("Cargo dry run modified a manifest or lockfile")
    return output


def resolved_updates(area, packages, direct, output):
    source = "cargo +stable update --dry-run"
    locked_versions = {(p["name"], p["version"]) for p in packages if p.get("source") in REGISTRY_SOURCES}
    requirements = defaultdict(set)
    for declaration, locked in direct:
        for package in locked:
            requirements[package["name"], package["version"]].add(declaration.requirement)
    resolved = set()
    changes = []
    summary = re.search(r"(?m)^\s*Locking (\d+) packages?\b", output)
    if summary is None:
        raise ProbeError("Cargo did not report a recognizable resolver summary")
    for line in output.splitlines():
        match = re.fullmatch(r"\s*(Updating|Downgrading|Removing|Adding) ([\w-]+) v(\S+)"
                             r"(?: -> v(\S+))?(?: .*)?", line)
        if match:
            operation, name, old, candidate = match.groups()
            if operation in {"Updating", "Downgrading"} and not candidate:
                raise ProbeError("Cargo returned an unrecognized package change")
            if operation != "Adding" and (name, old) not in locked_versions:
                raise ProbeError("Cargo reported a package change without a matching registry lock entry")
            changes.append((operation, name, old, candidate))
        elif re.match(r"\s*(?:Updating|Downgrading|Removing|Adding) \S+ v", line):
            raise ProbeError("Cargo returned an unrecognized package change")
    if sum(operation != "Removing" for operation, *_ in changes) != int(summary[1]):
        raise ProbeError("Cargo's resolver summary and package changes did not match")
    for operation, name, old, candidate in changes:
        key = name, old
        if operation == "Adding" or key not in requirements:
            continue
        resolved.add(key)
        area.record("update", name, observed=old, declared=", ".join(sorted(requirements[key])),
                    candidate=candidate or "removed", source=source, within_requirements=True)
    for name, version in requirements.keys() - resolved:
        area.record("current", f"{name} {version}")


def check_crates(area, probe, tracked):
    if not {"Cargo.toml", "Cargo.lock"} <= tracked:
        raise ProbeError("the tracked workspace manifest or lockfile is unavailable")
    packages = probe.toml("Cargo.lock")["package"]
    direct = list(dict.fromkeys(declarations(probe, tracked)))
    custom_source = (any(p.get("source") and p["source"] not in REGISTRY_SOURCES for p in packages)
                     or any(decl.source not in REGISTRY_SOURCES for decl in direct))
    home_config = cargo_configuration(probe)

    direct_locks = []
    query_names = sorted({decl.name for decl in direct if decl.source in REGISTRY_SOURCES})
    area.announce("  Checking direct crates on crates.io...")

    def lookup(name):
        try:
            return registry_release(probe, name)
        except DATA_ERRORS as error:
            return error if isinstance(error, ProbeError) else ProbeError("invalid crates.io index data")

    with ThreadPoolExecutor(max_workers=6) as workers:
        releases = dict(zip(query_names, workers.map(lookup, query_names)))
    for decl in direct:
        item = decl.name
        context = decl.member
        if decl.source not in REGISTRY_SOURCES:
            area.incomplete(item, "Cargo manifest", "check the declared source separately",
                            declared=decl.requirement, context=context)
            continue
        try:
            locked = locked_for(decl, packages)
            if not locked:
                raise ProbeError("no matching stable registry version could be tied to this declaration")
            direct_locks.append((decl, locked))
            latest = releases[decl.name]
            if isinstance(latest, ProbeError):
                raise latest
            for row in locked:
                observed = stable_crate_version(row["version"])
                if cargo_allows(decl.requirement, latest):
                    area.record("current", item)
                else:
                    area.compare(item, observed, latest, CRATES_INDEX, declared=decl.requirement,
                                 category="pin review", context=context)
        except DATA_ERRORS as error:
            area.incomplete(item, CRATES_INDEX,
                            error if isinstance(error, ProbeError) else "invalid lockfile or crate version data",
                            declared=decl.requirement, context=context)
    try:
        if custom_source:
            raise ProbeError("the workspace uses sources outside the supported registry")
        if home_config:
            raise ProbeError("the isolated cache would omit Cargo home configuration")
        area.announce("  Checking what Cargo can resolve with current manifests...")
        resolved_updates(area, packages, direct_locks, cargo_dry_run(probe, tracked))
    except DATA_ERRORS as error:
        area.incomplete("resolved updates", "cargo update --dry-run",
                        error if isinstance(error, ProbeError) else "invalid Cargo inputs or resolver data")

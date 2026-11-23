"""Compare Node release lines and npm lock entries using npm's semver rules."""

from collections import defaultdict
from datetime import date, datetime, timezone
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import sys
from tempfile import TemporaryDirectory
from urllib.parse import quote, urlsplit

from .common import DATA_ERRORS, ProbeError, triplet, version_text


NODE_INDEX = "https://nodejs.org/dist/index.json"
NODE_SCHEDULE = "https://raw.githubusercontent.com/nodejs/Release/main/schedule.json"
NPM_REGISTRY = "https://registry.npmjs.org"
NPM_DEPENDENCIES = ("dependencies", "devDependencies", "optionalDependencies", "peerDependencies")


def check_node(area, probe, node):
    current = triplet(node)
    index = probe.http(NODE_INDEX)
    schedule = probe.http(NODE_SCHEDULE)
    entry = schedule[f"v{current[0]}"]
    today = datetime.now(timezone.utc).date()
    line = [triplet(row["version"]) for row in index
            if row["version"].startswith(f"v{current[0]}.") and "-" not in row["version"]]
    if not line:
        raise ProbeError("the installed Node.js line is absent from the release index")
    area.compare("Node.js", current, max(line), NODE_INDEX, detail="installed release line")
    if date.fromisoformat(entry["end"]) <= today:
        area.record("review", "Node.js", observed=version_text(current), source=NODE_SCHEDULE,
                    detail="installed release line is no longer supported")
    lines = {major for major, dates in schedule.items() if dates.get("lts") and
             date.fromisoformat(dates["lts"]) <= today < date.fromisoformat(dates["end"])}
    newer_lts = defaultdict(list)
    for row in index:
        major = row["version"].split(".")[0]
        if major in lines and row["lts"] and int(major[1:]) > current[0]:
            newer_lts[major].append(triplet(row["version"]))
    if not newer_lts:
        area.record("current", "newer supported Node.js LTS lines")
    for major, candidates in sorted(newer_lts.items(), key=lambda item: int(item[0][1:])):
        area.compare("Node.js", current, max(candidates), NODE_INDEX,
                     category="review", detail=f"supported LTS line {major}")


def installed_npm_cli(probe, location, version):
    cli = location / "bin/npm-cli.js"
    if cli.is_file():
        try:
            package = probe.json(location / "package.json")
            if package["name"] == "npm" and package["version"] == version:
                return cli.resolve()
        except DATA_ERRORS:
            pass
    return None


def npm_runtime(probe):
    launcher = shutil.which("npm.cmd" if sys.platform == "win32" else "npm")
    if not launcher:
        raise ProbeError("npm is unavailable")
    executable = Path(launcher)
    probe.scratch.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory(prefix="npm-version-", dir=probe.scratch) as scratch:
        settings = {
            "npm_config_cache": scratch, "npm_config_logs_dir": scratch, "npm_config_logs_max": "0",
            "npm_config_timing": "false", "npm_config_update_notifier": "false", "npm_config_offline": "true",
            "NODE_OPTIONS": None, "NODE_PATH": None,
        }
        # npm accepts case variants of its environment settings.
        keys = {key.lower() for key in settings}
        env = {key: None for key in os.environ if key.lower() in keys}
        env.update(settings)
        # With no version argument this lists versions without updating the package.
        versions = json.loads(probe.run([launcher, "version", "--json"], env=env))
        installed, runtime = versions["npm"], versions["node"]
        adjacent = executable.parent / "node.exe"
        node = str(adjacent) if sys.platform == "win32" and adjacent.is_file() else "node"
        cli = None
        try:
            global_root = Path(probe.run([launcher, "root", "--global"], env=env))
            if global_root.is_absolute():
                cli = installed_npm_cli(probe, global_root / "npm", installed)
        except ProbeError:
            pass
        if cli is None:
            # Some launchers use a bundled or system npm outside the global package directory.
            candidates = (executable.resolve().parent.parent, executable.parent / "node_modules/npm",
                          executable.parent.parent / "lib/node_modules/npm",
                          executable.parent.parent / "share/nodejs/npm")
            matches = {path for candidate in candidates if (path := installed_npm_cli(probe, candidate, installed))}
            if len(matches) == 1:
                cli = matches.pop()
        return installed, runtime, node, str(cli) if cli else None


SEMVER_QUERY = """
const fs = require("node:fs");
const { createRequire } = require("node:module");
const semver = createRequire(process.argv[1])("semver");
const rows = JSON.parse(fs.readFileSync(0, "utf8"));
const results = rows.map(({ version, ranges, candidates, engine }) => {
    if (!semver.valid(version) || ranges.some(range => semver.validRange(range) === null)) {
        return null;
    }
    const releases = candidates.filter(([candidate]) => semver.valid(candidate) &&
        !semver.prerelease(candidate) && semver.gt(candidate, version));
    releases.sort(([left], [right]) => semver.rcompare(left, right));
    const latest = releases[0]?.[0] ?? null;
    const allowed = releases.find(([candidate, requires]) =>
        ranges.every(range => semver.satisfies(candidate, range)) &&
        (!engine || semver.satisfies(engine, requires ?? "*")))?.[0] ?? null;
    return { latest, allowed, matches: ranges.every(range => semver.satisfies(version, range)) };
});
process.stdout.write(JSON.stringify(results));
"""


def npm_metadata(probe, name):
    if not re.fullmatch(r"(?:@[a-zA-Z0-9_.-]+/)?[a-zA-Z0-9_.-]+", name):
        raise ProbeError("invalid npm registry package name")
    data = probe.http(f"{NPM_REGISTRY}/{quote(name, safe='')}", "application/vnd.npm.install-v1+json")
    if data["name"] != name or not data["versions"]:
        raise ProbeError("npm did not report matching package releases")
    return [(version, entry.get("engines", {}).get("node"))
            for version, entry in data["versions"].items()]


def declared_dependencies(manifest):
    requirements = defaultdict(set)
    required = set()
    for group in NPM_DEPENDENCIES:
        declarations = manifest.get(group, {})
        for name, requirement in declarations.items():
            requirements[name].add(requirement)
        if group != "optionalDependencies":
            required.update(declarations)
    return requirements, required


def check_node_packages(area, probe, tracked):
    node_version = None
    try:
        node = probe.run(["node", "--version"], env={"NODE_OPTIONS": None, "NODE_PATH": None})
        node_version = version_text(triplet(node))
        check_node(area, probe, node)
    except DATA_ERRORS as error:
        area.incomplete("Node.js", NODE_INDEX,
                        error if isinstance(error, ProbeError) else "invalid Node.js release data",
                        observed=node_version)
    rows = []

    def collect(name, observed, ranges, *, context=None, engine=None, declaration=None):
        try:
            candidates = npm_metadata(probe, name)
            rows.append((name, observed, declaration, context, {
                "version": observed, "ranges": ranges, "candidates": candidates, "engine": engine,
            }))
        except DATA_ERRORS as error:
            area.incomplete(name, NPM_REGISTRY,
                            error if isinstance(error, ProbeError) else "invalid npm registry response",
                            observed=observed, declared=declaration, context=context)

    npm_node = npm_cli = None
    try:
        installed, runtime, npm_node, npm_cli = npm_runtime(probe)
        if runtime != node_version:
            area.notes.append(f"npm uses Node.js {runtime}.")
        collect("npm", installed, [], engine=runtime)
    except DATA_ERRORS as error:
        area.incomplete("npm", "resolved npm launcher",
                        error if isinstance(error, ProbeError) else "invalid npm version or runtime data")

    packages_paths = sorted(path for path in tracked if PurePosixPath(path).name == "package.json")
    for path in packages_paths:
        manifest_path = PurePosixPath(path)
        owner = manifest_path.parent.name or "checkout root"
        lockfile = str(manifest_path.with_name("package-lock.json"))
        try:
            if lockfile not in tracked:
                raise ProbeError("no tracked package-lock.json accompanies the manifest")
            packages = probe.json(lockfile)["packages"]
            dependencies, required = declared_dependencies(probe.json(path))
            for name, requirements in sorted(dependencies.items()):
                data = packages.get(f"node_modules/{name}")
                if data is None:
                    if name in required:
                        area.incomplete(name, lockfile, "a declared dependency is missing from the lockfile", context=owner)
                    continue
                name = data.get("name") or name
                resolved = urlsplit(data.get("resolved", ""))
                ranges = sorted(requirements)
                if (data.get("link") or resolved.scheme != "https" or resolved.netloc != "registry.npmjs.org"
                        or any(value.startswith(("npm:", "file:", "git:")) for value in ranges)):
                    area.incomplete(name, lockfile, "check the declared package source separately", context=owner)
                    continue
                collect(name, data["version"], ranges, context=owner, declaration=", ".join(ranges))
        except DATA_ERRORS as error:
            area.incomplete(f"npm packages ({owner})", lockfile,
                            error if isinstance(error, ProbeError) else "invalid npm manifest or lockfile")
    if not rows:
        return
    try:
        if not npm_cli or not npm_node:
            raise ProbeError("could not identify the installed npm package matching the resolved launcher")
        output = probe.run([npm_node, "-e", SEMVER_QUERY, npm_cli],
                           input_text=json.dumps([row[-1] for row in rows]), timeout=12,
                           env={"NODE_OPTIONS": None, "NODE_PATH": None})
        results = json.loads(output)
        if not isinstance(results, list) or len(results) != len(rows):
            raise ProbeError("npm semver did not return a result for each checked package")
    except DATA_ERRORS as error:
        for item, observed, declared, context, _ in rows:
            area.incomplete(item, NPM_REGISTRY,
                            error if isinstance(error, ProbeError) else "invalid npm semver result",
                            observed=observed, declared=declared, context=context)
        return
    for (item, observed, declared, context, query), result in zip(rows, results):
        if result is None:
            area.incomplete(item, NPM_REGISTRY, "npm semver could not interpret the version or range",
                            observed=observed, declared=declared, context=context)
            continue
        if not result["matches"]:
            area.record("drift", item, observed=observed, declared=declared, context=context,
                        source="package.json and package-lock.json", detail="locked version does not match requirements")
        if result["allowed"]:
            area.record("update" if query["engine"] else "candidate", item, observed=observed, declared=declared,
                        candidate=result["allowed"], source=NPM_REGISTRY, context=context)
        if result["latest"] and result["latest"] != result["allowed"]:
            area.record("pin review" if not query["engine"] else "review", item, observed=observed, declared=declared,
                        candidate=result["latest"], source=NPM_REGISTRY, context=context,
                        detail="requires a different Node.js version" if query["engine"] else None)
        if not result["latest"]:
            area.record("current", item)

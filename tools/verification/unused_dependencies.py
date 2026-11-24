"""Read Cargo's manifest diagnostics before nextest starts running tests."""

import re
from pathlib import Path

from impact import workspace_packages


COLOR = re.compile(r"\x1b\[[0-9;]*m")
HEADER = re.compile(r"(?:warning|error): unused dependency `([^`]+)`")
LOCATION = re.compile(r"\s*--> (.+Cargo\.toml|Cargo\.toml):([1-9]\d*):[1-9]\d*")
BUILD_END = re.compile(r"\s*Finished `[^`]+` profile .* target\(s\) in .+")
NEXTEST_START = re.compile(r"\s*(?:Nextest run ID |Starting \d+ tests? across )")


class UnusedDependencies:
    def __init__(self, coverage):
        self.coverage = coverage
        self.build = True
        self.dependency = None
        self.locations = []

    def consume(self, line):
        if not self.build:
            return
        line = COLOR.sub("", line.rstrip())
        if self.coverage and (BUILD_END.fullmatch(line) or NEXTEST_START.match(line)):
            self.build = False
            return
        # Each Cargo diagnostic puts its primary manifest span after its title.
        if self.dependency and (location := LOCATION.fullmatch(line)):
            self.locations.append((self.dependency, location[1], int(location[2])))
        header = HEADER.fullmatch(line)
        self.dependency = header[1] if header else None

    def findings(self, root, command):
        if not self.locations:
            return []
        root = Path(root).resolve()
        selected = {argument for option, argument in zip(command, command[1:]) if option == "-p"}
        excluded = {argument for option, argument in zip(command, command[1:]) if option == "--exclude"}
        packages, _ = workspace_packages(root)
        manifests = {Path(package["manifest_path"]).resolve() for package in packages
                     if (not selected or package["name"] in selected) and package["name"] not in excluded}
        findings = {}
        for name, location, line in self.locations:
            path = (root / location.replace("\\", "/")).resolve()
            if path in manifests:
                findings[(name, path.relative_to(root).as_posix(), line)] = None
        return [f"`{name}` at {path}:{line}" for name, path, line in findings]

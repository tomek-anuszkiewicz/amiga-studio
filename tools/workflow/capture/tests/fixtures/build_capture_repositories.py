"""Rebuild the disposable capture repositories using native Git."""

import json
import os
from pathlib import Path
import re
import sys
import unittest
from unittest.mock import patch
from zipfile import ZipFile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from capture_fixtures import (
    ARCHIVE, NO_SERVICE_CASES, REPOSITORY, SERVICE, TESTS, VERIFICATION_TESTS,
    fixture_git_environment, json_bytes, marker, normalized_bytes, service_sources, sha256,
)
from build_record_inputs import ENVIRONMENT_CASES, PRESERVATION_CASES, RECORDS_CASES, environment, preservation, records
from build_startup_inputs import CONTROL_ROLLOUTS, IGNORE, PROOF_CASES, SERVICE_CASES, control, proof, service

sys.path.insert(0, str(VERIFICATION_TESTS / "fixtures"))
from build_repositories import CONFIG, ENVIRONMENT, git, init, repository_files, temporary_root, tracked_index, write, write_entry


def unborn(ignore):
    with temporary_root() as root:
        git(root, "init", "-q", "--template=", "-b", "main")
        write(root, ".git/config", CONFIG)
        write(root, ".gitignore", ignore)
        return {path: (root / path).read_bytes() for path in (".git/HEAD", ".git/config", ".gitignore")}


def linked(ignore, branch, message):
    with temporary_root() as parent:
        primary = parent / "primary"
        task = parent / "task"
        init(primary, {".gitignore": ignore}, branch="master", message=message)
        base = git(primary, "rev-parse", "HEAD")
        git(primary, "worktree", "add", "-q", "-b", branch, str(task))
        files = dict(repository_files(parent, primary, (task,)))
        files["fixture.json"] = (json.dumps({"base": base}, indent=2, sort_keys=True) + "\n").encode()
        return files, base


class CaseTree:
    def __init__(self, files=(), *, base=None, sources=None, linked=False):
        self.files = dict(files)
        self.directories = set()
        self.paths = {}
        self.bindings = {}
        self.native_newlines = set()
        self.revisions = {}
        self.stage = []
        self.history = None
        self.base = base
        self.sources = sources
        self.linked = linked

    def file(self, path, data, *, bind=False, native_newlines=False):
        self.files[path] = data
        if bind:
            self.bindings[path] = sorted({field.decode() for field in re.findall(rb"@fixture:([a-z0-9-]+)@", data)})
        if native_newlines:
            self.native_newlines.add(path)
        else:
            self.native_newlines.discard(path)

    def directory(self, path):
        self.directories.add(path)

    def path(self, name, path):
        if self.paths.setdefault(name, path) != path:
            raise ValueError(f"Conflicting fixture path for {name}: {path}")
        return marker(name)

    def record(self, path, value, *, native_newlines=False):
        # Keep the ZIP portable when the original writer opened the file in text mode.
        self.file(path, json_bytes(value), bind=True, native_newlines=native_newlines)

    def lines(self, path, values, *, final_newline=True):
        data = b"\n".join(json.dumps(value).encode() for value in values)
        if final_newline:
            data += b"\n"
        self.file(path, data, bind=True)
        return data

    def revision(self, name, settings, contents, *, materialize=True):
        code = name if materialize else "current"
        self.revisions[name] = {"settings": settings, "code": code}
        if materialize:
            for path, data in contents.items():
                self.file(f"{SERVICE}/compose/{marker(name)}/{path}", data)

    def git_index(self):
        if not self.stage:
            return
        with temporary_root() as root:
            primary = root / "sessions"
            staged = [path.removeprefix("sessions/") for path in self.stage]
            for path in (".git/HEAD", ".git/config", ".gitignore", *staged):
                destination = primary / path
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(self.files[f"sessions/{path}"])
            for path in (".git/objects", ".git/refs"):
                (primary / path).mkdir()
            git(primary, "add", "-f", "--", *staged)
            self.file("sessions/.git/index", tracked_index(primary))
            for path in (primary / ".git/objects").glob("??/*"):
                self.file(f"sessions/{path.relative_to(primary).as_posix()}", path.read_bytes())

    def save(self, archive, key, blobs):
        self.git_index()
        files = {}
        for path, data in sorted(self.files.items()):
            digest = sha256(data)
            if digest not in blobs:
                write_entry(archive, f"blobs/{digest}", data)
                blobs.add(digest)
            files[path] = digest
        description = {"files": files, "directories": sorted(self.directories)}
        if self.paths:
            description["paths"] = self.paths
        bindings = {path: fields for path, fields in self.bindings.items() if fields}
        if bindings:
            description["bindings"] = bindings
        if self.native_newlines:
            description["native_newlines"] = sorted(self.native_newlines)
        if self.revisions:
            description["revisions"] = self.revisions
        if self.sources:
            description["sources"] = self.sources
        if self.linked:
            description["backlinks"] = ["task"]
        if self.history:
            description["history"] = self.history
        write_entry(archive, f"cases/{key}.json", json_bytes(description))


def input_cases():
    suites = unittest.defaultTestLoader.discover(str(TESTS), pattern="test_*.py")
    recipes = {
        "ArchiveProofTests": PROOF_CASES, "ServicePathsTests": SERVICE_CASES, "RegistrationControlTests": CONTROL_ROLLOUTS,
        "EnvironmentInventoryTests": ENVIRONMENT_CASES, "WholeFilePreservationTests": PRESERVATION_CASES,
        "PreservationTests": RECORDS_CASES,
    }
    found = set()
    for module in suites:
        for group in module:
            for test in group:
                _, class_name, method = test.id().split(".")
                if class_name not in recipes or method in NO_SERVICE_CASES:
                    continue
                matches = [prefix for prefix in recipes[class_name] if method.startswith(f"test_{prefix}_")]
                if len(matches) != 1 or (class_name, matches[0]) in found:
                    raise ValueError(f"Expected one unique input recipe for {test.id()}: {matches}")
                found.add((class_name, matches[0]))
                yield test.id(), class_name, matches[0]
    missing = {(name, prefix) for name, methods in recipes.items() for prefix in methods} - found
    if missing:
        raise ValueError(f"Missing capture tests for input recipes: {sorted(missing)}")


def prepared_tree(class_name, case, templates, bases, contents):
    if class_name == "EnvironmentInventoryTests":
        tree = CaseTree()
        environment(case, tree)
    elif class_name == "ArchiveProofTests":
        tree = CaseTree({**templates["proof"], ".gitignore": IGNORE.encode()})
        for directory in (".git/objects", ".git/refs"):
            tree.directory(directory)
        proof(case, tree)
    elif class_name in ("ServicePathsTests", "RegistrationControlTests"):
        tree = CaseTree({f"sessions/{path}": data for path, data in templates["service"].items()})
        for directory in ("sessions/.git/objects", "sessions/.git/refs"):
            tree.directory(directory)
        (service if class_name == "ServicePathsTests" else control)(case, tree, contents)
        if tree.revisions:
            tree.sources = "service"
    elif class_name == "WholeFilePreservationTests":
        tree = CaseTree(templates["preservation"], linked=True)
        preservation(case, tree)
    else:
        tree = (CaseTree() if case == "removed_collection_command" else
                CaseTree(templates["records"], base=bases["records"], sources="records", linked=True))
        records(case, tree)
    return tree


def build(output):
    with patch.dict(os.environ, {**fixture_git_environment(), **ENVIRONMENT}, clear=True), ZipFile(output, "w") as archive:
        ignore = normalized_bytes(REPOSITORY / ".gitignore")
        contents = service_sources()
        records_files, records_base = linked(ignore.decode(), "task", "Starting commit")
        preservation_files, _ = linked(".agent/\n", "trial", "Create a disposable preservation trial")
        templates = {"proof": unborn(".agent/task-archives/\n"), "service": unborn(IGNORE),
                     "records": records_files, "preservation": preservation_files}
        for name, files in templates.items():
            entries = sorted(files.items()) if name in ("records", "preservation") else files.items()
            for path, content in entries:
                write_entry(archive, f"{name}/{path}", content)

        blobs = set()
        cases = {}
        for index, (case_id, class_name, case) in enumerate(sorted(input_cases())):
            key = f"{index:02d}"
            tree = prepared_tree(class_name, case, templates, {"records": records_base}, contents)
            tree.save(archive, key, blobs)
            cases[case_id] = key
        write_entry(archive, "cases.json", json_bytes({
            "cases": cases,
            "sources": {
                "service": {name: sha256(data) for name, data in contents.items()},
                "records": {".gitignore": sha256(ignore)},
            },
        }))


if __name__ == "__main__":
    build(Path(sys.argv[1]) if len(sys.argv) > 1 else ARCHIVE)

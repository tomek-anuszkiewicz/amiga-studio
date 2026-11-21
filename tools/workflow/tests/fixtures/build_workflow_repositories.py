"""Rebuild the workflow Git starting states using native Git commands."""

import os
from pathlib import Path
from shutil import copytree
import sys
from unittest.mock import patch
from zipfile import ZipFile

TESTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TESTS))
sys.path.insert(0, str(TESTS.parents[1] / "verification/tests/fixtures"))
from build_repositories import (CONFIG, ENVIRONMENT, commit, git, init, repository_files,
                                save, temporary_root, write, write_entry)
from fixture_repository import ARCHIVE


def overlay(archive, name, before, after):
    for path, content in sorted(after.items()):
        if before.get(path) != content:
            write_entry(archive, f"{name}/{path}", content)


def empty_repository(archive, name, parent, *, bare=False):
    root = parent / name
    options = ("--bare",) if bare else ()
    git(parent, "init", "--template=", "--initial-branch=master", *options, str(root))
    database = root if bare else root / ".git"
    write(database, "config", CONFIG.replace("bare = false", "bare = true") if bare else CONFIG)
    prefix = name if bare else name + "/.git"
    for path in ("HEAD", "config"):
        write_entry(archive, f"{prefix}/{path}", (database / path).read_bytes())


def delivery(archive):
    with temporary_root() as parent:
        root = parent / "primary"
        init(root, {".gitignore": ".agent/\n", "shared.txt": "baseline\n"},
             branch="master", message="Create baseline")
        base = git(root, "rev-parse", "HEAD")
        save(archive, "delivery", parent, root, metadata={"base": base})
        starting = dict(repository_files(parent, root, ()))

        for task in ("ready", "draft", "paused", "dirty", "moved", "stale", "amended", "original", "other",
                     "changed", "reverted", "same-tree", "working", "top", "foundation", "task",
                     "first", "second", "different"):
            with temporary_root() as fork:
                primary = copytree(root, fork / "primary")
                checkout = fork / f"primary-{task}"
                git(primary, "worktree", "add", "-b", task, str(checkout), base)
                write(checkout, task + ".txt", task + "\n")
                commit(checkout, "Complete " + task)
                regular = dict(repository_files(fork, primary, (checkout,)))
                overlay(archive, "source-" + task, starting, regular)
                if task in ("first", "second"):
                    write(checkout, task + ".txt", "primary\n")
                    commit(checkout, "Assign an exclusive channel")
                    overlay(archive, "channel-" + task, regular, dict(repository_files(fork, primary, (checkout,))))
        empty_repository(archive, "bare", parent, bare=True)


def routing(archive):
    for name in ("routing", "routing-native-diff", "routing-historical"):
        with temporary_root() as root:
            init(root, {"source.txt": "baseline\n", ".gitignore": ".agent/\n"}, message="Baseline")
            if name == "routing-native-diff":
                write(root, "old name.txt", "Native rename source\n")
                (root / "table.bin").write_bytes(bytes(range(256)))
                commit(root, "Add native diff fixtures")
            if name == "routing-historical":
                git(root, "config", "--local", "core.autocrlf", "true")
                (root / "rule.txt").write_bytes(b"Original recorded criterion\r\n")
                commit(root, "Record a separate baseline criterion")
            save(archive, name, root, root, metadata={"base": git(root, "rev-parse", "HEAD")})
    with temporary_root() as root:
        empty_repository(archive, "command", root)


def build(output):
    environment = {name: value for name, value in os.environ.items() if not name.upper().startswith("GIT_")}
    with patch.dict(os.environ, {**environment, **ENVIRONMENT}, clear=True), ZipFile(output, "w") as archive:
        delivery(archive)
        routing(archive)


if __name__ == "__main__":
    build(Path(sys.argv[1]) if len(sys.argv) > 1 else ARCHIVE)

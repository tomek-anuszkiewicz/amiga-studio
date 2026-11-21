"""Rebuild the disposable capture repositories using native Git."""

import os
from pathlib import Path
import sys
from unittest.mock import patch
from zipfile import ZipFile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from capture_fixtures import ARCHIVE, VERIFICATION_TESTS, fixture_git_environment

sys.path.insert(0, str(VERIFICATION_TESTS / "fixtures"))
from build_repositories import CONFIG, ENVIRONMENT, git, init, save, temporary_root, write, write_entry


def save_unborn(archive, name, ignore):
    with temporary_root() as root:
        git(root, "init", "-q", "--template=", "-b", "main")
        write(root, ".git/config", CONFIG)
        write(root, ".gitignore", ignore)
        for path in (".git/HEAD", ".git/config", ".gitignore"):
            write_entry(archive, f"{name}/{path}", (root / path).read_bytes())


def save_linked(archive, name, ignore, branch, message):
    with temporary_root() as parent:
        primary = parent / "primary"
        task = parent / "task"
        init(primary, {".gitignore": ignore}, branch="master", message=message)
        base = git(primary, "rev-parse", "HEAD")
        git(primary, "worktree", "add", "-q", "-b", branch, str(task))
        save(archive, name, parent, primary, (task,), metadata={"base": base})


def build(output):
    with patch.dict(os.environ, {**fixture_git_environment(), **ENVIRONMENT}, clear=True), ZipFile(output, "w") as archive:
        save_unborn(archive, "proof", ".agent/task-archives/\n")
        save_unborn(archive, "service", ".agent/task-archives/\n.agent/tools/\n")
        ignore = ARCHIVE.parents[5] / ".gitignore"
        save_linked(archive, "records", ignore.read_text(encoding="utf-8"), "task", "Starting commit")
        save_linked(archive, "preservation", ".agent/\n", "trial", "Create a disposable preservation trial")


if __name__ == "__main__":
    build(Path(sys.argv[1]) if len(sys.argv) > 1 else ARCHIVE)

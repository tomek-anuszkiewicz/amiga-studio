"""Restore the initial Git state for one capture test."""

import os
from pathlib import Path
import sys


TESTS = Path(__file__).resolve().parent
VERIFICATION_TESTS = TESTS.parents[2] / "verification/tests"
sys.path.insert(0, str(VERIFICATION_TESTS))
from git_fixtures import restore_repository


ARCHIVE = TESTS / "fixtures/capture-repositories.zip"


def fixture_git_environment():
    environment = {key: value for key, value in os.environ.items() if not key.upper().startswith("GIT_")}
    return {**environment, "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull}


def restore_capture(name, destination):
    restore_repository(name, destination, ARCHIVE)
    if name in ("proof", "service"):
        # The unborn repositories start with empty object and ref directories.
        (destination / ".git/objects").mkdir()
        (destination / ".git/refs").mkdir()
    if name == "proof":
        with (destination / ".gitignore").open("a", encoding="utf-8") as stream:
            stream.write(".agent/tools/\n")

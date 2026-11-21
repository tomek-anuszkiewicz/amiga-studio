"""Restore portable Git states for the workflow tests."""

import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "verification/tests"))
from git_fixtures import restore_repository


ARCHIVE = Path(__file__).with_name("fixtures") / "workflow-repositories.zip"


def restore(name, destination):
    restore_repository(name, destination, ARCHIVE)


def baseline(name, destination):
    restore(name, destination)
    metadata = destination / "fixture.json"
    base = json.loads(metadata.read_bytes())["base"]
    metadata.unlink()
    return base


def empty_repository(name, destination, *, bare=False):
    restore(name, destination)
    database = destination if bare else destination / ".git"
    (database / "objects").mkdir()
    (database / "refs").mkdir()

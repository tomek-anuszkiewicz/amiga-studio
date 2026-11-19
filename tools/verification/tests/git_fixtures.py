"""Restore disposable Git fixtures in each test's own temporary directory."""

from pathlib import Path
from zipfile import ZipFile


ARCHIVE = Path(__file__).with_name("fixtures") / "repositories.zip"


def restore_repository(name, destination):
    prefix = name + "/"
    with ZipFile(ARCHIVE) as archive:
        members = [entry for entry in archive.infolist() if entry.filename.startswith(prefix)]
        if not members:
            raise ValueError(f"Unknown Git fixture: {name}")
        for entry in members:
            path = destination / entry.filename.removeprefix(prefix)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(archive.read(entry))

    # Git versions without relative worktree links need an absolute backlink.
    for admin in (destination / "primary/.git/worktrees").glob("*"):
        backlink = (destination / admin.name / ".git").as_posix()
        (admin / "gitdir").write_text(backlink + "\n", encoding="utf-8", newline="\n")

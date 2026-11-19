"""Rebuild partial-cleanup.zip by running this file with Python from any directory."""

import json
import os
from pathlib import Path
import sys
from unittest.mock import patch
from zipfile import ZipFile

sys.path[:0] = [str(directory) for directory in Path(__file__).resolve().parents[:2]]
from build_repositories import write_entry
from test_integration import IntegrationTests, integration, repository


def build(archive):
    fixture = IntegrationTests()
    try:
        # Restore the checkout's current removal tool when the test runs.
        with patch.object(fixture, "install_worktree_tool"):
            fixture.setUp()
        sources = fixture.arrange_sources()
        commit = fixture.prepare(sources=sources)
        integration.publish(fixture.source, fixture.record)
        published = integration.read(fixture.record)
        assert published == integration.read(integration.cleanup_record(published))
        assert published["state"] == "published" and published["commit"] == commit
        assert fixture.git(fixture.target, "rev-parse", "HEAD") == commit
        later = fixture.write(Path(sources[1]["root"]), "later.txt", "Preserve later work\n")

        # Cleanup consumes these publication fields. Verification logs and
        # machine-specific review and evidence paths belong to the producer.
        record = {key: published[key] for key in ("version", "state", "base", "commit", "source_ref", "target_ref")}
        record["sources"] = [{**item, "root": Path(item["root"]).name} for item in published["sources"]]
        files = {"published.json": (json.dumps(record, indent=2) + "\n").encode(),
                 later.relative_to(fixture.parent).as_posix(): b"Preserve later work\n"}
        common = fixture.target / ".git"
        files["primary/.git/config"] = (
            b"[core]\n"
            b"\t" b"repositoryformatversion = 0\n"
            b"\t" b"filemode = false\n"
            b"\t" b"bare = false\n"
        )
        files["primary/.git/info/exclude"] = b"/tools/worktree.ps1\n"
        for path in [*common.glob("objects/??/*"), *(common / "refs/heads").iterdir()]:
            files[path.relative_to(fixture.parent).as_posix()] = path.read_bytes()
        for root in [fixture.target, *(Path(item["root"]) for item in sources)]:
            admin = common if root == fixture.target else common / "worktrees" / root.name
            if root != fixture.target:
                files[f"{root.name}/.git"] = f"gitdir: ../primary/.git/worktrees/{root.name}\n".encode()
                files[f"primary/.git/worktrees/{root.name}/commondir"] = b"../..\n"
            # An index rebuilt from its tree contains no host file timestamps or inode numbers.
            (admin / "index").unlink()
            fixture.git(root, "read-tree", "HEAD")
            for name in ("HEAD", "index"):
                path = admin / name
                files[path.relative_to(fixture.parent).as_posix()] = path.read_bytes()
            for name in fixture.git(root, "ls-files").splitlines():
                files[f"{root.name}/{name}"] = repository.git(root, "show", "HEAD:" + name)
        with ZipFile(archive, "w") as output:
            for name, content in sorted(files.items()):
                write_entry(output, name, content)
    finally:
        fixture.doCleanups()


if __name__ == "__main__":
    date = "2000-01-01T00:00:00+00:00"
    with patch.dict(os.environ, {"GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull,
                                 "GIT_CONFIG_COUNT": "1", "GIT_CONFIG_KEY_0": "core.autocrlf",
                                 "GIT_CONFIG_VALUE_0": "input", "GIT_AUTHOR_DATE": date,
                                 "GIT_COMMITTER_DATE": date, "GIT_AUTHOR_NAME": "Fixture",
                                 "GIT_COMMITTER_NAME": "Fixture", "GIT_AUTHOR_EMAIL": "fixture@example.invalid",
                                 "GIT_COMMITTER_EMAIL": "fixture@example.invalid"}):
        build(Path(__file__).with_name("partial-cleanup.zip"))

"""Rebuild the verification suite's Git snapshots using native Git and verification commands."""

from contextlib import contextmanager
from datetime import datetime
from itertools import count
import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
from unittest.mock import patch
from uuid import UUID
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from git_fixtures import ARCHIVE
from test_evidence import ContinuationTests
from test_integration import IntegrationTests
import evidence
import integration
import timing_report


DATE = "2000-01-01T00:00:00+00:00"
ENVIRONMENT = {
    "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_AUTHOR_DATE": DATE, "GIT_COMMITTER_DATE": DATE,
    "GIT_AUTHOR_NAME": "Fixture", "GIT_AUTHOR_EMAIL": "fixture@example.invalid",
    "GIT_COMMITTER_NAME": "Fixture", "GIT_COMMITTER_EMAIL": "fixture@example.invalid",
}
CONFIG = """[core]
    repositoryformatversion = 0
    filemode = false
    bare = false
    logallrefupdates = true
    autocrlf = false
[user]
    name = Fixture
    email = fixture@example.invalid
"""


def git(root, *args):
    return subprocess.check_output(["git", *args], cwd=root, stderr=subprocess.PIPE).decode().strip()


def write(root, name, content):
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8", newline="\n")
    return path


def init(root, contents, branch="main", exclude="", message="fixture"):
    root.mkdir(parents=True, exist_ok=True)
    git(root, "init", "-q", "--template=", "-b", branch)
    write(root, ".git/config", CONFIG)
    write(root, ".git/info/exclude", exclude)
    for name, content in contents.items():
        write(root, name, content)
    commit(root, message)


def commit(root, message):
    git(root, "add", "-A")
    git(root, "commit", "--quiet", "--allow-empty", "--message", message)
    return git(root, "rev-parse", "HEAD")


@contextmanager
def temporary_root():
    with TemporaryDirectory() as temporary:
        yield Path(temporary).resolve()


@contextmanager
def fixture_case(kind, root):
    case = kind()
    try:
        case.configure(root)
        yield case
    finally:
        case.doCleanups()


def tracked_index(root):
    # Reading a tree into a fresh index omits host file times and inode numbers.
    tree = git(root, "write-tree")
    with TemporaryDirectory() as temporary:
        index = Path(temporary) / "index"
        with patch.dict(os.environ, {"GIT_INDEX_FILE": str(index)}):
            git(root, "read-tree", tree)
        return index.read_bytes()


def repository_files(parent, root, linked):
    common = root / ".git"
    paths = [common / name for name in ("config", "info/exclude", "HEAD")]
    paths.extend(common.glob("objects/??/*"))
    paths.extend((common / "refs").rglob("*"))
    for path in paths:
        if path.is_file():
            yield path.relative_to(parent).as_posix(), path.read_bytes()
    yield (common / "index").relative_to(parent).as_posix(), tracked_index(root)
    for checkout in linked:
        admin = common / "worktrees" / checkout.name
        link = admin.relative_to(parent).as_posix()
        yield f"{checkout.name}/.git", f"gitdir: ../primary/.git/worktrees/{checkout.name}\n".encode()
        yield link + "/commondir", b"../..\n"
        yield link + "/HEAD", (admin / "HEAD").read_bytes()
        yield link + "/index", tracked_index(checkout)
    for checkout in (root, *linked):
        for directory, directories, files in os.walk(checkout):
            parent_dir = Path(directory)
            if parent_dir == checkout and ".git" in directories:
                directories.remove(".git")
            for name in files:
                path = parent_dir / name
                if path != checkout / ".git":
                    yield path.relative_to(parent).as_posix(), path.read_bytes()


def write_entry(archive, name, content):
    entry = ZipInfo(name)
    entry.create_system = 0  # Match the creator field of the checked-in ZIPs on every host.
    archive.writestr(entry, content, compress_type=ZIP_DEFLATED)


def save(archive, name, parent, root, linked=(), metadata=None):
    files = dict(repository_files(parent, root, linked))
    if metadata is not None:
        files["fixture.json"] = (json.dumps(metadata, indent=2, sort_keys=True) + "\n").encode()
    for path, content in sorted(files.items()):
        write_entry(archive, f"{name}/{path}", content)


def build_small(archive):
    with temporary_root() as root:
        init(root, {}, exclude=".agent/tmp/\n")
        save(archive, "empty", root, root)
    with temporary_root() as root:
        init(root, dict.fromkeys(("old.txt", "working.txt", "staged.txt"), "original"))
        git(root, "mv", "old.txt", "new.txt")
        write(root, "staged.txt", "staged")
        git(root, "add", "staged.txt")
        write(root, "working.txt", "working")
        write(root, "untracked.txt", "new")
        save(archive, "impact", root, root)


def build_worktree(archive):
    with temporary_root() as parent:
        root = parent / "primary"
        init(root, {".gitignore": "ignored*\n.agent/\n", "tracked.txt": "baseline\n"}, branch="master",
             message="initial fixture")
        save(archive, "worktree", parent, root)
        for name in ("existing", "task"):
            checkout = parent / f"primary-{name}"
            git(root, "worktree", "add", "-q", "-b", name, str(checkout))
            save(archive, f"worktree-{name}", parent, root, [checkout])
            git(root, "worktree", "remove", str(checkout))
            git(root, "branch", "-D", name)

        baseline = git(root, "rev-parse", "HEAD")
        config = "ref_src/.graphifyignore"
        write(root, ".gitignore", "/ref_src/*\n!/ref_src/.graphifyignore\n/tools/AmigaTestKit/\n"
              "/graphify-out/\n/.env\nignored*\n")
        write(root, config, "target-specific/\n")
        for directory in ("Obsidian/Amiga/Reference", ".agent/tmp/singlestep", ".agent/tmp/benchmarks"):
            write(root, f"{directory}/tracked.txt", "checked-out content\n")
        commit(root, "asset fixture")
        save(archive, "worktree-assets", parent, root)
        git(root, "reset", "--hard", baseline)

        write(root, ".env", "FIXTURE=target\n")
        commit(root, "tracked environment fixture")
        save(archive, "worktree-environment", parent, root)
        git(root, "reset", "--hard", baseline)

        write(root, "ref_src", "destination conflict\n")
        commit(root, "copy destination conflict")
        save(archive, "worktree-conflict", parent, root)


def make_portable_evidence(root, directory):
    record = integration.read(directory / "commit.json")
    record["checkout"] = ""
    for group in record["groups"].values():
        group["elapsed_seconds"] = 0.0
        for _, command, _ in group["commands"]:
            command[0] = "python"
        group["identity"] = ""
        artifacts = group["artifacts"]
        for relative in list(artifacts):
            if not relative.endswith(".log"):
                continue
            log = directory / relative
            portable = log.with_name("fixture.log")
            write(portable.parent, portable.name, log.read_text(encoding="utf-8").replace(sys.executable, "python"))
            log.unlink()
            del artifacts[relative]
            artifacts[portable.relative_to(directory).as_posix()] = evidence.file_digest(portable)
    if "timing_report" in record and "path" in record["timing_report"]:
        timing = record["timing_report"]
        for row in timing["rows"]:
            for command in row["commands"]:
                command[0] = "python"
            if row["seconds"] is not None:
                row["seconds"] = 0.0
        report = directory / timing["path"]
        timing.update(generation_seconds=0.0, write_seconds=0.0, wall_seconds_before_report=0.0)
        write(report.parent, report.name,
              timing_report.render(root, "commit", record["head"], timing["rows"], 0.0))
        timing["sha256"] = evidence.file_digest(report)

    # The verification lock is transient, even when its file persists.
    (root / ".agent/tmp/verification.lock").unlink(missing_ok=True)
    write(directory, "commit.json", json.dumps(record, indent=2) + "\n")
    return record


def build_evidence(archive):
    for state in ("evidence-passed", "evidence-diary"):
        with temporary_root() as root, fixture_case(ContinuationTests, root) as case:
            init(root, {".gitignore": ".agent/\n", "source.rs": "fn main() {}\n", case.words: "accepted\n"})
            write(root, "source.rs", "fn main() { /* changed */ }\n")
            if state == "evidence-passed":
                save(archive, "evidence", root, root)
            else:
                write(root, "DIARY.md", "First entry.\n")
                case.include_diary_groups()
            if case.run_profile() != 0:
                raise ValueError(case.output)
            make_portable_evidence(root, case.directory)
            save(archive, state, root, root)


def save_integration(archive, name, case, sources=(), linked=()):
    relative_sources = [{**item, "root": Path(item["root"]).relative_to(case.parent).as_posix()} for item in sources]
    if case.record.is_file():
        record = integration.read(case.record)
        if not relative_sources:
            relative_sources = [{**item, "root": Path(item["root"]).name} for item in record["sources"]]
        record["evidence"] = make_portable_evidence(case.source, case.directory)
        for key in ("source_root", "target_root", "evidence_directory"):
            record[key] = Path(record[key]).relative_to(case.parent).as_posix()
        record["sources"] = relative_sources
        record["review_artifacts"] = {
            Path(name).relative_to(case.parent).as_posix(): digest
            for name, digest in record["review_artifacts"].items()
        }
        if record["sources_explicit"]:
            record["review"]["sources"] = evidence.digest(relative_sources)
            for artifact, data in (("sources.json", relative_sources), ("review.json", record["review"])):
                path = write(case.source, ".agent/tmp/" + artifact, json.dumps(data))
                record["review_artifacts"][path.relative_to(case.parent).as_posix()] = evidence.file_digest(path)
        write(case.directory, case.record.name, json.dumps(record, indent=2) + "\n")
    save(archive, name, case.parent, case.target, (case.source, *linked),
         metadata={"base": case.base, "sources": relative_sources})


def build_integration(archive):
    for state in ("integration-sealed", "integration-published", "integration-combined-sealed"):
        with temporary_root() as root, fixture_case(IntegrationTests, root) as case:
            init(case.target, {".gitignore": ".agent/\n.env\nignored*\ntarget/\n", "value.txt": "baseline\n"},
                 branch="master", exclude="/tools/worktree.ps1\n")
            case.base = case.git(case.target, "rev-parse", "HEAD")
            case.git(case.target, "worktree", "add", "-q", "-b", "task", str(case.source), case.base)
            case.configure_verifier()
            if state == "integration-combined-sealed":
                sources = case.arrange_sources()
                linked = [Path(item["root"]) for item in sources[1:]]
                save_integration(archive, "integration-assembled", case, sources, linked)
                case.prepare(sources=sources)
                save_integration(archive, state, case, sources, linked)
            else:
                if state == "integration-sealed":
                    save_integration(archive, "integration", case)
                case.prepare()
                if state == "integration-published":
                    integration.publish(case.source, case.record)
                    integration.cleanup_record(integration.read(case.record)).unlink()
                save_integration(archive, state, case)


def build(output):
    serial = count(1)
    with patch.dict(os.environ, ENVIRONMENT), \
         patch.object(evidence.uuid, "uuid4", side_effect=lambda: UUID(int=next(serial))), \
         patch.object(timing_report, "datetime") as clock, ZipFile(output, "w") as archive:
        clock.now.return_value = datetime.fromisoformat(DATE)
        build_small(archive)
        build_worktree(archive)
        build_evidence(archive)
        build_integration(archive)


if __name__ == "__main__":
    build(Path(sys.argv[1]) if len(sys.argv) > 1 else ARCHIVE)

"""Rebuild the verification suite's Git snapshots using native Git."""

from contextlib import contextmanager
import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
from unittest.mock import patch
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from git_fixtures import ARCHIVE


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


def build(output):
    with patch.dict(os.environ, ENVIRONMENT), ZipFile(output, "w") as archive:
        build_small(archive)
        build_worktree(archive)


if __name__ == "__main__":
    build(Path(sys.argv[1]) if len(sys.argv) > 1 else ARCHIVE)

"""Native Git state and process-held locks for local publication."""
from contextlib import contextmanager
import json
import os
from pathlib import Path
import subprocess

from repository import git


def value(root, *args):
    return git(root, *args).decode("utf-8").strip()


def save(path, record):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    with temporary.open("w", encoding="utf-8") as stream:
        json.dump(record, stream, indent=2)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    temporary.replace(path)


def common(root):
    return Path(value(root, "rev-parse", "--path-format=absolute", "--git-common-dir"))


@contextmanager
def publication_lock(common_directory):
    # Retain the inode/file: another opener must lock the same object.
    with (common_directory / "local-integration.lock").open("a+b") as stream:
        try:
            stream.seek(0)
            if os.name == "nt":
                import msvcrt
                msvcrt.locking(stream.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as error:
            raise ValueError("Another local publisher holds the integration lock") from error
        yield


def worktrees(root):
    entries = []
    for block in git(root, "worktree", "list", "--porcelain", "-z").decode("utf-8").split("\0\0"):
        entry = dict(line.split(" ", 1) if " " in line else (line, "")
                     for line in block.split("\0") if line)
        if entry:
            entries.append(entry)
    return entries


def checkout(root, branch, registrations=None):
    entries = worktrees(root) if registrations is None else registrations
    matches = [entry for entry in entries if entry.get("branch") == branch]
    if len(matches) != 1 or any(key in matches[0] for key in ("locked", "prunable")):
        raise ValueError("Expected one unlocked registered checkout for " + branch)
    return Path(matches[0]["worktree"]).resolve()


def no_operation(root):
    names = ("MERGE_HEAD", "CHERRY_PICK_HEAD", "REVERT_HEAD",
             "rebase-merge", "rebase-apply", "sequencer", "BISECT_START")
    options = [option for name in names for option in ("--git-path", name)]
    output = git(root, "rev-parse", "--path-format=absolute", *options).decode("utf-8")
    paths = [Path(line) for line in output.removesuffix("\n").split("\n")]
    if ("\0" in output or len(paths) != len(names)
            or any(not path.is_absolute() or path.name != name for name, path in zip(names, paths))):
        raise ValueError("Git returned invalid operation marker paths")
    for name, path in zip(names, paths):
        if path.exists():
            raise ValueError("Git operation is active: " + name)


def clean(root, untracked=True):
    no_operation(root)
    if git(root, "status", "--porcelain", "-z", "--untracked-files=" + ("all" if untracked else "no")):
        raise ValueError("Checkout has pending changes: " + str(root))


def represents(root, tree):
    return (value(root, "write-tree") == tree
            and subprocess.run(["git", "diff", "--quiet", "--ignore-submodules=none"], cwd=root).returncode == 0)


def ancestor(root, older, newer):
    if subprocess.run(["git", "merge-base", "--is-ancestor", older, newer], cwd=root).returncode != 0:
        raise ValueError("Recorded baseline is not an ancestor of the source commit")


def collisions(root, commit):
    extras = set()
    for options in ((), ("--ignored",)):
        extras.update(git(root, "ls-files", "--others", "--exclude-standard", *options, "-z").decode("utf-8").split("\0"))
    tracked = git(root, "ls-tree", "-r", "--name-only", "-z", commit).decode("utf-8").split("\0")
    normalize = str.casefold if os.name == "nt" else str
    files = {normalize(name) for name in tracked if name}
    directories = {"/".join(name.split("/")[:index])
                   for name in files for index in range(1, len(name.split("/")))}
    for extra in extras - {""}:
        name = normalize(extra)
        parents = {"/".join(name.split("/")[:index]) for index in range(1, len(name.split("/")))}
        if name in files or name in directories or parents & files:
            raise ValueError("Untracked or ignored path would be overwritten: " + extra)


class RefTransaction:
    """Keep prepared native reference locks alive through the checkout transition."""
    def __init__(self, root, target, base, source, commit, delete=False, verify_only=False, sources=()):
        self.process = subprocess.Popen(
            ["git", "update-ref", "-m", "local integration " + base + " -> " + commit, "--stdin"],
            cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            self.send("start")
            changes = (f"verify {target} {base}\n" if verify_only else
                       f"verify {target} {base}\ndelete {source} {commit}\n" if delete else
                       f"update {target} {commit} {base}\nverify {source} {commit}\n")
            changes += "".join(f"verify {branch} {tip}\n" for branch, tip in sources)
            self.process.stdin.write(changes.encode("utf-8"))
            self.send("prepare")
        except BaseException:
            self.close()
            raise

    def send(self, command):
        # Git's protocol requires LF, including on Windows. A text pipe would
        # translate it to CRLF and turn `start` into an unknown command.
        self.process.stdin.write((command + "\n").encode("utf-8"))
        self.process.stdin.flush()
        if self.process.stdout.readline().decode("utf-8").strip() != command + ": ok":
            detail = self.process.stderr.read().decode("utf-8", errors="replace")
            raise ValueError("Git reference transaction refused: " + detail.strip())

    def commit(self):
        self.send("commit")

    def close(self):
        self.process.stdin.close()
        self.process.wait(timeout=10)
        self.process.stdout.close()
        self.process.stderr.close()

"""Repository root, changed paths and verification snapshot identity."""
import hashlib
import subprocess
import re
import os
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def git(root, *args):
    result = subprocess.run(["git", *args], cwd=root, capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr.decode("utf-8", errors="replace"))
    return result.stdout


def resolve_base(root, revision):
    """Integration comparisons name an immutable, complete commit ID."""
    if not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", revision):
        raise ValueError("--integration-base requires a full immutable commit ID")
    if git(root, "rev-parse", "--verify", revision + "^{commit}").decode().strip() != revision:
        raise ValueError("Integration base must identify a commit")
    return revision


def collect_changes(root, base=None):
    """Include staged, unstaged and new paths, with both rename endpoints."""
    changes = set()
    if base is not None:
        changes.update(git(root, "diff", "--name-only", "--no-renames", "-z", base, "HEAD")
                       .decode("utf-8").split("\0"))
    for args in (("diff", "--cached", "--name-only", "--no-renames", "-z"),
                 ("diff", "--name-only", "--no-renames", "-z"),
                 ("ls-files", "--others", "--exclude-standard", "-z")):
        changes.update(git(root, *args).decode("utf-8").split("\0"))
    return changes - {""}


def candidate_tree(root):
    """Build a tree from the candidate without changing the real index."""
    directory = root / ".agent/tmp"
    directory.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=directory) as temporary:
        environment = {**os.environ, "GIT_INDEX_FILE": str(Path(temporary) / "index")}
        def run(*args):
            result = subprocess.run(["git", *args], cwd=root, env=environment, capture_output=True)
            if result.returncode:
                raise ValueError(result.stderr.decode("utf-8", errors="replace"))
            return result.stdout.decode().strip()
        # Start from the real index tree, retaining explicitly force-added
        # ignored files. Applying the worktree then includes unstaged edits.
        run("read-tree", git(root, "write-tree").decode().strip())
        run("add", "-A", "--", ".")
        return run("write-tree")


def merge_heads(root):
    path = Path(git(root, "rev-parse", "--path-format=absolute", "--git-path", "MERGE_HEAD").decode().strip())
    return path.read_text().splitlines() if path.exists() else []


def divergent_integration(root, base, source=None):
    if source is not None:
        original = resolve_base(root, source)
        common = git(root, "merge-base", base, original).decode().strip()
        if common not in {base, original}:
            return True
    return bool(merge_heads(root) or git(root, "rev-list", "--merges", base + "..HEAD").strip())


def snapshot(root):
    """Fingerprint source and index, excluding generated benchmark reports."""
    digest = hashlib.sha256()
    for args in (("rev-parse", "HEAD"), ("diff", "--cached", "--binary"),
                 ("diff", "--binary")):
        digest.update(git(root, *args))
    paths = [path for path in git(root, "ls-files", "--others", "--exclude-standard", "-z")
             .decode("utf-8").split("\0") if path and not path.startswith("benchmarks/results/")]
    for path in paths:
        digest.update(path.encode("utf-8"))
        digest.update(hashlib.sha256((root / path).read_bytes()).digest())
    return digest.hexdigest()

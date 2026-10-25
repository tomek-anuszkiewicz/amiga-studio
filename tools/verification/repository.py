"""Repository root, changed paths and verification snapshot identity."""
import hashlib
import subprocess
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]


def git(root, *args):
    result = subprocess.run(["git", *args], cwd=root, capture_output=True)
    if result.returncode:
        raise RuntimeError(result.stderr.decode("utf-8", errors="replace"))
    return result.stdout


def collect_changes(root):
    """Include staged, unstaged and new paths, with both rename endpoints."""
    changes = set()
    for args in (("diff", "--cached", "--name-only", "--no-renames", "-z"),
                 ("diff", "--name-only", "--no-renames", "-z"),
                 ("ls-files", "--others", "--exclude-standard", "-z")):
        changes.update(git(root, *args).decode("utf-8").split("\0"))
    return changes - {""}


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

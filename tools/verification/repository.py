"""Repository root and currently changed paths for check selection."""
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

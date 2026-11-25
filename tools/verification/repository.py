"""Repository root, changed paths and Cargo workspace membership."""
import json
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


def workspace_packages(root):
    """Read current workspace membership without building or changing the lockfile."""
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline", "--locked"],
        cwd=root, capture_output=True, text=True, encoding="utf-8")
    if result.returncode:
        raise ValueError("Cargo metadata failed: " + result.stderr.strip())
    data = json.loads(result.stdout)
    members = set(data["workspace_members"])
    packages = [package for package in data["packages"] if package["id"] in members]
    if not members or {package["id"] for package in packages} != members:
        raise ValueError("Cargo metadata has incomplete workspace membership")
    roots = {Path(package["manifest_path"]).resolve().parent: package["name"] for package in packages}
    if len(roots) != len(packages) or len(set(roots.values())) != len(packages):
        raise ValueError("Cargo metadata has ambiguous package ownership")
    for directory in roots:
        directory.relative_to(root.resolve())
    return packages, roots

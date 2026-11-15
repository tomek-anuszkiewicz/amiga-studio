"""Remove an unchanged integrated task checkout and its ignored local files."""
import os
from pathlib import Path
import subprocess

import psutil

from integration_state import ancestor, clean, common, publication_lock, RefTransaction, value, worktrees


def stopped(source):
    """A process with a confirmed working directory here blocks deletion."""
    for process in psutil.process_iter():
        try:
            working = Path(process.cwd()).resolve()
            if working == source or working.is_relative_to(source):
                raise ValueError(f"A process still uses the source checkout (PID {process.pid})")
        except (psutil.NoSuchProcess, psutil.ZombieProcess, psutil.AccessDenied):
            continue



def cleanup(root, path):
    from integration import read
    frozen = read(path)
    if frozen["state"] != "published":
        raise ValueError("Cleanup requires confirmed publication")
    source = Path(frozen["source_root"]).resolve()
    target = Path(frozen["target_root"]).resolve()
    if root.resolve() != target or root.resolve() == source:
        raise ValueError("Run cleanup from the recorded surviving target checkout")
    if source == common(root).parent or source.parent != target.parent:
        raise ValueError("Refusing to remove the primary checkout or a non-sibling directory")
    branch, commit = frozen["source_ref"], frozen["commit"]
    with publication_lock(root):
        if (common(root) / "local-integration.json").exists():
            raise ValueError("Recover the unfinished publication before cleanup")
        target_tip = value(root, "rev-parse", frozen["target_ref"])
        ancestor(root, commit, target_tip)
        entries = worktrees(root)
        registered = [entry for entry in entries if entry.get("branch") == branch]
        source_entries = [entry for entry in entries if Path(entry["worktree"]).resolve() == source]
        branch_exists = subprocess.run(["git", "show-ref", "--verify", "--quiet", branch], cwd=root).returncode
        if not source.exists() and not registered and not source_entries and branch_exists == 1:
            return
        if branch_exists or value(root, "rev-parse", branch) != commit:
            raise ValueError("Source branch advanced; retain its checkout and branch")
        if source.exists():
            if len(registered) != 1 or registered != source_entries:
                raise ValueError("Source checkout ownership changed")
            if any(key in registered[0] for key in ("locked", "prunable")) or source.is_symlink():
                raise ValueError("Source checkout is locked, missing or linked")
            clean(source)
            if value(source, "rev-parse", "HEAD") != commit or value(source, "symbolic-ref", "HEAD") != branch:
                raise ValueError("Source checkout advanced or changed branch")
            stopped(source)
        elif registered or source_entries:
            raise ValueError("Source checkout registration remains; retain its branch")
        transaction = RefTransaction(root, frozen["target_ref"], target_tip, branch, commit, delete=True)
        try:
            if source.exists():
                clean(source)
                if value(source, "rev-parse", "HEAD") != commit:
                    raise ValueError("Source changed while preparing cleanup locks")
                shell = "powershell" if os.name == "nt" else "pwsh"
                result = subprocess.run([shell, "-NoProfile", "-File", str(target / "tools/worktree.ps1"), "remove", str(source)], cwd=target)
                if result.returncode:
                    raise ValueError("Integration succeeded; worktree cleanup failed. Inspect the remaining directory, registration and branch.")
            remaining = worktrees(root)
            if source.exists() or any(Path(entry["worktree"]).resolve() == source for entry in remaining):
                raise ValueError("Integration succeeded; worktree removal is incomplete")
            if any(entry.get("branch") == branch for entry in remaining):
                raise ValueError("Integrated branch is checked out elsewhere; retain it")
            transaction.commit()
        finally:
            transaction.close()
        if subprocess.run(["git", "show-ref", "--verify", "--quiet", branch], cwd=root).returncode != 1:
            raise ValueError("Integrated branch remains after conditional deletion")

"""Preserve local material before removing an unchanged integrated task checkout."""
import os
from pathlib import Path
import shutil
import subprocess

import psutil

from evidence import file_digest
from integration_state import ancestor, clean, common, publication_lock, RefTransaction, save, value, worktrees
from repository import git


def stopped(source):
    """A task-owned command with a working directory here blocks deletion."""
    user = psutil.Process().username()
    for process in psutil.process_iter():
        try:
            if process.username() != user:
                continue
            working = Path(process.cwd()).resolve()
            if working == source or working.is_relative_to(source):
                raise ValueError(f"A process still uses the source checkout (PID {process.pid})")
        except (psutil.NoSuchProcess, psutil.ZombieProcess):
            continue
        except psutil.AccessDenied as error:
            raise ValueError(f"Cannot establish process ownership for cleanup (PID {process.pid})") from error


def ignored_inventory(source):
    names = git(source, "ls-files", "--others", "--ignored", "--exclude-standard", "-z").decode("utf-8").split("\0")
    result = {}
    for name in names:
        if not name:
            continue
        path = source / name
        if path.is_symlink() or any(parent.is_symlink() for parent in path.parents if parent != source):
            raise ValueError("Cannot preserve local files through a link: " + name)
        if not path.is_file():
            raise ValueError("Cannot inventory ignored local file: " + name)
        # Build products and Python bytecode are recreatable. Keep all other
        # unknown material, including plans, diagnostics, logs and .env changes.
        if name.startswith(("target/", "graphify-out/")) or "__pycache__" in path.parts:
            result[name] = None
        else:
            result[name] = file_digest(path)
    return result


def preserve(source, surviving, destination, inventory):
    destination.mkdir(parents=True, exist_ok=True)
    for name, expected in inventory.items():
        if expected is None:
            continue
        original = source / name
        peer = surviving / name
        # Identical physically copied fixtures already survive in the target.
        if peer.is_file() and not peer.is_symlink() and file_digest(peer) == expected:
            continue
        saved = destination / "files" / name
        saved.parent.mkdir(parents=True, exist_ok=True)
        if saved.exists() and file_digest(saved) != expected:
            raise ValueError("Preservation destination contains different material: " + name)
        shutil.copy2(original, saved)
        if file_digest(saved) != expected or file_digest(original) != expected:
            raise ValueError("Local material changed while preserving: " + name)
    save(destination / "ignored-inventory.json", inventory)


def cleanup(root, path):
    from integration import read
    frozen = read(path)
    if frozen["state"] not in {"published", "worktree-removed"}:
        raise ValueError("Cleanup requires confirmed publication")
    source = Path(frozen["source_root"]).resolve()
    target = Path(frozen["target_root"]).resolve()
    if root.resolve() != target or root.resolve() == source:
        raise ValueError("Run cleanup from the recorded surviving target checkout")
    if source == common(root).parent or source.parent != target.parent:
        raise ValueError("Refusing to remove the primary checkout or a non-sibling directory")
    branch, commit = frozen["source_ref"], frozen["commit"]
    destination = target / ".agent/tmp/integration-archives" / (branch.removeprefix("refs/heads/").replace("/", "-") + "-" + commit[:12])
    retained = destination / "integration.json"
    with publication_lock(root):
        if (common(root) / "local-integration.json").exists():
            raise ValueError("Recover the unfinished publication before cleanup")
        target_tip = value(root, "rev-parse", frozen["target_ref"])
        ancestor(root, commit, target_tip)
        branch_exists = subprocess.run(["git", "show-ref", "--verify", "--quiet", branch], cwd=root).returncode
        if branch_exists == 1 and frozen["state"] == "worktree-removed" and not source.exists():
            if any(entry.get("branch") == branch for entry in worktrees(root)):
                raise ValueError("Branch was removed but a checkout still names it")
            frozen["state"] = "cleaned"
            save(retained, frozen)
            return
        if branch_exists or value(root, "rev-parse", branch) != commit:
            raise ValueError("Source branch advanced; retain its checkout and branch")
        registered = [entry for entry in worktrees(root) if entry.get("branch") == branch]
        if (frozen["state"] == "published" and not source.exists() and not registered
                and path.resolve() == retained.resolve() and frozen.get("archive") == str(destination)
                and not any(Path(entry["worktree"]).resolve() == source for entry in worktrees(root))):
            # An interruption can occur after native removal but before the
            # next phase write. Only the pre-removal archive can bridge it.
            frozen["state"] = "worktree-removed"
            save(retained, frozen)
        if frozen["state"] == "published":
            if len(registered) != 1 or Path(registered[0]["worktree"]).resolve() != source:
                raise ValueError("Source checkout ownership changed")
            if any(key in registered[0] for key in ("locked", "prunable")) or source.is_symlink():
                raise ValueError("Source checkout is locked, missing or linked")
            clean(source)
            if value(source, "rev-parse", "HEAD") != commit or value(source, "symbolic-ref", "HEAD") != branch:
                raise ValueError("Source checkout advanced or changed branch")
            stopped(source)
            inventory = ignored_inventory(source)
            preserve(source, target, destination, inventory)
            # Archive the record before removing the checkout that owns it.
            frozen["archive"] = str(destination)
            save(retained, frozen)
            clean(source)
            stopped(source)
            if ignored_inventory(source) != inventory:
                raise ValueError("Ignored files changed during preservation; retain the checkout")
            if value(source, "rev-parse", "HEAD") != commit:
                raise ValueError("Source advanced during preservation")
            # Removal never forces Git, prunes registrations or recursively
            # deletes a leftover directory. A failure leaves the branch intact.
            shell = "powershell" if os.name == "nt" else "pwsh"
            transaction = RefTransaction(root, frozen["target_ref"], target_tip, branch, commit, delete=True)
            try:
                clean(source)
                if value(source, "rev-parse", "HEAD") != commit:
                    raise ValueError("Source changed while preparing cleanup locks")
                result = subprocess.run([shell, "-NoProfile", "-File", str(target / "tools/worktree.ps1"), "remove", str(source)], cwd=target)
                if result.returncode:
                    raise ValueError("Integration succeeded; worktree cleanup failed. Retained record: " + str(retained))
                if source.exists() or any(Path(entry["worktree"]).resolve() == source for entry in worktrees(root)):
                    raise ValueError("Integration succeeded; worktree removal is incomplete")
                frozen["state"] = "worktree-removed"
                save(retained, frozen)
                if any(entry.get("branch") == branch for entry in worktrees(root)):
                    raise ValueError("Integrated branch is checked out elsewhere; retain it")
                transaction.commit()
            finally:
                transaction.close()
        if any(entry.get("branch") == branch for entry in worktrees(root)):
            raise ValueError("Integrated branch is checked out elsewhere; retain it")
        # Expected-old deletion cannot discard a concurrently advanced branch.
        if frozen["state"] == "worktree-removed" and branch_exists == 0:
            # Retrying after a removal/transaction failure still uses expected-old.
            result = subprocess.run(["git", "show-ref", "--verify", "--quiet", branch], cwd=root)
            if result.returncode == 0:
                transaction = RefTransaction(root, frozen["target_ref"], target_tip, branch, commit, delete=True)
                try:
                    if any(entry.get("branch") == branch for entry in worktrees(root)):
                        raise ValueError("Integrated branch is checked out elsewhere; retain it")
                    transaction.commit()
                finally:
                    transaction.close()
        if subprocess.run(["git", "show-ref", "--verify", "--quiet", branch], cwd=root).returncode != 1:
            raise ValueError("Integrated branch remains after conditional deletion")
        frozen["state"] = "cleaned"
        save(retained, frozen)
        print("[INFO] Preserved integration material: " + str(destination))

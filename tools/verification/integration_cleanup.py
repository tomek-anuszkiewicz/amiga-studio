"""Remove an unchanged integrated task checkout and its ignored local files."""
from contextlib import ExitStack
import os
from pathlib import Path
import subprocess
import sys

import psutil

from integration_state import ancestor, checkout, clean, common, publication_lock, RefTransaction, save, value, worktrees
from integration_sources import cleanup_record


def stopped(source):
    """A process with a confirmed working directory here blocks deletion."""
    for process in psutil.process_iter():
        try:
            working = Path(process.cwd()).resolve()
            if working == source or working.is_relative_to(source):
                raise ValueError(f"A process still uses the source checkout (PID {process.pid})")
        except (psutil.NoSuchProcess, psutil.ZombieProcess, psutil.AccessDenied):
            continue


def branch_status(root, branch):
    return subprocess.run(["git", "show-ref", "--verify", "--quiet", branch], cwd=root).returncode


def source_outcome(root, source, branch, *, confirm_deletion=False):
    entries = worktrees(root)
    directory_present = source.exists()
    registered = any(Path(entry["worktree"]).resolve() == source for entry in entries)
    branch_present = any(entry.get("branch") == branch for entry in entries)
    if confirm_deletion or not branch_present:
        ref_present = branch_status(root, branch) != 1
        if confirm_deletion and ref_present:
            raise ValueError("Integrated branch remains after conditional deletion")
        branch_present = branch_present or ref_present
    return {"directory": "present" if directory_present else "removed",
            "registration": "present" if registered else "removed",
            "branch": "present" if branch_present else "removed"}


def remove_source(root, common_directory, frozen, selected, *, closure_commit=None):
    """Remove one source under the caller's publication lock and report its current state."""
    source = Path(selected["root"]).resolve()
    target = Path(frozen["target_root"]).resolve()
    if root.resolve() != target or root.resolve() == source:
        raise ValueError("Run cleanup from the recorded surviving target checkout")
    if source == common_directory.parent or source.parent != target.parent:
        raise ValueError("Refusing to remove the primary checkout or a non-sibling directory")
    branch = selected["ref"]
    commit = frozen["commit"] if branch == frozen["source_ref"] else selected["tip"]
    if (common_directory / "local-integration.json").exists():
        raise ValueError("Recover the unfinished publication before cleanup")
    target_tip = value(root, "rev-parse", frozen["target_ref"])
    if closure_commit is not None and target_tip != closure_commit:
        raise ValueError("Master differs from the recorded closure commit")
    ancestor(root, commit, target_tip)
    entries = worktrees(root)
    registered = [entry for entry in entries if entry.get("branch") == branch]
    source_entries = [entry for entry in entries if Path(entry["worktree"]).resolve() == source]
    if not registered and not source_entries:
        status = branch_status(root, branch)
        if status == 1 and not source.exists():
            return {"directory": "removed", "registration": "removed", "branch": "removed"}
        if status != 0:
            raise ValueError("Source branch advanced; retain its checkout and branch")
    tip = subprocess.run(["git", "show-ref", "--verify", "--hash", branch], cwd=root, capture_output=True)
    if tip.returncode or tip.stdout.strip() != commit.encode("utf-8"):
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
    if frozen.get("delivery") is not None:
        workflow = Path(__file__).resolve().parents[1] / "workflow"
        if str(workflow) not in sys.path:
            sys.path.append(str(workflow))
        from delivery import require_preserved_source
        require_preserved_source(root, frozen, branch, source)
    transaction = RefTransaction(root, frozen["target_ref"], target_tip, branch, commit, delete=True)
    try:
        if source.exists():
            clean(source)
            if (value(source, "symbolic-ref", "HEAD") != branch or checkout(root, branch) != source
                    or value(source, "rev-parse", "HEAD") != commit):
                raise ValueError("Source checkout ownership changed while preparing cleanup locks")
            shell = "powershell" if os.name == "nt" else "pwsh"
            result = subprocess.run(
                [shell, "-NoProfile", "-File", str(target / "tools/worktree.ps1"), "remove", str(source),
                 "-ExpectedBranch", branch], cwd=target)
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
    return source_outcome(root, source, branch, confirm_deletion=True)


def retained_record(durable, frozen):
    from integration import read
    retained = read(durable)
    if ({key: item for key, item in retained.items() if key != "cleanup"}
            != {key: item for key, item in frozen.items() if key != "cleanup"}):
        raise ValueError("Surviving integration record differs from the input")
    return retained


def cleanup_sources(frozen):
    """Use explicit delivery cleanup authority when present."""
    sources = frozen["sources"]
    selection = frozen.get("delivery")
    if selection is not None:
        branches = set(selection["cleanup"])
        if branches - {source["ref"] for source in sources}:
            raise ValueError("The delivery authorizes cleanup only for selected branches")
        sources = [source for source in sources if source["ref"] in branches]
    # Try tracked source worktrees before removing the assembly checkout.
    return sorted(sources, key=lambda item: item["ref"] == frozen["source_ref"])


def cleanup(root, path):
    from integration import read
    frozen = read(path)
    target = Path(frozen["target_root"]).resolve()
    if frozen["state"] != "published":
        raise ValueError("Cleanup requires confirmed publication")
    if root.resolve() != target or checkout(root, frozen["target_ref"]) != target:
        raise ValueError("Run cleanup from the recorded surviving target checkout")
    common_directory = common(root)
    durable = cleanup_record(frozen)
    with publication_lock(common_directory):
        if durable.exists():
            frozen = retained_record(durable, frozen)
        closure_commit = None
        if frozen["target_ref"] == "refs/heads/master":
            from integration_closure import require_closed
            if not durable.is_file():
                raise ValueError("Cleanup requires the surviving publication record")
            closure_commit = require_closed(root, frozen)
        if not durable.exists():
            save(durable, frozen)
    print("[INFO] Surviving cleanup record: " + str(durable))
    failures = []
    for selected in cleanup_sources(frozen):
        branch, source = selected["ref"], Path(selected["root"]).resolve()
        error = None
        with ExitStack() as locks:
            try:
                locks.enter_context(publication_lock(common_directory))
            except (ValueError, RuntimeError, OSError) as failure:
                # A refused attempt can only record its failure after acquiring the lock.
                error = str(failure)
                locks.enter_context(publication_lock(common_directory))
            retained = retained_record(durable, frozen)
            if error is None:
                try:
                    outcome = remove_source(root, common_directory, retained, selected, closure_commit=closure_commit)
                except (ValueError, RuntimeError, OSError) as failure:
                    error = str(failure)
            if error is not None:
                failures.append(branch + ": " + error)
                outcome = source_outcome(root, source, branch)
                outcome["error"] = error
            retained.setdefault("cleanup", {})[branch] = outcome
            save(durable, retained)
        print(("[BLOCKED] " if error is not None else "[PASS] ") + branch + ": " +
              ", ".join(key + "=" + item for key, item in outcome.items()))
    if failures:
        raise ValueError("Integration succeeded; cleanup remains incomplete: " + "; ".join(failures))

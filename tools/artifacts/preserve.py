"""Copy finished task outputs to an ignored archive in the primary checkout."""
import argparse
import json
from pathlib import Path, PurePosixPath
from stat import S_ISDIR, S_ISREG
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/verification"))
sys.path.insert(0, str(ROOT / "tools/workflow/capture"))
from case_report import primary_checkout
from repository import git
from preservation import copy_whole, reject_links, require_ignored


DEFAULTS = (
    ".agent/tmp/verification", ".agent/tmp/verify", ".agent/tmp/singlestep",
    ".agent/tmp/coverage", "benchmarks/results",
)
ARCHIVES = Path(".agent/task-archives")


def same_content(source, destination):
    try:
        target = destination.stat()
    except (FileNotFoundError, NotADirectoryError):
        return False
    if not S_ISREG(target.st_mode) or source.stat().st_size != target.st_size:
        return False
    with source.open("rb") as first, destination.open("rb") as second:
        while chunk := first.read(1024 * 1024):
            if chunk != second.read(len(chunk)):
                return False
        return not second.read(1)


def selected_files(worktree, includes, destination, *, shared):
    archives = worktree / ARCHIVES
    roots = includes if shared else (*DEFAULTS, *includes)
    selected = set()
    for name in roots:
        relative = PurePosixPath(name)
        if relative.is_absolute() or not relative.parts or ".." in relative.parts or ":" in name or "\\" in name:
            raise ValueError(f"Expected a worktree-relative output path: {name}")
        source = worktree / relative
        pending = [source]
        while pending:
            path = pending.pop()
            if path == archives or path == destination:
                continue
            reject_links(path)
            try:
                mode = path.stat().st_mode
            except (FileNotFoundError, NotADirectoryError):
                if path != source:
                    raise
                if name in includes:
                    raise ValueError(f"Selected output does not exist: {name}") from None
                continue
            if S_ISDIR(mode):
                if path.is_relative_to(destination) or (shared and path.is_relative_to(archives)):
                    continue
                try:
                    pending.extend(path.iterdir())
                except PermissionError as error:
                    raise PermissionError(f"Cannot list selected output directory {path}: {error}") from error
                continue
            if not S_ISREG(mode):
                continue
            relative_path = path.relative_to(worktree).as_posix()
            # check-ignore respects tracked files, including force-added ignored paths.
            ignored = subprocess.run(["git", "check-ignore", "--quiet", "--", relative_path],
                                     cwd=worktree, capture_output=True)
            if ignored.returncode not in (0, 1):
                raise ValueError(ignored.stderr.decode("utf-8", errors="replace"))
            if ignored.returncode == 0:
                selected.add(path)
            elif name in includes:
                raise ValueError(f"Selected output is not ignored: {relative_path}")
    return sorted(selected)


def preserve(worktree, destination, includes=()):
    reject_links(worktree)
    reject_links(destination)
    worktree, destination = worktree.resolve(strict=True), destination.resolve()
    primary = primary_checkout(worktree)
    actual_root = Path(git(worktree, "rev-parse", "--show-toplevel").decode().strip()).resolve()
    if actual_root != worktree:
        raise ValueError("Select the root of the source checkout")
    if not destination.is_relative_to(primary):
        raise ValueError("Archive must be inside the primary checkout")
    shared = worktree == primary
    if shared and not includes:
        raise ValueError("Select the task's output paths with --include when using the shared primary checkout")
    if not shared and destination.is_relative_to(worktree):
        raise ValueError("Archive must survive outside the task worktree")
    require_ignored(primary, destination / ".archive-probe")
    files = selected_files(worktree, includes, destination, shared=shared)
    outputs = []
    for source in files:
        target = destination / source.relative_to(worktree)
        if shared and (source.is_relative_to(primary / ARCHIVES) or source.is_relative_to(destination)):
            target, reused = source, True
        else:
            try:
                reject_links(target)
                before = source.stat()
                reused = same_content(source, target)
                if not reused:
                    target = copy_whole(primary, source, target)
                after = source.stat()
                unchanged = (before.st_size, before.st_mtime_ns) == (after.st_size, after.st_mtime_ns)
                if not unchanged or not same_content(source, target):
                    raise ValueError(f"Output changed or its copy differs at {target}: {source}")
            except OSError as error:
                raise type(error)(f"Cannot preserve {source} at {target}: {error}") from error
        outputs.append({"source": source.relative_to(worktree).as_posix(),
                        "archive": target.relative_to(primary).as_posix(), "reused": reused})
    return {"copied": sum(not item["reused"] for item in outputs),
            "reused": sum(item["reused"] for item in outputs), "files": outputs,
            "primary_reports": ".agent/task-archives/ (retained in place)",
            "limits": "Known output locations only; add --include for custom outputs. Producers must be finished."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worktree", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--include", action="append", default=[], metavar="RELATIVE_PATH")
    args = parser.parse_args()
    try:
        print(json.dumps(preserve(args.worktree, args.destination, args.include), indent=2))
        return 0
    except (OSError, ValueError, RuntimeError) as error:
        parser.exit(1, f"Preservation failed; retain the worktree: {error}\n")


if __name__ == "__main__":
    sys.exit(main())

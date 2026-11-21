"""Copy ignored task outputs to the primary checkout before worktree removal."""
import argparse
import json
from pathlib import Path, PurePosixPath
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
    ".agent/tmp/coverage", ".agent/task-archives", "benchmarks/results",
)


def same_content(source, destination):
    if not destination.is_file() or source.stat().st_size != destination.stat().st_size:
        return False
    with source.open("rb") as first, destination.open("rb") as second:
        while chunk := first.read(1024 * 1024):
            if chunk != second.read(len(chunk)):
                return False
        return not second.read(1)


def selected_files(worktree, includes):
    selected = set()
    for name in (*DEFAULTS, *includes):
        relative = PurePosixPath(name)
        if relative.is_absolute() or not relative.parts or ".." in relative.parts or ":" in name or "\\" in name:
            raise ValueError(f"Expected a worktree-relative output path: {name}")
        source = worktree / relative
        reject_links(source)
        if not source.exists():
            if name in includes:
                raise ValueError(f"Selected output does not exist: {name}")
            continue
        candidates = [source] if source.is_file() else source.rglob("*")
        for path in candidates:
            reject_links(path)
            if not path.is_file():
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
    if actual_root != worktree or worktree == primary:
        raise ValueError("Select the root of a task worktree, not the primary checkout")
    if not destination.is_relative_to(primary) or destination.is_relative_to(worktree):
        raise ValueError("Archive must survive outside the task worktree, inside the primary checkout")
    require_ignored(primary, destination / ".archive-probe")
    files = selected_files(worktree, includes)
    outputs = []
    for source in files:
        target = destination / source.relative_to(worktree)
        reject_links(target)
        before = source.stat()
        reused = same_content(source, target)
        if not reused:
            target = copy_whole(primary, source, target)
        after = source.stat()
        if (before.st_size, before.st_mtime_ns) != (after.st_size, after.st_mtime_ns) or not same_content(source, target):
            raise ValueError(f"Output changed or its copy differs; retain the worktree: {source}")
        outputs.append({"source": source.relative_to(worktree).as_posix(),
                        "archive": target.relative_to(primary).as_posix(), "reused": reused})
    return {"copied": sum(not item["reused"] for item in outputs),
            "reused": sum(item["reused"] for item in outputs), "files": outputs,
            "primary_reports": ".agent/task-archives/ (already outside the task worktree)",
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

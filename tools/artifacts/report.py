#!/usr/bin/env python3
"""Report local artifact sizes and timestamp groups without changing files."""

import argparse
from collections import Counter
from dataclasses import dataclass, field
from datetime import date, datetime
from pathlib import Path
import os
import stat


REPO_ROOT = Path(__file__).resolve().parents[2]
GIB = 1024 ** 3


@dataclass
class DirectoryUsage:
    size: int = 0
    first: date | None = None
    last: date | None = None


@dataclass
class Usage:
    files: int = 0
    size: int = 0
    missing_birthtime: int = 0
    created: Counter = field(default_factory=Counter)
    modified: Counter = field(default_factory=Counter)
    latest: Counter = field(default_factory=Counter)
    directories: dict[str, DirectoryUsage] = field(default_factory=dict)
    skipped: int = 0
    errors: list[str] = field(default_factory=list)

    def add(self, relative: Path, size: int, created: date | None, modified: date):
        self.files += 1
        self.size += size
        if created is None:
            self.missing_birthtime += 1
        else:
            self.created[created] += size
        self.modified[modified] += size
        latest = max(created, modified) if created is not None else modified
        self.latest[latest] += size
        # Keep Cargo profiles separate while exposing their large caches.
        depth = 3
        group = relative.parent.parts[:depth]
        key = "/".join(group)
        if len(relative.parent.parts) < depth:
            key += "/<files>"
        directory = self.directories.setdefault(key, DirectoryUsage())
        directory.size += size
        directory.first = min(directory.first or latest, latest)
        directory.last = max(directory.last or latest, latest)


def is_link(info: os.stat_result) -> bool:
    """Skip symbolic links and Windows reparse points, including junctions."""
    return stat.S_ISLNK(info.st_mode) or bool(
        getattr(info, "st_file_attributes", 0) & stat.FILE_ATTRIBUTE_REPARSE_POINT
    )


def scan(root: Path, repo: Path) -> Usage:
    usage = Usage()
    try:
        root_info = root.lstat()
    except FileNotFoundError:
        return usage
    except OSError as error:
        usage.errors.append(f"{root.relative_to(repo).as_posix()}: {error.strerror}")
        return usage
    if is_link(root_info):
        usage.skipped += 1
        return usage
    if not stat.S_ISDIR(root_info.st_mode):
        usage.errors.append(f"{root.relative_to(repo).as_posix()}: not a directory")
        return usage
    pending = [root]
    while pending:
        directory = pending.pop()
        try:
            with os.scandir(directory) as entries:
                for entry in entries:
                    relative = Path(entry.path).relative_to(repo)
                    try:
                        info = entry.stat(follow_symlinks=False)
                        if is_link(info):
                            usage.skipped += 1
                        elif stat.S_ISDIR(info.st_mode):
                            pending.append(Path(entry.path))
                        elif stat.S_ISREG(info.st_mode):
                            birth = getattr(info, "st_birthtime", None)
                            created = datetime.fromtimestamp(birth).date() if birth is not None else None
                            modified = datetime.fromtimestamp(info.st_mtime).date()
                            usage.add(relative, info.st_size, created, modified)
                    except OSError as error:
                        usage.errors.append(f"{relative.as_posix()}: {error.strerror}")
        except OSError as error:
            usage.errors.append(f"{directory.relative_to(repo).as_posix()}: {error.strerror}")
    return usage


def date_range(first, last):
    return str(first) if first == last else f"{first}..{last}"


def render(label: str, usage: Usage) -> str:
    lines = [f"\n{label}: {usage.size / GIB:.2f} GiB, {usage.files:,} files"]
    if usage.files:
        lines += [
            "Current file sizes by observed dates (GiB):",
            "Date           Created    Modified     Later date    Share",
        ]
        days = usage.created.keys() | usage.modified.keys()
        for day in sorted(days):
            created = usage.created[day]
            modified = usage.modified[day]
            latest = usage.latest[day]
            share = 100 * latest / usage.size if usage.size else 0
            lines.append(
                f"{str(day):<14} "
                f"{created / GIB:9.2f} {modified / GIB:11.2f} "
                f"{latest / GIB:14.2f} {share:7.1f}%"
            )
        lines.append("Largest directories (range of later file dates):")
        largest = sorted(usage.directories.items(), key=lambda item: item[1].size, reverse=True)
        for name, directory in largest[:8]:
            lines.append(
                f"  {directory.size / GIB:7.2f} GiB  "
                f"{date_range(directory.first, directory.last):<22}  {name}"
            )
    if usage.missing_birthtime:
        lines.append(
            f"Creation time unavailable for {usage.missing_birthtime:,} files; "
            "their later date uses modification time only."
        )
    if usage.skipped:
        lines.append(f"Excluded {usage.skipped:,} links or reparse points.")
    if usage.errors:
        lines.append(f"INCOMPLETE: {len(usage.errors)} scan errors; totals exclude unreadable entries.")
        lines.extend(f"  {error}" for error in usage.errors[:5])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    print(f"Local artifact storage: {datetime.now().astimezone().isoformat(timespec='seconds')}")
    print("Logical file sizes in GiB; dates use the local timezone. No files are changed.")
    print("Each column independently sums current file sizes; do not add the columns.")
    print("Later date = max(creation, modification). Every observed date has its own row.")
    print("This is surviving data, not historical growth or last-use tracking.")
    print("Copied or extracted files can preserve both timestamps; age does not prove disuse.")
    totals = []
    for label in ("target", ".agent/tmp"):
        usage = scan(REPO_ROOT / label, REPO_ROOT)
        totals.append(usage)
        print(render(label, usage))
    print(f"\nCombined: {sum(usage.size for usage in totals) / GIB:.2f} GiB")
    print("\nOptional cleanup commands, from the repository root with builds/tests stopped:")
    print("  cargo clean --target-dir target --dry-run")
    print("  cargo clean --target-dir target")
    print("  cargo clean --target-dir target/llvm-cov-target  # Coverage cache only")
    print("For .agent/tmp, select completed-task directories after reviewing their contents.")
    print("Cleanup requires a separate user decision; this report never deletes files.")
    return int(any(usage.errors or usage.skipped for usage in totals))


if __name__ == "__main__":
    raise SystemExit(main())

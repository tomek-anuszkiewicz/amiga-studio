"""Publish completed Markdown and assets without replacing an existing book."""

from pathlib import Path
import os
import shutil
import tempfile


def book_directory(staged_path):
    path = Path(staged_path).resolve()
    for candidate in (path, *path.parents):
        if candidate.name.endswith("-tmp") and candidate.name != "-tmp":
            return candidate.with_name(candidate.name[:-4])
    raise ValueError("Publishing requires a source/workspace inside a <book>-tmp directory")


def check_destination(destination):
    destination = Path(destination)
    if destination.is_symlink() or destination.resolve() != destination.absolute():
        raise FileExistsError(f"Publication destination must not be a link: {destination}")
    if destination.exists() and (not destination.is_dir() or any(destination.iterdir())):
        raise FileExistsError(f"Publication destination is not an empty directory: {destination}")


def publish_output(output, destination):
    """Prepare a complete copy, then rename it into an absent or empty book directory."""
    output, destination = Path(output), Path(destination)
    check_destination(destination)
    markdown = sorted(output.glob("*.md"))
    if not markdown:
        raise ValueError("No completed Markdown to publish")
    with tempfile.TemporaryDirectory(prefix=".reference-publish-", dir=destination.parent) as temporary:
        prepared = Path(temporary) / "book"
        prepared.mkdir()
        for document in markdown:
            shutil.copy2(document, prepared / document.name)
        if (output / "assets").exists():
            shutil.copytree(output / "assets", prepared / "assets")
        # Recheck after copying; rmdir/rename also refuse a newly occupied directory.
        check_destination(destination)
        if destination.exists():
            destination.rmdir()
        os.rename(prepared, destination)

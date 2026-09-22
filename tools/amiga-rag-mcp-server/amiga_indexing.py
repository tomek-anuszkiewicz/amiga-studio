"""Amiga project indexing scopes exposed by the RAG MCP server."""

from pathlib import Path
from typing import List


AMIGA_SOURCE = "amiga"


def build_index_commands(repository_root: Path, force: bool = False) -> List[List[str]]:
    """Build the CLI calls that index every supported Amiga documentation scope."""
    amiga_root = repository_root / "Obsidian" / "Amiga"
    commands = [
        [
            str(repository_root),
            "--source",
            AMIGA_SOURCE,
            "--include-dirs",
            "docs",
        ],
        [
            str(amiga_root),
            "--source",
            AMIGA_SOURCE,
            "--include-dirs",
            "Design",
            "Reference",
        ],
    ]
    if force:
        for command in commands:
            command.append("--reindex")
    return commands

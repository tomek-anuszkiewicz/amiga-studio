"""CLI argument builders used by the Amiga RAG MCP adapter."""

from pathlib import Path
from typing import List, Optional, Union


AMIGA_SOURCE = "amiga"


def build_index_commands(repository_root: Path, index_json: str) -> List[List[str]]:
    """Build the recursive Markdown indexing calls required by ``rag_qdrant``."""
    return [
        [
            str(repository_root / "docs"),
            "--source",
            AMIGA_SOURCE,
            "--index-json",
            index_json,
        ],
        [
            str(repository_root / "Obsidian" / "Amiga"),
            "--source",
            AMIGA_SOURCE,
            "--index-json",
            index_json,
        ],
    ]


def build_search_command(
    query: str,
    sources: Optional[Union[List[str], str]],
    limit: int,
    index_json: str,
) -> List[str]:
    """Build one CLI search using its comma-separated source-tag filter."""
    if isinstance(sources, str):
        source_tags = [sources.strip()] if sources.strip() else []
    elif isinstance(sources, list):
        source_tags = [source.strip() for source in sources if isinstance(source, str) and source.strip()]
    else:
        source_tags = []

    command = ["search", query, "--index-json", index_json]
    unique_source_tags = list(dict.fromkeys(source_tags))
    if unique_source_tags:
        command.extend(["--source", ",".join(unique_source_tags)])
    command.extend(["--limit", str(limit), "--json"])
    return command

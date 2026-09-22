"""CLI argument builders used by the Amiga RAG MCP adapter."""

from pathlib import Path
from typing import Any, List, Optional, Union


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


def build_search_commands(
    query: str,
    sources: Optional[Union[List[str], str]],
    limit: int,
) -> List[List[str]]:
    """Build one valid CLI search for every requested source tag.

    ``rag_qdrant`` accepts one ``--source NAME`` option per search.  The MCP
    surface accepts a list for convenience, so a multi-source request becomes
    multiple CLI invocations rather than a non-existent comma-separated tag.
    """
    if isinstance(sources, str):
        source_tags = [sources.strip()] if sources.strip() else []
    elif isinstance(sources, list):
        source_tags = [source.strip() for source in sources if isinstance(source, str) and source.strip()]
    else:
        source_tags = []

    unique_source_tags = list(dict.fromkeys(source_tags))
    commands = []
    for source_tag in unique_source_tags or [None]:
        command = ["search", query, "--limit", str(limit), "--json"]
        if source_tag is not None:
            command.extend(["--source", source_tag])
        commands.append(command)
    return commands


def combine_search_results(responses: List[Any], limit: int) -> List[dict]:
    """Merge per-source CLI responses while preserving the MCP result limit."""
    hits = [
        hit
        for response in responses
        if isinstance(response, list)
        for hit in response
        if isinstance(hit, dict)
    ]
    return sorted(hits, key=lambda hit: hit.get("score", 0.0), reverse=True)[:limit]

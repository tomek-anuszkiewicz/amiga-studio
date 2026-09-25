"""JSON command adapter for the Amiga RAG MCP server."""

import json
import os
import subprocess
from typing import Any, Callable, List, Optional, Union


RAG_QDRANT_COMMAND = "rag_qdrant.bat" if os.name == "nt" else "rag_qdrant"


class RagQdrantCommandError(RuntimeError):
    """Raised when the public RAG command cannot complete successfully."""


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


def run_rag_qdrant(
    arguments: List[str],
    runner: Callable[..., Any] = subprocess.run,
    timeout_seconds: int = 120,
) -> str:
    """Run the PATH-resolved CLI and return its standard output."""
    try:
        completed = runner(
            [RAG_QDRANT_COMMAND, *arguments],
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
            check=False,
        )
    except FileNotFoundError as error:
        raise RagQdrantCommandError(
            f"'{RAG_QDRANT_COMMAND}' was not found on PATH."
        ) from error
    except subprocess.TimeoutExpired as error:
        raise RagQdrantCommandError(
            f"rag_qdrant did not respond within {timeout_seconds} seconds."
        ) from error

    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip() or "unknown error"
        raise RagQdrantCommandError(f"rag_qdrant failed: {detail}")

    return completed.stdout


def run_rag_qdrant_json(
    arguments: List[str],
    runner: Callable[..., Any] = subprocess.run,
) -> Any:
    """Run the PATH-resolved CLI and decode its JSON response."""
    try:
        return json.loads(run_rag_qdrant(arguments, runner=runner))
    except json.JSONDecodeError as error:
        raise RagQdrantCommandError("rag_qdrant returned invalid JSON.") from error

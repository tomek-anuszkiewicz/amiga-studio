"""FastMCP adapter for the PATH-resolved ``rag_qdrant`` CLI."""

import os
from pathlib import Path
from typing import List, Optional, Union

from fastmcp import FastMCP

from amiga_indexing import build_index_commands, build_search_command
from environment import load_environment_file
from rag_qdrant_command import RagQdrantCommandError, run_rag_qdrant, run_rag_qdrant_json


mcp = FastMCP("amiga-rag")
PROJECT_ROOT = Path(__file__).resolve().parents[2]
RAG_INDEX_JSON_VARIABLE = "RAG_INDEX_JSON"
load_environment_file(PROJECT_ROOT / ".env")


def configured_index_json() -> Optional[str]:
    """Return the CLI state file configured for this shared collection."""
    value = os.environ.get(RAG_INDEX_JSON_VARIABLE, "").strip()
    return value or None


def response_error(response: object) -> Optional[str]:
    """Extract the documented JSON error response emitted by the CLI."""
    if isinstance(response, dict) and isinstance(response.get("error"), str):
        return response["error"]
    return None


@mcp.tool()
def rag_search(
    query: str,
    sources: Optional[Union[List[str], str]] = None,
    limit: int = 5,
) -> str:
    """Search documentation in the shared RAG collection by source tag."""
    index_json = configured_index_json()
    if index_json is None:
        return "RAG commands require RAG_INDEX_JSON to name the shared CLI state file."

    try:
        results = run_rag_qdrant_json(build_search_command(query, sources, limit, index_json))
    except RagQdrantCommandError as error:
        return f"Error during RAG search: {error}"

    error = response_error(results)
    if error:
        return f"Error during RAG search: {error}"
    if not isinstance(results, list):
        return "Error during RAG search: rag_qdrant returned an unexpected JSON response."

    if not results:
        return f"No relevant documentation found for query: '{query}'."

    output_lines = [f"Found {len(results)} relevant section(s) in RAG database:\n"]
    for index, hit in enumerate(results, 1):
        output_lines.append(
            f"--- [Result {index} | Score: {hit.get('score', 0.0)} | "
            f"Source: {hit.get('source', 'unknown')}] ---"
        )
        output_lines.append(f"File: {hit.get('file_path', '')}")
        output_lines.append(f"Header: {hit.get('header', 'Untitled Section')}")
        images = hit.get("images", [])
        if images:
            output_lines.append(f"Associated Diagram(s): {', '.join(images)}")
        output_lines.append(f"\n{hit.get('content', '').strip()}\n")
    return "\n".join(output_lines)


@mcp.tool()
def rag_list_sources() -> str:
    """List source tags with cached file and vector counts."""
    index_json = configured_index_json()
    if index_json is None:
        return "RAG commands require RAG_INDEX_JSON to name the shared CLI state file."

    try:
        sources = run_rag_qdrant_json(["--list-sources", "--index-json", index_json, "--json"])
    except RagQdrantCommandError as error:
        return f"Error listing RAG sources: {error}"

    error = response_error(sources)
    if error:
        return f"Error listing RAG sources: {error}"
    if not isinstance(sources, list):
        return "Error listing RAG sources: rag_qdrant returned an unexpected JSON response."

    if not sources:
        return "No sources currently indexed in Qdrant RAG database."

    lines = [
        "### Indexed Knowledge Sources in RAG Database\n",
        "| Source Tag | Files Indexed | Chunks/Vectors | Last Updated |",
        "| :--- | :---: | :---: | :--- |",
    ]
    for source in sources:
        lines.append(
            f"| `{source['source']}` | {source['files_count']} | "
            f"{source['chunks_count']} | {source['last_updated']} |"
        )
    return "\n".join(lines)


@mcp.tool()
def rag_status() -> str:
    """Report Qdrant connectivity and the active collection status."""
    index_json = configured_index_json()
    if index_json is None:
        return "RAG commands require RAG_INDEX_JSON to name the shared CLI state file."

    try:
        status = run_rag_qdrant_json(["--status", "--index-json", index_json, "--json"])
    except RagQdrantCommandError as error:
        return f"Qdrant connection error: {error}"

    error = response_error(status)
    if error:
        return f"Qdrant connection error: {error}"
    if not isinstance(status, dict):
        return "Qdrant connection error: rag_qdrant returned an unexpected JSON response."

    return (
        f"**Qdrant RAG Status**: Connected (`{status.get('qdrant_url', 'unknown')}`)\n"
        f"- Collection: `{status.get('collection', 'projects_docs')}`\n"
        f"- Health Status: `{status.get('health_status', 'unknown')}`\n"
        f"- Total Stored Vectors: `{status.get('total_vectors', 0)}\n"
        f"- Total Tracked Files: `{status.get('total_files', 0)} across "
        f"{status.get('source_count', 0)} source(s)."
    )


@mcp.tool()
def rag_reindex() -> str:
    """Incrementally index docs, Design, and Reference under source ``amiga``."""
    index_json = configured_index_json()
    if index_json is None:
        return "RAG commands require RAG_INDEX_JSON to name the shared CLI state file."

    try:
        outputs = [
            run_rag_qdrant(command, timeout_seconds=1800)
            for command in build_index_commands(PROJECT_ROOT, index_json)
        ]
    except RagQdrantCommandError as error:
        return f"Error indexing Amiga RAG documentation: {error}"

    output = "\n".join(line for result in outputs for line in result.splitlines() if line.strip())
    if not output:
        return "Amiga RAG incremental index completed for docs, Design, and Reference."
    return f"Amiga RAG incremental index completed for docs, Design, and Reference.\n{output}"


if __name__ == "__main__":
    mcp.run()

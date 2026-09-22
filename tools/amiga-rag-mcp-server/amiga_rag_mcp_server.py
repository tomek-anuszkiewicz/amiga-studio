"""FastMCP adapter for the PATH-resolved ``rag_qdrant`` CLI."""

from pathlib import Path
from typing import List, Optional, Union

from fastmcp import FastMCP

from amiga_indexing import build_index_commands, build_search_commands, combine_search_results
from rag_qdrant_command import RagQdrantCommandError, run_rag_qdrant, run_rag_qdrant_json


mcp = FastMCP("amiga-rag")
PROJECT_ROOT = Path(__file__).resolve().parents[2]


@mcp.tool()
def rag_search(
    query: str,
    sources: Optional[Union[List[str], str]] = None,
    limit: int = 5,
) -> str:
    """Search documentation in the shared RAG collection by source tag."""
    try:
        responses = [
            run_rag_qdrant_json(arguments)
            for arguments in build_search_commands(query, sources, limit)
        ]
    except RagQdrantCommandError as error:
        return f"Error during RAG search: {error}"

    results = combine_search_results(responses, limit)

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
    try:
        sources = run_rag_qdrant_json(["--list-sources", "--json"])
    except RagQdrantCommandError as error:
        return f"Error listing RAG sources: {error}"

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
    try:
        status = run_rag_qdrant_json(["--status", "--json"])
    except RagQdrantCommandError as error:
        return f"Qdrant connection error: {error}"

    return (
        f"**Qdrant RAG Status**: Connected (`{status['qdrant_url']}`)\n"
        f"- Collection: `{status['collection']}`\n"
        f"- Health Status: `{status['health_status']}`\n"
        f"- Total Stored Vectors: `{status['total_vectors']}`\n"
        f"- Total Tracked Files: `{status['total_files']}` across "
        f"{status['source_count']} source(s)."
    )


@mcp.tool()
def rag_reindex(force: bool = False) -> str:
    """Index docs, Design, and Reference under source ``amiga``.

    Set ``force`` to true to rebuild all files rather than using the shared
    SHA-256 cache.
    """
    try:
        outputs = [
            run_rag_qdrant(command, timeout_seconds=1800)
            for command in build_index_commands(PROJECT_ROOT, force=force)
        ]
    except RagQdrantCommandError as error:
        return f"Error indexing Amiga RAG documentation: {error}"

    mode = "forced reindex" if force else "incremental index"
    output = "\n".join(line for result in outputs for line in result.splitlines() if line.strip())
    if not output:
        return f"Amiga RAG {mode} completed for docs, Design, and Reference."
    return f"Amiga RAG {mode} completed for docs, Design, and Reference.\n{output}"


if __name__ == "__main__":
    mcp.run()

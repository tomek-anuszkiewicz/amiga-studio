"""
FastMCP Server for Amiga RAG & Obsidian Knowledge Base.
Exposes semantic search, source listing, and status to Antigravity (AGY).
"""

import sys
from pathlib import Path
from typing import Optional, List, Union

# Ensure tools directory is in sys.path
tools_dir = str(Path(__file__).parent.resolve())
if tools_dir not in sys.path:
    sys.path.insert(0, tools_dir)

from fastmcp import FastMCP
from rag_qdrant.indexer import KnowledgeIndexer
from rag_qdrant.config import QDRANT_URL, COLLECTION_NAME

mcp = FastMCP("amiga-rag")
indexer = KnowledgeIndexer()


@mcp.tool()
def rag_search(
    query: str,
    sources: Optional[Union[List[str], str]] = None,
    limit: int = 5
) -> str:
    """
    Search the local RAG knowledge base for technical documentation, hardware specs, and Obsidian notes.

    Args:
        query: The semantic search query (e.g. 'Copper list wait instruction timing', 'Blitter line mode algorithm').
        sources: Optional list of sources (e.g. ['amiga', 'obsidian']) or a single source string. Defaults to searching all sources.
        limit: Maximum number of chunks to return (default: 5).

    Returns:
        Formatted search results including source tags, file paths, section headers, text contents, and any image references.
    """
    try:
        source_list = None
        if isinstance(sources, str):
            source_list = [s.strip() for s in sources.split(",") if s.strip()]
        elif isinstance(sources, list):
            source_list = sources

        results = indexer.search(query=query, sources=source_list, limit=limit)
        if not results:
            return f"No relevant documentation found for query: '{query}'."

        output_lines = [f"Found {len(results)} relevant section(s) in RAG database:\n"]
        for i, hit in enumerate(results, 1):
            source_tag = hit.get("source", "unknown")
            header = hit.get("header", "Untitled Section")
            file_path = hit.get("file_path", "")
            content = hit.get("content", "").strip()
            score = hit.get("score", 0.0)
            images = hit.get("images", [])

            output_lines.append(f"--- [Result {i} | Score: {score} | Source: {source_tag}] ---")
            output_lines.append(f"File: {file_path}")
            output_lines.append(f"Header: {header}")
            if images:
                output_lines.append(f"Associated Diagram(s): {', '.join(images)}")
            output_lines.append(f"\n{content}\n")

        return "\n".join(output_lines)
    except Exception as e:
        return f"Error during RAG search: {e}"


@mcp.tool()
def rag_list_sources() -> str:
    """
    Lists all indexed knowledge sources in the Qdrant RAG database with file and vector counts.
    """
    try:
        sources = indexer.get_sources_stats()
        if not sources:
            return "No sources currently indexed in Qdrant RAG database."

        lines = [
            "### Indexed Knowledge Sources in RAG Database\n",
            "| Source Tag | Files Indexed | Chunks/Vectors | Last Updated |",
            "| :--- | :---: | :---: | :--- |"
        ]
        for s in sources:
            lines.append(f"| `{s['source']}` | {s['files_count']} | {s['chunks_count']} | {s['last_updated']} |")
        return "\n".join(lines)
    except Exception as e:
        return f"Error listing RAG sources: {e}"


@mcp.tool()
def rag_status() -> str:
    """
    Checks connection to Qdrant vector database and reports total vector count.
    """
    try:
        indexer.ensure_collection()
        info = indexer.client.get_collection(COLLECTION_NAME)
        sources = indexer.get_sources_stats()
        total_files = sum(s["files_count"] for s in sources)
        return (
            f"**Qdrant RAG Status**: Connected (`{QDRANT_URL}`)\n"
            f"- Collection: `{COLLECTION_NAME}`\n"
            f"- Health Status: `{info.status}`\n"
            f"- Total Stored Vectors: `{info.points_count}`\n"
            f"- Total Tracked Files: `{total_files}` across {len(sources)} source(s)."
        )
    except Exception as e:
        return f"Qdrant connection error: {e}"


if __name__ == "__main__":
    mcp.run()

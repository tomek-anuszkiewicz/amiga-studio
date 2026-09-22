#!/usr/bin/env python3
"""Retired direct-search entry point that directs callers to the Amiga RAG MCP."""

import sys
import argparse

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

def main():
    parser = argparse.ArgumentParser(
        description="Show the Amiga project's supported RAG retrieval interface."
    )
    parser.add_argument("query", help="Query text to use with the Amiga RAG MCP tool")
    parser.add_argument(
        "-s", "--source",
        choices=["amiga", "devnotes", "all"],
        default="all",
        help="Knowledge source to pass to the MCP tool"
    )
    parser.add_argument(
        "-l", "--limit",
        type=int,
        default=2,
        help="Maximum result count to pass to the MCP tool"
    )
    parser.add_argument(
        "--full",
        action="store_true",
        help="Retained for command-line compatibility"
    )

    args = parser.parse_args()

    source = None if args.source == "all" else args.source
    limit = min(max(args.limit, 1), 5)
    print("[RAG] Direct project-side search is retired.")
    print(
        "Use the Amiga RAG MCP tool: "
        f"rag_search(query={args.query!r}, sources={([source] if source else None)!r}, limit={limit})."
    )
    return 1

if __name__ == "__main__":
    raise SystemExit(main())

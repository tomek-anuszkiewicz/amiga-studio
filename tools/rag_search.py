#!/usr/bin/env python3
"""
Fast CLI Search Tool for Local Amiga RAG (Qdrant)
Provides rapid, token-efficient semantic lookup across official Amiga manuals,
chip specifications, and Obsidian architecture notes.
"""

import os
import sys
import argparse
from pathlib import Path

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

REPO_ROOT = Path(__file__).resolve().parent.parent
RAG_DIR = REPO_ROOT / "tools" / "rag"

if str(RAG_DIR) not in sys.path:
    sys.path.insert(0, str(RAG_DIR))

def main():
    parser = argparse.ArgumentParser(
        description="Fast semantic search across Amiga hardware docs and Obsidian design specs."
    )
    parser.add_argument("query", help="Search query (e.g. 'DMACON bit 10', 'Copper WAIT timing')")
    parser.add_argument(
        "-s", "--source",
        choices=["amiga", "obsidian", "all"],
        default="all",
        help="Knowledge source to search (default: all)"
    )
    parser.add_argument(
        "-l", "--limit",
        type=int,
        default=2,
        help="Maximum results to return (default: 2, max: 5)"
    )
    parser.add_argument(
        "--full",
        action="store_true",
        help="Print full section content without truncation"
    )

    args = parser.parse_args()

    try:
        from rag_qdrant.indexer import KnowledgeIndexer
        from rag_qdrant.config import QDRANT_URL
    except ImportError as e:
        print(f"[RAG Error] Failed to load RAG dependencies: {e}")
        sys.exit(1)

    try:
        indexer = KnowledgeIndexer()
        sources = None if args.source == "all" else [args.source]
        limit = min(max(args.limit, 1), 5)

        hits = indexer.search(query=args.query, sources=sources, limit=limit)
        if not hits:
            print(f"[RAG] No relevant documentation found for: '{args.query}'")
            sys.exit(0)

        print(f">> [RAG Search] Found {len(hits)} match(es) for: '{args.query}'\n")

        for idx, hit in enumerate(hits, 1):
            score = hit.get("score", 0.0)
            src = hit.get("source", "unknown")
            header = hit.get("header", "Untitled")
            rel_path = hit.get("relative_path") or hit.get("file_path", "")
            content = hit.get("content", "").strip()

            print(f"[{idx}] Score: {score:.3f} | Source: {src} | Section: {header}")
            print(f"    Path: {rel_path}")
            print("    " + "-" * 70)

            lines = content.splitlines()
            if not args.full and len(lines) > 12:
                snippet = "\n    ".join(lines[:12])
                print(f"    {snippet}\n    [... truncated {len(lines) - 12} lines. Use --full for complete section ...]")
            else:
                snippet = "\n    ".join(lines)
                print(f"    {snippet}")

            images = hit.get("images", [])
            if images:
                print(f"    Images/Diagrams: {', '.join(images)}")
            print()

    except Exception as e:
        print(f"[RAG Error] Query failed (check if Qdrant is running on {QDRANT_URL}): {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()

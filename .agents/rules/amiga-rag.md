---
trigger: always_on
description: >
  Amiga RAG knowledge base tools, mandatory pre-task conceptual retrieval,
  RAG vs Graphify routing, and prohibition of direct manual crawling.
  Reindexing procedures live in the index-amiga-rag skill.
---

## Amiga RAG Knowledge Base (Amiga Docs + Obsidian)

Tools available: `rag_search`, `rag_list_sources`, `rag_status` (MCP) and
`python tools/harness/rag_search.py` (CLI).

**Two sources in the `amiga` Qdrant collection:**
- `amiga` — Commodore hardware reference manuals, chip specs, M68000 PRM.
- `obsidian` — Architecture guidelines, design philosophy, personal research notes.

## Mandatory Pre-Task Retrieval

Before writing code or making architectural decisions on any non-trivial task:
1. Query the obsidian source for architectural context:
   `rag_search(query="<task-topic>", sources=["obsidian"])`
2. For hardware chipsets (Agnus/Denise/Paula/CIA), query both sources.
3. **Graceful fallback:** if Qdrant is offline, proceed from repo specs — never invent citations.

## RAG vs Graphify Routing

- **Graphify** (`graphify query/path/explain`): code structure, call hierarchies, AST, symbol relationships within this repo.
- **RAG** (`rag_search` / `rag_search.py`): hardware specs, register definitions, chip timing, AmigaOS APIs, personal design notes.
- **Both**: when implementing a feature — RAG first for the hardware spec, then Graphify to locate the corresponding code.

## Zero Raw Manual Scanning

Never open large files under `Obsidian/Amiga/Reference/` directly via `view_file` or
run wide `grep_search` across manuals without first running `rag_search`. Use `view_file`
only on the specific section identified by RAG.

## Reindexing

Full step-by-step reindexing procedure (trigger matrix, asset sidecar audit, smoke-test
query, troubleshooting) → [`index-amiga-rag`](../skills/index-amiga-rag/SKILL.md) skill.

Quick reference:
```powershell
.\tools\rag\bin\amiga_rag.ps1 . --source amiga      # hardware manuals
.\tools\rag\bin\amiga_rag.ps1 <PATH_TO_VAULT> --source obsidian  # design notes
```

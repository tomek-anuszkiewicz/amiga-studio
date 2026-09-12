# Amiga RAG & Knowledge Base Tools

This directory contains the ingestion pipeline, indexing scripts, CLI utilities, and the FastMCP server for the local Amiga & Obsidian RAG (Retrieval-Augmented Generation) system.

---

## 1. Quick Overview

- **CLI Indexer**: `bin/amiga_rag.ps1` (or `python -m rag_qdrant.cli`)
- **FastMCP Server**: `rag_mcp_server.py` (registers `rag_search`, `rag_list_sources`, `rag_status` for AI agents)
- **Vector Database**: Runs in Docker on `http://localhost:6333` (Collection: `amiga`, unified sources: `amiga` and `obsidian` for general knowledge)
- **Hash Cache**: Configured via `RAG_CACHE_FILE` in `.env`
- **Configuration**: Loaded from `rag_qdrant/config.py` and repository `.env` (`.env`)

---

## 2. Setup & Installation

Install Python dependencies:

```powershell
pip install -r tools\rag\requirements.txt
```

Dependencies:
- `qdrant-client>=1.13.0`
- `fastembed>=0.4.0` (or `google-genai>=1.0.0` for Gemini cloud embeddings)
- `fastmcp>=0.4.0`
- `pillow>=10.0.0`
- `python-dotenv>=1.0.1`
- `rich>=13.0.0`

### Environment Configuration (`.env`)

Configure your environment in `.env` (repository root):

```env
GEMINI_API_KEY=your_gemini_api_key
RAG_CACHE_FILE=<PATH_TO_CACHE_DIR>\amiga_rag_cache.json
```

---

## 3. CLI Usage: `amiga_rag`

You can invoke the indexer using the helper PowerShell script:

```powershell
# Check database status and connection
.\tools\rag\bin\amiga_rag.ps1 --status

# List indexed knowledge sources and vector counts
.\tools\rag\bin\amiga_rag.ps1 --list-sources

# Index Amiga technical documentation in this repository (tagged as 'amiga')
.\tools\rag\bin\amiga_rag.ps1 . --source amiga

# Index Obsidian notes (tagged as 'obsidian' for general knowledge)
.\tools\rag\bin\amiga_rag.ps1 <PATH_TO_VAULT> --source obsidian

# Force re-index of all files (ignores SHA256 cache)
.\tools\rag\bin\amiga_rag.ps1 . --source amiga --reindex
```

### Privacy & Ignored Files (`.ragignore`)

The indexer automatically respects `.ragignore` files placed in the target directory (e.g. `<PATH_TO_VAULT>/.ragignore`), excluding private directories (`_Private/`, `*private*`) and system folders (`.obsidian`, `.smart-env`, `.agents`, `.antigravity`, `.git`).

Options for fine-grained ingestion control:
- `-e, --exclude PATTERNS`: Substring or glob patterns to exclude from scanning.
- `--include-dirs DIRS`: Whitelist specific top-level subdirectories (e.g. `01* 02*`).
- `--no-root-notes`: Skip standalone markdown files at the root of the indexed folder.
```

---

## 4. Antigravity FastMCP Server Integration

The server script is located at [`tools/rag/rag_mcp_server.py`](rag_mcp_server.py).
It exposes 3 tools to Antigravity:
1. `rag_search(query, sources=["amiga", "obsidian"], limit=5)`: Semantic vector search across specified sources.
2. `rag_list_sources()`: Returns list of active sources and document counts.
3. `rag_status()`: Checks Qdrant vector database health and point count.

The server is configured at the project level in `.agents/mcp_config.json`:
```json
{
  "mcpServers": {
    "amiga-rag": {
      "command": "python",
      "args": ["tools/rag/rag_mcp_server.py"],
      "env": {
        "PYTHONPATH": "tools/rag"
      }
    }
  }
}
```

---

## 5. File Structure

```
tools/rag/
├── README.md                 # This documentation
├── requirements.txt          # Python dependencies
├── rag_mcp_server.py         # FastMCP server for Antigravity AI
├── bin/
│   └── amiga_rag.ps1         # CLI PowerShell launcher
└── rag_qdrant/               # Core Python package
    ├── __init__.py
    ├── config.py             # Config & .env loading
    ├── chunker.py            # Hierarchical markdown chunking
    ├── indexer.py            # Qdrant client & SHA256 cache management
    ├── vision.py             # Gemini Flash diagram OCR & captioning
    └── cli.py                # Command-line interface logic
```

# Amiga RAG & Knowledge Base Tools

This directory contains the ingestion pipeline, indexing scripts, CLI utilities, and the FastMCP server for the local Amiga & Obsidian RAG (Retrieval-Augmented Generation) system.

---

## 1. Quick Overview

- **CLI Indexer**: `bin/rag-qdrant-index.bat` (or `python -m rag_qdrant.cli`)
- **FastMCP Server**: `rag_mcp_server.py` (registers `rag_search`, `rag_list_sources`, `rag_status` for AI agents)
- **Vector Database**: Runs in Docker on `http://localhost:6333` (hosted at `D:\GoogleDrive\AI\qdrant`)
- **Hash Cache**: `D:\GoogleDrive\AI\qdrant\rag_cache.json` (incremental indexing, only re-indexes modified files)
- **Configuration**: Loaded from `rag_qdrant/config.py` and repository `.env` (`D:\Programowanie\Amiga\.env`)

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

---

## 3. CLI Usage: `rag-qdrant-index`

You can invoke the indexer using the helper batch script:

```powershell
# Check database status and connection
.\tools\rag\bin\rag-qdrant-index.bat --status

# List indexed knowledge sources and vector counts
.\tools\rag\bin\rag-qdrant-index.bat --list-sources

# Index Amiga technical documentation in this repository (tagged as 'amiga')
.\tools\rag\bin\rag-qdrant-index.bat D:\Programowanie\Amiga --source amiga

# Index Obsidian notes (tagged as 'obsidian')
.\tools\rag\bin\rag-qdrant-index.bat D:\GoogleDrive\AI\Obsidian --source obsidian

# Force re-index of all files (ignores SHA256 cache)
.\tools\rag\bin\rag-qdrant-index.bat D:\Programowanie\Amiga --source amiga --reindex
```

---

## 4. Antigravity FastMCP Server Integration

The server script is located at [`tools/rag/rag_mcp_server.py`](rag_mcp_server.py).
It exposes 3 tools to Antigravity:
1. `rag_search(query, sources=["amiga", "obsidian"], limit=5)`: Semantic vector search across specified sources.
2. `rag_list_sources()`: Returns list of active sources and document counts.
3. `rag_status()`: Checks Qdrant vector database health and point count.

The server is configured in `C:\Users\Tomek\.gemini\config\mcp_config.json`:
```json
{
  "mcpServers": {
    "amiga-rag": {
      "command": "python",
      "args": ["D:\\Programowanie\\Amiga\\tools\\rag\\rag_mcp_server.py"],
      "env": {
        "PYTHONPATH": "D:\\Programowanie\\Amiga\\tools\\rag"
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
│   └── rag-qdrant-index.bat  # CLI batch launcher
└── rag_qdrant/               # Core Python package
    ├── __init__.py
    ├── config.py             # Config & .env loading
    ├── chunker.py            # Hierarchical markdown chunking
    ├── indexer.py            # Qdrant client & SHA256 cache management
    ├── vision.py             # Gemini Flash diagram OCR & captioning
    └── cli.py                # Command-line interface logic
```

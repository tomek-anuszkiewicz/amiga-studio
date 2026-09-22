# Amiga RAG MCP Server

This project-specific FastMCP server exposes Amiga documentation retrieval and scoped indexing through `rag_search`, `rag_list_sources`, `rag_status`, and `rag_reindex`.

It does not import Qdrant, its cache, or the indexer. It invokes the standalone
`rag_qdrant` command from `PATH` for every search, status, source-list, and
index operation, so the CLI and its dependencies must be installed independently.

## Installation

```powershell
pip install -r requirements.txt
```

Ensure `rag_qdrant` is available on `PATH`, then register the server in the Amiga project:

```json
{
  "mcpServers": {
    "amiga-rag": {
      "command": "python",
      "args": ["tools/amiga-rag-mcp-server/amiga_rag_mcp_server.py"],
      "env": {
        "PYTHONPATH": "tools/amiga-rag-mcp-server"
      }
    }
  }
}
```

`rag_reindex()` invokes `rag_qdrant` only for the Amiga documentation scopes:

- repository `docs`
- `Obsidian/Amiga/Design`
- `Obsidian/Amiga/Reference`

It uses the `amiga` source tag and lets the CLI reuse its cache by default. Call
`rag_reindex(force=true)` to pass `--reindex` to both CLI calls and rebuild the
scoped documents.

When the MCP `rag_search` tool receives multiple source tags, it invokes one
valid `rag_qdrant search ... --source NAME --json` command per tag and combines
the returned hits. This mirrors the CLI's single-source option rather than
inventing a comma-separated source syntax.

The MCP server is the RAG integration point for agent retrieval and reindexing.
For operator-driven setup, `tools/bootstrap.ps1 -Rag` invokes the same
PATH-resolved command with these same two Amiga scopes.

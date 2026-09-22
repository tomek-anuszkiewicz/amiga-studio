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

Create the ignored project `.env` file with the CLI state-file location shared
by every command operating on `projects_docs`:

```dotenv
RAG_INDEX_JSON=<shared-rag-index-state-file>
```

`rag_reindex()` invokes `rag_qdrant` only for the Amiga documentation scopes:

- repository `docs`
- Markdown below `Obsidian/Amiga`, including Design and Reference while the CLI excludes `.obsidian` and other technical directories

It uses the `amiga` source tag and passes `RAG_INDEX_JSON` to every invocation.
The CLI hashes every Markdown file on each run and has no forced-reindex mode.

When the MCP `rag_search` tool receives multiple source tags, it passes the
deduplicated tags as the CLI's supported comma-separated `--source` value.

The MCP server is the RAG integration point for agent retrieval and reindexing.
For operator-driven setup, `tools/bootstrap.ps1 -Rag` invokes the same
PATH-resolved command with these same two Amiga scopes.

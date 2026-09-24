# Amiga RAG MCP Server

This project-specific FastMCP server exposes Amiga documentation retrieval and health inspection through `rag_search`, `rag_list_sources`, and `rag_status`.

It does not import Qdrant, its cache, or the indexer. It invokes the standalone
`rag_qdrant` command from `PATH` for query execution and status inspection, so the CLI and its dependencies must be installed independently.

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

## Documentation Indexing

Documentation indexing is a heavy batch process handled strictly outside the MCP server through the `rag_qdrant` CLI or `tools/bootstrap.ps1 -Rag` across the Amiga documentation scopes:

- repository `docs`
- `Obsidian/Amiga/Design`
- `Obsidian/Amiga/Reference`

The CLI uses the `amiga` source tag and passes `RAG_INDEX_JSON` to every invocation. It hashes every Markdown file on each run and processes only modified or added files.

When the MCP `rag_search` tool receives multiple source tags, it passes the
deduplicated tags as the CLI's supported comma-separated `--source` value.

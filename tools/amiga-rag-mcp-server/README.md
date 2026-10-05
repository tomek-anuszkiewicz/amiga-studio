# Amiga RAG MCP Server

This project-specific FastMCP server exposes Amiga documentation retrieval and health inspection through `rag_search`, `rag_list_sources`, and `rag_status`.

It does not import Qdrant, its cache, or the indexer. It invokes the standalone
`rag_qdrant` command from `PATH` for query execution and status inspection, so the CLI and its dependencies must be installed independently.

## Installation

```powershell
pip install -r requirements.txt
```

Ensure `rag_qdrant` is available on `PATH`, then register the server in the Amiga project:

Use the checked-in [`.codex/config.toml`](../../.codex/config.toml):

```toml
[mcp_servers.amiga-rag]
command = "python"
args = ["tools/amiga-rag-mcp-server/amiga_rag_mcp_server.py"]

[mcp_servers.amiga-rag.env]
PYTHONPATH = "tools/amiga-rag-mcp-server"
```

Open the repository root as the Codex session's working directory. Omit `cwd`
so the server inherits that session directory and resolves the script path and
`PYTHONPATH` from it. In the tested Codex CLI 0.160.0, an explicit relative MCP
`cwd` resolves from the host process directory, so `cwd = ".."` does not reliably
select the repository root. Project configuration loads only for trusted
projects. See [Codex configuration](https://learn.chatgpt.com/docs/config-file/config-advanced)
and [MCP setup](https://learn.chatgpt.com/docs/extend/mcp).

Check the effective registration with `codex mcp get amiga-rag --json`. After
changing it, restart the local Codex client and inspect `/mcp` for the server's
connection status; a configuration listing alone does not test a connection.

The launch regression starts a real Codex app-server from an unrelated host
directory, creates an ephemeral session at the repository root without a model
turn, and verifies the server connects and exposes all three tools:

```powershell
python -m unittest discover -s tools/amiga-rag-mcp-server/tests -p test_codex_mcp_launch.py -q
```

This check requires the Codex CLI on `PATH`, a trusted project, and the server
requirements installed in the configured Python environment. It skips when
Codex or FastMCP is absent. Verify backend connectivity with `rag_status` and a
source-filtered `rag_search` in the refreshed Codex session.

Create the ignored project `.env` file with the CLI state-file location shared
by every command operating on `projects_docs`:

```dotenv
RAG_INDEX_JSON=<shared-rag-index-state-file>
```

## Documentation Indexing

Documentation indexing is a heavy batch process handled strictly outside the MCP server through the `rag_qdrant` CLI or `tools/bootstrap/bootstrap.ps1 -Rag` across the Amiga documentation scopes:

- repository `docs`
- `Obsidian/Amiga/Design`
- `Obsidian/Amiga/Reference`

The CLI uses the `amiga` source tag and passes `RAG_INDEX_JSON` to every invocation. It hashes every Markdown file on each run and processes only modified or added files.

When the MCP `rag_search` tool receives multiple source tags, it passes the
deduplicated tags as the CLI's supported comma-separated `--source` value.

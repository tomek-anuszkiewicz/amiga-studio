# Amiga RAG MCP Server

Clients connect to this project's FastMCP server to search Amiga documentation
with `rag_search` and `rag_search_reference`, list indexed sources with
`rag_list_sources` and check service health with `rag_status`.

`rag_search_reference` queries the shared `amiga` tag and examines at most the
first 100 ranked Amiga hits. It returns only hits whose resolved file paths are
under `Obsidian/Amiga/Reference/` and reports when none qualify. Newly indexed
results use repository-relative paths. The server resolves them against this
checkout to apply the filter. Existing absolute-path results in the shared index
may still point to another checkout until you explicitly rebuild that index.

The adapter runs the standalone `rag_qdrant` command from `PATH` for queries and
status checks. Install the CLI and its dependencies separately. The adapter
imports neither Qdrant, its cache nor the indexer. It decodes CLI output as
UTF-8 to match the CLI's encoding regardless of the host's default locale.

The server uses the records stored in Qdrant, called points, to count sources
and files independently of the incremental JSON cache. It lists only sources
with points. For old points without a stored timestamp, it reports `N/A` as
the indexing date.

## Installation

```powershell
pip install -r requirements.txt
```

Verify that `rag_qdrant` is on `PATH`. Use the checked-in Amiga project
configuration to register the server. [`.codex/config.toml`](../../.codex/config.toml)

```toml
[mcp_servers.amiga-rag]
command = "python"
args = ["tools/amiga-rag-mcp-server/amiga_rag_mcp_server.py"]

[mcp_servers.amiga-rag.env]
PYTHONPATH = "tools/amiga-rag-mcp-server"
```

Use the repository root as the Codex session's working directory. Omit `cwd`
to let the server inherit that directory and resolve the script path and
`PYTHONPATH` from it. In the tested Codex CLI 0.160.0, an explicit relative MCP
`cwd` resolved from the host process directory. As a result, `cwd = ".."` could
select a directory other than the repository root. Codex loads project
configuration only for trusted projects.
[Codex configuration](https://learn.chatgpt.com/docs/config-file/config-advanced)
and [MCP setup](https://learn.chatgpt.com/docs/extend/mcp).

Check which registration Codex uses with `codex mcp get amiga-rag --json`. After
you change the registration, restart the local Codex client. Use `/mcp` to inspect
the server's connection status. The configuration listing shows the registration
settings only.

The launch regression starts a real Codex app-server from an unrelated host
directory. The test creates a temporary session at the repository root without
a model turn. It then checks that the server connects and exposes the required
retrieval and status tools.

```powershell
python tools/verification/verify.py test python directory tools/amiga-rag-mcp-server/tests file test_codex_mcp_launch.py
```

This check requires the Codex CLI on `PATH`, a trusted project, and the server
requirements installed in the configured Python environment. It skips when
Codex or FastMCP is absent. Verify backend connectivity with `rag_status` and a
source-filtered `rag_search` in the refreshed Codex session.

Run all adapter and launch checks from the repository root.

```powershell
python tools/verification/verify.py test python directory tools/amiga-rag-mcp-server/tests --log-level full
```

Read-only MCP tools query the service using its configuration. You need a local
index-state path only for indexing. Set it in the ignored project `.env`.

```dotenv
RAG_INDEX_JSON=<shared-rag-index-state-file>
```

## Documentation Indexing

Documentation indexing is a heavy batch process. Run it outside the MCP server
through the `rag_qdrant` CLI or `./tools/bootstrap/bootstrap.ps1 -Rag` for the
following Amiga documentation directories.

- repository `docs`
- `Obsidian/Amiga/Design`
- `Obsidian/Amiga/Reference`

The CLI uses the `amiga` source tag, an explicit `--repo-root` and the absolute
path to the directory it scans. It needs `RAG_INDEX_JSON` only for indexing.
On each run, it hashes every Markdown file and processes only documents added
or modified at repository-relative paths. Follow the indexing procedure.
[Indexing procedure](../../.agents/skills/TA-RAG-index-amiga/SKILL.md)

When the MCP `rag_search` tool receives multiple source tags, it passes the
deduplicated tags as the CLI's supported comma-separated `--source` value.

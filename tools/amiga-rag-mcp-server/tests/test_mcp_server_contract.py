"""Regression tests for the Amiga RAG MCP command boundary."""

import os
import sys
import tempfile
import unittest
from importlib import import_module
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch


SERVER_ROOT = Path(__file__).resolve().parents[1]
REPOSITORY_ROOT = SERVER_ROOT.parents[1]
sys.path.insert(0, str(SERVER_ROOT))

from amiga_indexing import build_index_commands, build_search_command
from environment import load_environment_file
from rag_qdrant_command import run_rag_qdrant, run_rag_qdrant_json


class FakeMCP:
    """Minimal FastMCP stand-in that preserves decorated tool functions."""

    def __init__(self, _name):
        pass

    def tool(self):
        return lambda function: function


class CommandContractTests(unittest.TestCase):
    def load_server(self):
        """Import the adapter without requiring the optional FastMCP package."""
        original_module = sys.modules.pop("amiga_rag_mcp_server", None)
        try:
            with patch.dict(sys.modules, {"fastmcp": SimpleNamespace(FastMCP=FakeMCP)}):
                return import_module("amiga_rag_mcp_server")
        finally:
            sys.modules.pop("amiga_rag_mcp_server", None)
            if original_module is not None:
                sys.modules["amiga_rag_mcp_server"] = original_module

    def test_path_command_runner_decodes_json_response(self):
        calls = []

        def runner(command, **kwargs):
            calls.append((command, kwargs))
            return SimpleNamespace(returncode=0, stdout='[{"source": "amiga"}]', stderr="")

        result = run_rag_qdrant_json(
            ["search", "Copper timing", "--index-json", "index.json", "--json"],
            runner=runner,
        )

        self.assertEqual(result, [{"source": "amiga"}])
        self.assertEqual(
            calls[0][0],
            ["rag_qdrant", "search", "Copper timing", "--index-json", "index.json", "--json"],
        )
        self.assertTrue(calls[0][1]["capture_output"])

    def test_path_command_runner_returns_index_output(self):
        def runner(command, **kwargs):
            self.assertEqual(
                command,
                ["rag_qdrant", "docs", "--source", "amiga", "--index-json", "index.json"],
            )
            self.assertEqual(kwargs["timeout"], 1800)
            return SimpleNamespace(returncode=0, stdout="Indexed 3 files\n", stderr="")

        self.assertEqual(
            run_rag_qdrant(
                ["docs", "--source", "amiga", "--index-json", "index.json"],
                runner=runner,
                timeout_seconds=1800,
            ),
            "Indexed 3 files\n",
        )

    def test_indexing_uses_explicit_scopes_and_shared_state_file(self):
        repository_root = Path("repository")

        self.assertEqual(
            build_index_commands(repository_root, "index.json"),
            [
                ["repository\\docs", "--source", "amiga", "--index-json", "index.json"],
                [
                    str(repository_root / "Obsidian" / "Amiga" / "Design"),
                    "--source",
                    "amiga",
                    "--index-json",
                    "index.json",
                ],
                [
                    str(repository_root / "Obsidian" / "Amiga" / "Reference"),
                    "--source",
                    "amiga",
                    "--index-json",
                    "index.json",
                ],
            ],
        )

    def test_indexing_uses_no_retired_scope_or_force_options(self):
        commands = build_index_commands(Path("repository"), "index.json")

        self.assertFalse(any("--include-dirs" in command for command in commands))
        self.assertFalse(any("--reindex" in command for command in commands))

    def test_multisource_search_uses_the_cli_comma_separated_filter(self):
        self.assertEqual(
            build_search_command("Copper timing", ["amiga", "devnotes", "amiga"], 2, "index.json"),
            [
                "search",
                "Copper timing",
                "--index-json",
                "index.json",
                "--source",
                "amiga,devnotes",
                "--limit",
                "2",
                "--json",
            ],
        )

    def test_unfiltered_search_omits_source_option(self):
        self.assertEqual(
            build_search_command("Copper timing", None, 5, "index.json"),
            ["search", "Copper timing", "--index-json", "index.json", "--limit", "5", "--json"],
        )

    def test_mcp_search_passes_the_required_state_file_and_combined_sources(self):
        server = self.load_server()
        original_index_json = os.environ.get("RAG_INDEX_JSON")
        calls = []
        try:
            os.environ["RAG_INDEX_JSON"] = "index.json"
            server.run_rag_qdrant_json = lambda arguments: calls.append(arguments) or []

            self.assertIn("No relevant documentation found", server.rag_search("Copper timing", ["amiga", "devnotes"]))
            self.assertEqual(
                calls,
                [["search", "Copper timing", "--index-json", "index.json", "--source", "amiga,devnotes", "--limit", "5", "--json"]],
            )
        finally:
            if original_index_json is None:
                os.environ.pop("RAG_INDEX_JSON", None)
            else:
                os.environ["RAG_INDEX_JSON"] = original_index_json

    def test_mcp_status_reports_the_cli_json_error(self):
        server = self.load_server()
        original_index_json = os.environ.get("RAG_INDEX_JSON")
        try:
            os.environ["RAG_INDEX_JSON"] = "index.json"
            server.run_rag_qdrant_json = lambda _arguments: {"error": "connection refused"}

            self.assertEqual(server.rag_status(), "Qdrant connection error: connection refused")
        finally:
            if original_index_json is None:
                os.environ.pop("RAG_INDEX_JSON", None)
            else:
                os.environ["RAG_INDEX_JSON"] = original_index_json

    def test_environment_file_sets_missing_index_state_without_overriding_process(self):
        variable_name = "RAG_INDEX_JSON"
        original = os.environ.pop(variable_name, None)
        try:
            with tempfile.TemporaryDirectory() as temporary_directory:
                environment_file = Path(temporary_directory) / ".env"
                environment_file.write_text(f"{variable_name}=index.json\n", encoding="utf-8")

                load_environment_file(environment_file)
                self.assertEqual(os.environ[variable_name], "index.json")

                os.environ[variable_name] = "process-index.json"
                load_environment_file(environment_file)
                self.assertEqual(os.environ[variable_name], "process-index.json")
        finally:
            if original is None:
                os.environ.pop(variable_name, None)
            else:
                os.environ[variable_name] = original

    def test_mcp_server_requires_cli_state_file_and_exposes_incremental_indexing(self):
        server_source = (SERVER_ROOT / "amiga_rag_mcp_server.py").read_text(encoding="utf-8")

        self.assertIn('RAG_INDEX_JSON_VARIABLE = "RAG_INDEX_JSON"', server_source)
        self.assertIn("build_search_command", server_source)
        self.assertIn("run_rag_qdrant_json", server_source)
        self.assertIn("def rag_reindex()", server_source)
        self.assertNotIn("RAG_CACHE_FILE", server_source)
        self.assertNotIn("--reindex", server_source)
        self.assertNotIn("KnowledgeIndexer", server_source)

    def test_project_no_longer_contains_the_rag_qdrant_tool(self):
        self.assertFalse((REPOSITORY_ROOT / "tools" / "rag-qdrant").exists())


if __name__ == "__main__":
    unittest.main()

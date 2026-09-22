"""Regression tests for the Amiga RAG MCP command boundary."""

import sys
import os
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace


SERVER_ROOT = Path(__file__).resolve().parents[1]
REPOSITORY_ROOT = SERVER_ROOT.parents[1]
sys.path.insert(0, str(SERVER_ROOT))

from amiga_indexing import build_index_commands
from environment import load_environment_file
from rag_qdrant_command import run_rag_qdrant, run_rag_qdrant_json


class CommandContractTests(unittest.TestCase):
    def test_path_command_runner_decodes_json_response(self):
        calls = []

        def runner(command, **kwargs):
            calls.append((command, kwargs))
            return SimpleNamespace(returncode=0, stdout='[{"source": "amiga"}]', stderr="")

        result = run_rag_qdrant_json(["search", "Copper timing", "--json"], runner=runner)

        self.assertEqual(result, [{"source": "amiga"}])
        self.assertEqual(calls[0][0], ["rag_qdrant", "search", "Copper timing", "--json"])
        self.assertTrue(calls[0][1]["capture_output"])

    def test_path_command_runner_returns_index_output(self):
        def runner(command, **kwargs):
            self.assertEqual(command, ["rag_qdrant", "repo", "--source", "amiga"])
            self.assertEqual(kwargs["timeout"], 1800)
            return SimpleNamespace(returncode=0, stdout="Indexed 3 files\n", stderr="")

        self.assertEqual(
            run_rag_qdrant(["repo", "--source", "amiga"], runner=runner, timeout_seconds=1800),
            "Indexed 3 files\n",
        )

    def test_indexing_covers_only_the_three_supported_scopes(self):
        repository_root = Path("repository")

        commands = build_index_commands(repository_root)

        self.assertEqual(
            commands,
            [
                ["repository", "--source", "amiga", "--include-dirs", "docs"],
                [
                    str(repository_root / "Obsidian" / "Amiga"),
                    "--source",
                    "amiga",
                    "--include-dirs",
                    "Design",
                    "Reference",
                ],
            ],
        )

    def test_force_indexing_adds_reindex_to_every_scope(self):
        commands = build_index_commands(Path("repository"), force=True)

        self.assertEqual([command[-1] for command in commands], ["--reindex", "--reindex"])

    def test_environment_file_sets_missing_cache_variable_without_overriding_process(self):
        variable_name = "RAG_MCP_TEST_CACHE"
        original = os.environ.pop(variable_name, None)
        try:
            with tempfile.TemporaryDirectory() as temporary_directory:
                environment_file = Path(temporary_directory) / ".env"
                environment_file.write_text(f"{variable_name}=shared-cache.json\n", encoding="utf-8")

                load_environment_file(environment_file)
                self.assertEqual(os.environ[variable_name], "shared-cache.json")

                os.environ[variable_name] = "process-cache.json"
                load_environment_file(environment_file)
                self.assertEqual(os.environ[variable_name], "process-cache.json")
        finally:
            if original is None:
                os.environ.pop(variable_name, None)
            else:
                os.environ[variable_name] = original

    def test_mcp_server_loads_cache_environment_and_exposes_reindex(self):
        server_source = (SERVER_ROOT / "amiga_rag_mcp_server.py").read_text(encoding="utf-8")

        self.assertIn("load_environment_file(PROJECT_ROOT / \".env\")", server_source)
        self.assertIn("RAG_CACHE_FILE", server_source)
        self.assertIn("def rag_reindex(force: bool = False)", server_source)
        self.assertNotIn("KnowledgeIndexer", server_source)

    def test_project_no_longer_contains_the_rag_qdrant_tool(self):
        self.assertFalse((REPOSITORY_ROOT / "tools" / "rag-qdrant").exists())

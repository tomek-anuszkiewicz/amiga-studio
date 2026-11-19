"""Independent graph selection and filtering before traversal."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

with patch.object(sys, "path", [str(Path(__file__).resolve().parents[3]), *sys.path]):
    from tools import graphify_query as query


class GraphifyQueryScopeTests(unittest.TestCase):
    def test_scope_rejects_foreign_nodes_edges_hyperedges_and_escaping_provenance(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for folder in ("minimig", "vAmiga"):
                (root / folder).mkdir()
            graph = {
                "nodes": [{"id": "mini", "label": "Paula", "source_file": "minimig/Paula.v"},
                          {"id": "vamiga", "label": "Paula", "source_file": "vAmiga/Paula.cpp"},
                          {"id": "outside", "source_file": "minimig/../../secret.cpp"},
                          {"id": "unknown"}],
                "links": [{"source": "mini", "target": "vamiga", "source_file": "minimig/Paula.v"},
                          {"source": "mini", "target": "mini", "source_file": "vAmiga/Paula.cpp"}],
                "hyperedges": [{"nodes": ["mini", "vamiga"]}],
                "graph": {"hyperedges": [{"nodes": ["mini", "vamiga"]}]}}
            selected = query.filter_graph(graph, ["minimig"], root)
            self.assertEqual([node["id"] for node in selected["nodes"]], ["mini"])
            self.assertEqual(selected["links"], [])
            self.assertEqual(selected["hyperedges"], [])
            self.assertNotIn("hyperedges", selected["graph"])
            for scope in ("../escape", str(root), "missing", "minimig/../../", "C:/foreign"):
                with self.subTest(scope=scope), self.assertRaises(ValueError):
                    query.filter_graph(graph, [scope], root)

    def test_graph_selection_keeps_duplicate_names_separate_and_allows_project_docs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for prefix, source in (("", "docs/Paula.md"), ("ref_src/", "vAmiga/Paula.cpp")):
                out = root / prefix / "graphify-out"
                out.mkdir(parents=True)
                (out / "graph.json").write_text(json.dumps({"nodes": [{"id": prefix or "project",
                     "label": "Paula", "source_file": source}], "links": []}), encoding="utf-8")
            before = {p: p.read_bytes() for p in root.rglob("graph.json")}
            observed = []
            def run(command, **kwargs):
                data = json.loads(Path(command[-1]).read_text(encoding="utf-8"))
                observed.append(data["nodes"][0]["source_file"])
                return SimpleNamespace(returncode=0)
            for graph in ("project", "reference"):
                with patch.object(query, "REPOSITORY_ROOT", root), \
                     patch.object(query, "TEMP_ROOT", root / ".agent/tmp"), \
                     patch.object(query.subprocess, "run", side_effect=run), \
                     patch("sys.argv", ["query.py", "Paula", "--graph", graph]):
                    self.assertEqual(query.main(), 0)
            self.assertEqual(observed, ["docs/Paula.md", "vAmiga/Paula.cpp"])
            self.assertEqual(before, {p: p.read_bytes() for p in root.rglob("graph.json")})

    def test_missing_reference_is_reported_without_subprocess_or_creation(self):
        with tempfile.TemporaryDirectory() as directory, \
             patch.object(query, "REPOSITORY_ROOT", Path(directory)), \
             patch.object(query.subprocess, "run") as run, \
             patch("sys.argv", ["query.py", "Paula", "--graph", "reference"]):
            with self.assertRaises(SystemExit) as error:
                query.main()
            self.assertEqual(error.exception.code, 2)
            run.assert_not_called()
            self.assertEqual(list(Path(directory).iterdir()), [])


if __name__ == "__main__":
    unittest.main()

"""Host extraction rejects incomplete/stale input and reuses attributed cache."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from tools.graphify import refresh, semantic


class SemanticTests(unittest.TestCase):
    def test_host_extraction_cache_provenance_freshness_and_removal(self):
        try:
            interpreter = refresh.installed_python()
        except (RuntimeError, OSError, subprocess.CalledProcessError):
            self.skipTest("Optional Graphify tool is not installed")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repository = Path(refresh.__file__).resolve().parents[2]
            shutil.copyfile(repository / ".graphifyignore", root / ".graphifyignore")
            prompt = root / ".agents/skills/graphify/references/extraction-spec.md"
            prompt.parent.mkdir(parents=True)
            prompt.write_text("Fixture extraction contract.")
            doc = root / "design.md"
            doc.write_text("# Design\nA host extracted concept.\n")
            environment = dict(os.environ, GRAPHIFY_MAX_WORKERS="1")
            def run(*args, expected=0):
                result = subprocess.run([str(interpreter), str(Path(semantic.__file__).resolve()), *args,
                                         "--graph", "project", "--root", str(root)], env=environment,
                                        capture_output=True, encoding="utf-8")
                self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
                return result
            def prepare():
                before = set((root / "graphify-out").glob("semantic-*"))
                run("prepare")
                return (set((root / "graphify-out").glob("semantic-*")) - before).pop()
            batch = prepare()
            result = run("apply", "--run", str(batch), expected=1)
            self.assertIn("incomplete", result.stderr)
            self.assertFalse((root / "graphify-out/graph.json").exists())
            node = {"id": "design_concept", "label": "Host extracted concept", "file_type": "concept",
                    "source_file": str(doc), "source_location": "L2"}
            chunk = batch / "chunk-1.json"
            chunk.write_text(json.dumps({"nodes": [{**node, "source_file": str(root.parent / "outside.md")}]}))
            self.assertIn("provenance", run("apply", "--run", str(batch), expected=1).stderr)
            chunk.write_text(json.dumps({"nodes": [node], "edges": [], "hyperedges": []}))
            original = doc.read_text()
            doc.write_text(original + "Changed.\n")
            self.assertIn("changed", run("apply", "--run", str(batch), expected=1).stderr)
            doc.write_text(original)
            run("apply", "--run", str(batch))
            graph = json.loads((root / "graphify-out/graph.json").read_text())
            self.assertIn("design_concept", {n["id"] for n in graph["nodes"]})
            self.assertTrue(all(isinstance(n.get("community"), int) for n in graph["nodes"]))
            analysis = json.loads((root / "graphify-out/.graphify_analysis.json").read_text())
            self.assertIn("design_concept", {node for members in analysis["communities"].values() for node in members})
            report = root / "graphify-out/GRAPH_REPORT.md"
            self.assertIn("Host extracted concept", report.read_text(encoding="utf-8"))
            # Reapplying an unchanged semantic graph must regenerate missing reports.
            report.unlink()
            run("apply", "--run", str(batch))
            self.assertIn("Host extracted concept", report.read_text(encoding="utf-8"))
            self.assertTrue(all(n.get("source_file") == "design.md" for n in graph["nodes"]))
            cached = prepare()
            self.assertEqual((cached / "pending.txt").read_text(), "")
            prompt.write_text("Changed extraction contract.")
            self.assertIn("prompt changed", run("apply", "--run", str(cached), expected=1).stderr)
            new_batch = prepare()
            self.assertIn(str(doc), (new_batch / "pending.txt").read_text())
            # A code file keeps a valid nonempty graph after the only document is removed.
            (root / "main.py").write_text("def retained_function(): pass\n")
            doc.unlink()
            removed = prepare()
            run("apply", "--run", str(removed))
            graph = json.loads((root / "graphify-out/graph.json").read_text())
            self.assertNotIn("design_concept", {n["id"] for n in graph["nodes"]})
            self.assertIn("main.py", {n.get("source_file") for n in graph["nodes"]})


if __name__ == "__main__":
    unittest.main()

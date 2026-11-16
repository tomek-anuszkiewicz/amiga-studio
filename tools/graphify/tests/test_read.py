"""Native traversal must leave every graph artifact and query log unchanged."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from tools.graphify import refresh


class ReadOnlyTests(unittest.TestCase):
    def test_native_stamp_reproduction_and_writer_free_query_path_explain(self):
        try:
            interpreter = refresh.installed_python()
        except (RuntimeError, OSError, subprocess.CalledProcessError):
            self.skipTest("Optional Graphify tool is not installed")
        reader = Path(refresh.__file__).with_name("read.py").resolve()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            graph = {"directed": True, "multigraph": False, "graph": {},
                     "nodes": [{"id": "caller", "label": "Caller", "source_file": "main.py", "file_type": "code"},
                               {"id": "callee", "label": "Callee", "source_file": "other.py", "file_type": "code"}],
                     "links": [{"source": "caller", "target": "callee", "relation": "calls",
                                "source_file": "main.py", "confidence": "EXTRACTED", "confidence_score": 1.0}]}
            outputs = [root / "project/graphify-out", root / "ref_src/graphify-out"]
            for out in outputs:
                out.mkdir(parents=True)
                (out / "graph.json").write_text(json.dumps(graph), encoding="utf-8")
            log = root / "explicit-query.log"
            environment = dict(os.environ, GRAPHIFY_QUERY_LOG=str(log), GRAPHIFY_QUERY_LOG_ENABLE="1")
            environment.pop("GRAPHIFY_QUERY_LOG_DISABLE", None)
            raw = subprocess.run([str(interpreter), "-m", "graphify", "query", "Caller", "--graph",
                                  str(outputs[0] / "graph.json")], env=environment,
                                 capture_output=True, encoding="utf-8")
            self.assertEqual(raw.returncode, 0, raw.stdout + raw.stderr)
            self.assertTrue((outputs[0] / "cache/last_query_stamp").is_file())
            self.assertTrue(log.is_file())
            def snapshot():
                return {str(p.relative_to(root)): (p.read_bytes(), p.stat().st_mtime_ns)
                        for p in root.rglob("*") if p.is_file()}
            before = snapshot()
            for out in outputs:
                for operation, terms in (("query", ["Caller"]), ("path", ["Caller", "Callee"]),
                                         ("explain", ["Caller"])):
                    result = subprocess.run([sys.executable, "-B", str(reader), operation, *terms,
                                             "--graph", str((out / "graph.json").relative_to(root)),
                                             "--root", str(root)], cwd=root.parent, env=environment,
                                            capture_output=True, encoding="utf-8")
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertIn("Caller", result.stdout)
                    self.assertEqual(before, snapshot(), f"{operation} changed stored graph state or query log")

    def test_missing_graph_reports_gap_without_provisioning(self):
        reader = Path(refresh.__file__).with_name("read.py").resolve()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            result = subprocess.run([sys.executable, "-B", str(reader), "query", "Paula", "--graph",
                                     "ref_src/graphify-out/graph.json", "--root", str(root)],
                                    capture_output=True, encoding="utf-8")
            self.assertEqual(result.returncode, 2)
            self.assertIn("missing", result.stderr)
            self.assertEqual(list(root.iterdir()), [])


if __name__ == "__main__":
    unittest.main()

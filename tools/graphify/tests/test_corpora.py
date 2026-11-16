"""Exercise real Graphify reconciliation on two small independent corpora."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from tools.graphify import refresh


class CorpusTests(unittest.TestCase):
    def test_two_roots_ignore_policy_semantics_and_output_isolation(self):
        try:
            interpreter = refresh.installed_python()
        except (RuntimeError, OSError, subprocess.CalledProcessError):
            self.skipTest("Optional Graphify tool is not installed")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "checkout"
            root.mkdir()
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            repository = Path(refresh.__file__).resolve().parents[2]
            shutil.copyfile(repository / ".graphifyignore", root / ".graphifyignore")
            (root / ".gitignore").write_text("/ref_src/*\n!/ref_src/.graphifyignore\n/private.py\n/graphify-out/\n")
            (root / "main.py").write_text("def project_function(): pass\n")
            (root / "private.py").write_text("def private_function(): pass\n")
            (root / "docs").mkdir()
            document = root / "docs/design.md"
            document.write_text("# Project design\nA documented concept.\n")
            reference = root / "ref_src"
            (reference / "minimig/build").mkdir(parents=True)
            (reference / ".graphifyignore").write_text("vAmigaTS/\n")
            (reference / "minimig/Paula.c").write_text("void reference_function(void) {}\n")
            (reference / "minimig/design.md").write_text("# Excluded reference documentation\n")
            (reference / "minimig/build/generated.c").write_text("void generated_function(void) {}\n")
            (reference / "vAmigaTS").mkdir()
            (reference / "vAmigaTS/excluded.c").write_text("void excluded_function(void) {}\n")
            subprocess.run(["git", "add", ".gitignore", ".graphifyignore", "ref_src/.graphifyignore"], cwd=root, check=True)
            self.assertNotEqual(subprocess.run(["git", "check-ignore", "ref_src/minimig/Paula.c"], cwd=root,
                                               capture_output=True).returncode, 1)
            self.assertEqual(subprocess.run(["git", "check-ignore", "ref_src/.graphifyignore"], cwd=root,
                                            capture_output=True).returncode, 1)
            out = root / "graphify-out"
            out.mkdir()
            (out / ".graphify_build.json").write_text('{"gitignore": false}')
            (out / ".graphify_root").write_text(str(root.parent / "foreign-checkout"))
            (out / "generated.py").write_text("def generated_graph_function(): pass\n")
            environment = dict(os.environ, GRAPHIFY_OUT=str(root.parent / "wrong-output"), GRAPHIFY_MAX_WORKERS="1")
            def build(graph):
                result = subprocess.run([str(interpreter), str(Path(refresh.__file__).resolve()), "--graph", graph,
                                         "--root", str(root)], cwd=root.parent, env=environment,
                                        capture_output=True, encoding="utf-8")
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                return json.loads((root / ("ref_src/" if graph == "reference" else "") /
                                   "graphify-out/graph.json").read_text(encoding="utf-8"))
            project = build("project")
            sources = {n.get("source_file") for n in project["nodes"]}
            self.assertIn("main.py", sources)
            self.assertIn("docs/design.md", sources)
            self.assertFalse(any(s and (s.startswith("ref_src/") or s == "private.py" or "graphify-out" in s) for s in sources))
            self.assertTrue(json.loads((out / ".graphify_build.json").read_text())["gitignore"])
            before = {p: p.read_bytes() for p in out.rglob("*") if p.is_file()}
            ref = build("reference")
            self.assertEqual(before, {p: p.read_bytes() for p in out.rglob("*") if p.is_file()})
            self.assertEqual({n.get("source_file") for n in ref["nodes"]}, {"minimig/Paula.c"})
            self.assertTrue(all(n.get("_origin") == "ast" for n in ref["nodes"]))
            ref_out = reference / "graphify-out"
            self.assertFalse(json.loads((ref_out / ".graphify_build.json").read_text())["gitignore"])
            ref_before = {p: p.read_bytes() for p in ref_out.rglob("*") if p.is_file()}
            project["nodes"].append({"id": "semantic_concept", "label": "Documented concept",
                                     "file_type": "concept", "_origin": "semantic", "source_file": "docs/design.md"})
            (out / "graph.json").write_text(json.dumps(project), encoding="utf-8")
            refreshed = build("project")
            self.assertIn("semantic_concept", {n["id"] for n in refreshed["nodes"]})
            self.assertEqual(ref_before, {p: p.read_bytes() for p in ref_out.rglob("*") if p.is_file()})
            with (root / ".graphifyignore").open("a") as stream:
                stream.write("\ndocs/\n")
            refreshed = build("project")
            self.assertNotIn("semantic_concept", {n["id"] for n in refreshed["nodes"]})
            # Removal and explicit exclusion have the same corpus boundary.
            document.unlink()
            refreshed = build("project")
            self.assertNotIn("docs/design.md", {n.get("source_file") for n in refreshed["nodes"]})
            for selected_out in (out, ref_out):
                manifest = json.loads((selected_out / "manifest.json").read_text(encoding="utf-8"))
                self.assertTrue(all(not Path(path).is_absolute() and "graphify-out" not in path for path in manifest))
            self.assertFalse((root.parent / "wrong-output").exists())

    def test_missing_reference_code_fails_without_graph_creation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(RuntimeError, "sources are missing"):
                refresh.build(None, root, "reference")
            self.assertEqual(list(root.iterdir()), [])


if __name__ == "__main__":
    unittest.main()

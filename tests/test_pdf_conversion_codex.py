"""PDF migration contracts and preservation of artifacts on invalid configuration."""

from pathlib import Path
import json
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools/bootstrap"))
from conversion import load_config
from conversion.config import PDF_STAGES
from conversion.lineage import complete_stage, validate_prefix, restore_shared, stage_identity

ROOT = Path(__file__).resolve().parents[1]


class PdfConfigurationTests(unittest.TestCase):
    def test_all_inference_stage_pairs_are_explicit(self):
        config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml", known_stages=PDF_STAGES, required_stages=PDF_STAGES)
        self.assertEqual(set(config["llm"]["stages"]), PDF_STAGES)

    def test_duplicate_stage_selection_is_rejected_before_cleanup(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            workspace = root / "workspace"
            output = workspace / "14_link_toc"
            output.mkdir(parents=True)
            sentinel = output / "preserved.txt"
            sentinel.write_text("Existing accepted artifact")
            config = root / "config.yaml"
            config.write_text('llm:\n  stages:\n    13_refine_first_chapter_name: {model: gpt-6.1-sol, reasoning_effort: medium}\n    13_refine_first_chapter_name: {model: other, reasoning_effort: low}\n')
            result = subprocess.run([sys.executable, str(ROOT / "tools/bootstrap/pdf-to-markdown/pipeline.py"),
                                     "--config", str(config), "--workspace", str(workspace),
                                     "--from-stage", "14", "--to-stage", "14"], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertTrue(sentinel.exists(), result.stdout + result.stderr)

    def test_missing_predecessor_does_not_clear_downstream(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            workspace = root / "workspace"
            output = workspace / "14_link_toc"
            output.mkdir(parents=True)
            sentinel = output / "accepted.md"
            sentinel.write_text("Accepted output")
            result = subprocess.run([sys.executable, str(ROOT / "tools/bootstrap/pdf-to-markdown/pipeline.py"),
                                     "--config", str(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml"),
                                     "--workspace", str(workspace), "--from-stage", "14", "--to-stage", "14"], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertTrue(sentinel.is_file())


class LineageTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.workspace = Path(self.tmp.name) / "workspace"
        self.workspace.mkdir()
        self.skill = Path(self.tmp.name) / "skill"
        self.registry = [{"id": "01", "dir": "01_preprocess"}, {"id": "02", "dir": "02_page_segmentation"}]
        self.config = {"llm": {"stages": {stage["dir"]: {"model": "gpt-6.1-sol", "reasoning_effort": "medium"} for stage in self.registry}}}
        self.source = {"name": "sample.pdf", "sha256": "source-identity", "pages": [19]}
        self.state = {"source": self.source, "stages": {}}
        previous = None
        (self.workspace / "pages_manifest.json").write_text('{"pages":[19]}')
        for stage in self.registry:
            directory = self.skill / "stages" / stage["dir"]
            directory.mkdir(parents=True)
            (directory / "prompt.md").write_text("Convert source")
            output = self.workspace / stage["dir"]
            output.mkdir()
            (output / "page_0019.json").write_text('{"page":19}')
            identity = stage_identity(stage, self.config, self.source, previous, self.skill)
            previous = complete_stage(self.state, stage, identity, self.workspace, None)

    def validate(self):
        return validate_prefix(self.state, self.registry, 2, self.config, self.source, self.skill, self.workspace, None)

    def test_unchanged_predecessors_validate(self):
        self.assertEqual(self.validate(), self.state["stages"]["02"]["completion"])

    def test_changed_effort_identifies_earliest_affected_stage(self):
        self.config["llm"]["stages"]["02_page_segmentation"]["reasoning_effort"] = "high"
        with self.assertRaisesRegex(ValueError, "Stage 02"):
            self.validate()

    def test_modified_artifact_or_source_is_rejected(self):
        (self.workspace / "01_preprocess/page_0019.json").write_text('{"page":20}')
        with self.assertRaisesRegex(ValueError, "Stage 01"):
            self.validate()

    def test_shared_manifest_snapshot_restores_original(self):
        restore_shared(self.state, self.registry, 1, self.workspace)
        self.assertEqual(json.loads((self.workspace / "pages_manifest.json").read_text()), {"pages": [19]})


if __name__ == "__main__":
    unittest.main()

"""PDF migration contracts and preservation of artifacts on invalid configuration."""

from pathlib import Path
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools/bootstrap"))
from conversion import load_config
from conversion.config import PDF_STAGES
from conversion.lineage import complete_stage, validate_prefix, restore_shared, stage_identity, file_hash

ROOT = Path(__file__).resolve().parents[1]




class PdfRestartTests(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location("pdf_pipeline", ROOT / "tools/bootstrap/pdf-to-markdown/pipeline.py")
        self.pipeline = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.pipeline)
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.workspace = Path(self.tmp.name) / "workspace"
        self.workspace.mkdir()
        self.output = self.workspace / "14_link_toc"
        self.config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml", known_stages=PDF_STAGES, required_stages=PDF_STAGES)
        self.pdf = Path(self.tmp.name) / "sample.pdf"
        self.pdf.write_bytes(b"PDF transport is mocked in this orchestration test")
        self.source = {"name": self.pdf.name, "sha256": file_hash(self.pdf), "pages": list(range(5, 11))}
        self.state = {"source": self.source, "stages": {}}
        previous = None
        for stage in self.pipeline.STAGE_REGISTRY:
            self.write_artifacts(stage)
            identity = stage_identity(stage, self.config, self.source, previous, ROOT / "tools/bootstrap/pdf-to-markdown")
            previous = complete_stage(self.state, stage, identity, self.workspace, self.output)
        for number, name in (("06", "continuations"), ("07", "tables"), ("08", "graphics"), ("09", "prose")):
            task = self.workspace / "tasks" / name / "old.md"
            task.parent.mkdir(parents=True, exist_ok=True)
            task.write_text("Old manual task")
            marker = self.workspace / "tasks/.lineage" / f"{number}.json"
            marker.parent.mkdir(exist_ok=True)
            marker.write_text("{}")

    def write_artifacts(self, stage):
        directory = self.workspace / stage["dir"]
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "artifact.json").write_text('{"generated":true}')
        if stage["id"] == "01":
            (self.workspace / "pages_manifest.json").write_text('{"pages":[5,6,7,8,9,10]}')
        if stage["id"] in ("05", "10"):
            (self.workspace / "chapters_manifest.json").write_text(json.dumps({"writer": stage["id"]}))

    def run_pipeline(self, flags, worker, *, page_ranges="5-10"):
        argv = ["pipeline.py", "--workspace", str(self.workspace), "--config", str(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml")]
        if page_ranges:
            argv.extend(["--page-ranges", page_ranges])
        argv.extend(flags)
        with patch.object(sys, "argv", argv), patch.object(self.pipeline, "CodexTransport"), patch.object(self.pipeline, "run_stage", side_effect=worker):
            self.pipeline.main()

    def test_restart_five_clears_all_dependents_and_tasks_with_missing_manifest(self):
        (self.workspace / "chapters_manifest.json").unlink()
        before = (self.workspace / "04_stream_reduction/artifact.json").read_bytes()
        other_output = Path(self.tmp.name) / "other-workspace/14_link_toc/accepted.md"
        other_output.parent.mkdir(parents=True)
        other_output.write_text("Independent conversion")
        def worker(stage, *args):
            self.assertEqual(stage["id"], "05")
            for dependent in self.pipeline.STAGE_REGISTRY[4:]:
                self.assertFalse((self.workspace / dependent["dir"]).exists())
            self.assertFalse((self.workspace / "tasks/tables/old.md").exists())
            self.assertFalse((self.workspace / "tasks/.lineage/07.json").exists())
            self.assertFalse((self.workspace / "chapters_manifest.json").exists())
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "5", "--to-stage", "5"], worker)
        self.assertEqual((self.workspace / "04_stream_reduction/artifact.json").read_bytes(), before)
        self.assertEqual(other_output.read_text(), "Independent conversion")
        state = json.loads((self.workspace / ".conversion-state.json").read_text())
        self.assertEqual(list(state["stages"]), ["01", "02", "03", "04", "05"])

    def test_missing_current_manifest_is_restored_from_preserved_snapshot(self):
        (self.workspace / "pages_manifest.json").unlink()
        def worker(stage, *args):
            self.assertEqual(json.loads((self.workspace / "pages_manifest.json").read_text())["pages"], list(range(5, 11)))
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "5", "--to-stage", "5"], worker)

    def test_resume_restarts_first_stage_with_missing_output(self):
        (self.workspace / "05_chapter_partition/artifact.json").unlink()
        executed = []
        def worker(stage, *args):
            executed.append(stage["id"])
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--resume"], worker)
        self.assertEqual(executed, [stage["id"] for stage in self.pipeline.STAGE_REGISTRY[4:]])

    def test_changed_page_range_rejects_before_cleanup(self):
        with self.assertRaisesRegex(ValueError, "Stage 01"):
            self.run_pipeline(["--from-stage", "5", "--to-stage", "5", "--page-ranges", "6-10"], lambda *args: self.fail("Worker must not run"))
        self.assertTrue((self.workspace / "14_link_toc/artifact.json").exists())

    def test_reset_graphics_does_not_delete_preserved_assets(self):
        asset = self.workspace / "04_stream_reduction/assets/source.png.txt"
        asset.parent.mkdir()
        asset.write_text("Preserved source evidence")
        self.pipeline.clean_downstream_stages(self.workspace, self.output, 7, self.workspace / "stage_status.json")
        self.assertTrue(asset.exists())

    def test_output_defaults_to_selected_workspace_even_with_pdf(self):
        source_note = self.pdf.parent / "existing.md"
        source_note.write_text("Source directory must remain intact")
        def worker(stage, *args):
            self.assertEqual(args[3], self.output)
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--pdf", str(self.pdf), "--from-stage", "5", "--to-stage", "5"], worker)
        self.assertEqual(source_note.read_text(), "Source directory must remain intact")

    def test_missing_predecessor_requires_restart_at_its_stage(self):
        (self.workspace / "03_build_raw_stream/artifact.json").unlink()
        with self.assertRaisesRegex(ValueError, "Stage 03"):
            self.run_pipeline(["--from-stage", "5", "--to-stage", "5"], lambda *args: self.fail("Worker must not run"))
        self.assertTrue((self.workspace / "14_link_toc/artifact.json").exists())

    def test_resume_missing_preprocess_retains_selected_pages(self):
        (self.workspace / "01_preprocess/artifact.json").unlink()
        pdf = Path(self.tmp.name) / "sample.pdf"
        pdf.write_bytes(b"PDF transport is mocked in this orchestration test")
        executed = []
        def worker(stage, *args):
            executed.append(stage["id"])
            if stage["id"] == "01":
                self.assertEqual(args[-1], "5,6,7,8,9,10")
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--resume", "--pdf", str(pdf)], worker, page_ranges=None)
        self.assertEqual(executed[0], "01")
        state = json.loads((self.workspace / ".conversion-state.json").read_text())
        self.assertEqual(state["source"]["pages"], list(range(5, 11)))

    def test_completed_resume_reconstructs_missing_working_manifest(self):
        (self.workspace / "chapters_manifest.json").unlink()
        self.run_pipeline(["--resume"], lambda *args: self.fail("Completed stages must not run"))
        self.assertEqual(json.loads((self.workspace / "chapters_manifest.json").read_text()), {"writer": "10"})

    def test_manual_prepare_discards_dependents_and_apply_keeps_current_edits(self):
        def prepare(command, **kwargs):
            self.assertEqual(command[-1], "--prepare")
            self.assertFalse((self.workspace / "07_transform_tables").exists())
            self.assertFalse((self.workspace / "tasks/tables/old.md").exists())
            self.assertFalse((self.workspace / "tasks/graphics/old.md").exists())
            self.assertEqual(json.loads((self.workspace / "chapters_manifest.json").read_text()), {"writer": "05"})
            task = self.workspace / "tasks/tables/accepted.md"
            task.parent.mkdir(parents=True)
            task.write_text("Accepted manual edit")
        with patch.object(self.pipeline.subprocess, "run", side_effect=prepare):
            self.run_pipeline(["--prepare-stage", "7"], lambda *args: self.fail("Automatic worker must not run"))
        def apply(command, **kwargs):
            self.assertEqual(command[-1], "--apply")
            self.assertEqual((self.workspace / "tasks/tables/accepted.md").read_text(), "Accepted manual edit")
            self.write_artifacts(self.pipeline.STAGE_REGISTRY[6])
        with patch.object(self.pipeline.subprocess, "run", side_effect=apply):
            self.run_pipeline(["--apply-stage", "7"], lambda *args: self.fail("Automatic worker must not run"))
        state = json.loads((self.workspace / ".conversion-state.json").read_text())
        validate_prefix(state, self.pipeline.STAGE_REGISTRY, 7, self.config, self.source, ROOT / "tools/bootstrap/pdf-to-markdown", self.workspace, self.output)


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

"""PDF migration contracts and preservation of artifacts on invalid configuration."""

from pathlib import Path
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
import pymupdf
from unittest.mock import patch
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools/bootstrap"))
from conversion import load_config
from conversion.config import PDF_STAGES
from conversion.lineage import complete_stage, validate_prefix, restore_shared, stage_identity, file_hash
from conversion.pdf_geometry import text_blocks

ROOT = Path(__file__).resolve().parents[1]


class PdfTextBoundsTests(unittest.TestCase):
    def extract(self, box):
        page = SimpleNamespace(rect=pymupdf.Rect(0, 0, 504, 649),
                               rotation_matrix=pymupdf.Matrix(1, 1),
                               get_text=lambda *args, **kwargs: {"blocks": [
                                   {"type": 0, "number": 0, "bbox": box,
                                    "lines": [{"spans": [{"text": "Retained text"}]}]}]})
        return text_blocks(page)

    def test_native_text_bounds_clip_without_truncating_text(self):
        block = self.extract([-2, -3, 507.405, 652])[0]
        self.assertEqual(block["bbox"], [0, 0, 504, 649])
        self.assertEqual(block["bbox_norm"], [0, 0, 1, 1])
        self.assertEqual(block["text"], "Retained text\n")

    def test_invalid_or_fully_outside_bounds_still_fail(self):
        for box in ([505, 10, 510, 20], [20, 10, 10, 20], [0, 0, float("nan"), 20]):
            with self.subTest(box=box), self.assertRaises(ValueError):
                self.extract(box)




class PdfOcrValidationTests(unittest.TestCase):
    def test_raw_ocr_pdf_overlay_preserves_fractional_text_and_spacing(self):
        spec = importlib.util.spec_from_file_location(
            "prepare_text_layer", ROOT / "tools/bootstrap/pdf-to-markdown/stages/00_text_layer/prepare_text_layer.py")
        stage = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(stage)
        with pymupdf.open() as ocr, pymupdf.open() as target:
            raw = ocr.new_page(width=200, height=200)
            raw.insert_text((35.251, 80.503), "7 6 5 4", fontsize=12.345, render_mode=3)
            expected = raw.get_text("rawdict")
            page = target.new_page(width=200, height=200)
            stage.insert_ocr_pdf(page, ocr)
            self.assertEqual(page.get_text("text"), raw.get_text("text"))
            actual = page.get_text("rawdict")
            self.assertEqual(actual["blocks"], expected["blocks"])

    def test_failed_ocr_pdf_publishes_no_recovery_or_manifest(self):
        spec = importlib.util.spec_from_file_location(
            "prepare_text_layer", ROOT / "tools/bootstrap/pdf-to-markdown/stages/00_text_layer/prepare_text_layer.py")
        stage = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(stage)
        config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml",
                             known_stages=PDF_STAGES, required_stages=())
        with tempfile.TemporaryDirectory() as directory:
            source, workspace = Path(directory) / "source.pdf", Path(directory) / "workspace"
            with pymupdf.open() as document:
                document.new_page()
                document.save(source)
            with patch.object(stage, "tesseract_settings", return_value=("eng", "unused", {"engine": "tesseract"})), \
                    patch.object(stage, "tesseract_ocr", return_value=b"invalid PDF"):
                with self.assertRaises(pymupdf.FileDataError):
                    stage.prepare_text_layer(source, workspace, config)
            self.assertFalse((workspace / "00_text_layer/recovery/page_0001.json").exists())
            self.assertFalse((workspace / "00_text_layer/recovery/page_0001.pdf").exists())
            self.assertFalse((workspace / "00_text_layer/text_layer_manifest.json").exists())



class PdfRestartTests(unittest.TestCase):
    def setUp(self):
        spec = importlib.util.spec_from_file_location("pdf_pipeline", ROOT / "tools/bootstrap/pdf-to-markdown/pipeline.py")
        self.pipeline = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.pipeline)
        # These fixtures prove restart orchestration, not PDF content validity.
        contracts = patch("conversion.pdf_artifacts.validate_stage_artifacts")
        contracts.start()
        self.addCleanup(contracts.stop)
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.workspace = Path(self.tmp.name) / "workspace"
        self.workspace.mkdir()
        self.output = self.workspace / "14_link_toc"
        self.config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml", known_stages=PDF_STAGES, required_stages=PDF_STAGES)
        self.pdf = Path(self.tmp.name) / "sample.pdf"
        with pymupdf.open() as document:
            for _ in range(10):
                document.new_page()
            document.save(self.pdf)
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

    def stage_idx(self, stage_id):
        return next(index for index, stage in enumerate(self.pipeline.STAGE_REGISTRY) if stage["id"] == stage_id)

    def run_pipeline(self, flags, worker, *, page_ranges="5-10"):
        argv = ["pipeline.py", "--workspace", str(self.workspace), "--config", str(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml")]
        if page_ranges:
            argv.extend(["--page-ranges", page_ranges])
        argv.extend(flags)
        with patch.object(sys, "argv", argv), patch.object(self.pipeline, "CodexTransport"), patch.object(self.pipeline, "run_stage", side_effect=worker):
            self.pipeline.main()

    def test_named_review_stage_restarts_by_registry_order(self):
        self.assertEqual(self.pipeline.resolve_stage_idx("02k"), self.stage_idx("02k"))
        self.assertEqual(self.pipeline.resolve_stage_idx("3"), self.stage_idx("03"))
        before = json.loads((self.workspace / ".conversion-state.json").read_text())["stages"]["02"]
        def worker(stage, *args):
            self.assertEqual(stage["id"], "02k")
            retained = json.loads((self.workspace / ".conversion-state.json").read_text())["stages"]
            self.assertEqual(set(retained), {"00", "01", "02"})
            self.assertFalse((self.workspace / "03_build_raw_stream").exists())
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "02k", "--to-stage", "02k"], worker)
        after = json.loads((self.workspace / ".conversion-state.json").read_text())["stages"]
        self.assertEqual(after["02"], before)
        self.assertEqual(after["02k"]["status"], "completed")
        self.assertNotIn("03", after)

    def test_restart_five_clears_all_dependents_and_tasks_with_missing_manifest(self):
        (self.workspace / "chapters_manifest.json").unlink()
        before = (self.workspace / "04_stream_reduction/artifact.json").read_bytes()
        other_output = Path(self.tmp.name) / "other-workspace/14_link_toc/accepted.md"
        other_output.parent.mkdir(parents=True)
        other_output.write_text("Independent conversion")
        def worker(stage, *args):
            self.assertEqual(stage["id"], "05")
            for dependent in self.pipeline.STAGE_REGISTRY[self.stage_idx("05"):]:
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
        self.assertEqual(list(state["stages"]), ["00", "01", "02", "02k", "03", "04", "05"])

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
        self.assertEqual(executed, [stage["id"] for stage in self.pipeline.STAGE_REGISTRY[self.stage_idx("05"):]])

    def test_changed_page_range_rejects_before_cleanup(self):
        with self.assertRaisesRegex(ValueError, "Stage 00"):
            self.run_pipeline(["--from-stage", "5", "--to-stage", "5", "--page-ranges", "6-10"], lambda *args: self.fail("Worker must not run"))
        self.assertTrue((self.workspace / "14_link_toc/artifact.json").exists())

    def test_reset_graphics_does_not_delete_preserved_assets(self):
        asset = self.workspace / "04_stream_reduction/assets/source.png.txt"
        asset.parent.mkdir()
        asset.write_text("Preserved source evidence")
        self.pipeline.clean_downstream_stages(self.workspace, self.output, self.stage_idx("08"), self.workspace / "stage_status.json")
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
            self.write_artifacts(self.pipeline.STAGE_REGISTRY[self.stage_idx("07")])
        with patch.object(self.pipeline.subprocess, "run", side_effect=apply):
            self.run_pipeline(["--apply-stage", "7"], lambda *args: self.fail("Automatic worker must not run"))
        state = json.loads((self.workspace / ".conversion-state.json").read_text())
        validate_prefix(state, self.pipeline.STAGE_REGISTRY, self.stage_idx("07") + 1, self.config, self.source, ROOT / "tools/bootstrap/pdf-to-markdown", self.workspace, self.output)


class PdfConfigurationTests(unittest.TestCase):
    def test_invisible_text_preserves_wrapped_source_streams(self):
        script = ROOT / "tools/bootstrap/pdf-to-markdown/stages/00_text_layer/prepare_text_layer.py"
        spec = importlib.util.spec_from_file_location("text_layer_worker", script)
        worker = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(worker)
        with tempfile.TemporaryDirectory() as directory:
            source, candidate = Path(directory) / "source.pdf", Path(directory) / "candidate.pdf"
            with pymupdf.open() as document:
                page = document.new_page(width=200, height=200)
                page.draw_rect(pymupdf.Rect(20, 20, 80, 80))
                # A valid unwrapped source stream makes PyMuPDF insert q/Q
                # wrappers around it when an overlay is added.
                document.update_stream(page.get_contents()[0], b"20 20 80 80 re S\n")
                document.save(source)
            with pymupdf.open(source) as document:
                with pymupdf.open() as ocr:
                    layer = ocr.new_page(width=200, height=200)
                    layer.insert_text((20, 40), "A", render_mode=3)
                    worker.insert_ocr_pdf(document[0], ocr)
                document.save(candidate)
            entry = {"page": 1, "provenance": "ocr"}
            worker.verify_candidate(source, candidate, [entry], 72)

    def test_preprocess_rejects_missing_text_layer_before_worker(self):
        spec = importlib.util.spec_from_file_location("handoff_pipeline", ROOT / "tools/bootstrap/pdf-to-markdown/pipeline.py")
        pipeline = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(pipeline)
        with tempfile.TemporaryDirectory() as directory, patch.object(pipeline.subprocess, "run") as worker:
            workspace = Path(directory)
            stage = next(item for item in pipeline.STAGE_REGISTRY if item["id"] == "01")
            with self.assertRaisesRegex(ValueError, "Stage 00"):
                pipeline.run_stage(stage, ROOT / "tools/bootstrap/pdf-to-markdown", workspace,
                                   workspace / "original.pdf", workspace / "output", workspace / "config.yaml")
            worker.assert_not_called()

    def test_all_inference_stage_pairs_are_explicit(self):
        config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml", known_stages=PDF_STAGES, required_stages=PDF_STAGES)
        self.assertEqual(set(config["llm"]["stages"]), PDF_STAGES)
        self.assertNotIn("00_text_layer", PDF_STAGES)
        self.assertNotIn("01_preprocess", PDF_STAGES)

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
        self.registry = [{"id": "00", "dir": "00_text_layer"}, {"id": "01", "dir": "01_preprocess"}, {"id": "02", "dir": "02_page_segmentation"}]
        self.config = {"llm": {"stages": {stage["dir"]: {"model": "gpt-6.1-sol", "reasoning_effort": "medium"}
                                          for stage in self.registry if stage["dir"] in PDF_STAGES}}}
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
        return validate_prefix(self.state, self.registry, len(self.registry), self.config, self.source, self.skill, self.workspace, None)

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
        restore_shared(self.state, self.registry, 2, self.workspace)
        self.assertEqual(json.loads((self.workspace / "pages_manifest.json").read_text()), {"pages": [19]})


if __name__ == "__main__":
    unittest.main()

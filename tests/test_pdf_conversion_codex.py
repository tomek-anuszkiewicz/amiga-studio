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
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools/bootstrap/pdf-to-markdown"))
from conversion import load_config
from conversion.config import PDF_STAGES
from common.lineage import complete_stage, validate_prefix, restore_shared, stage_identity, file_hash, artifact_predecessor
from common.pdf_geometry import text_blocks
from common.pdf_page_conversion import validate_page

ROOT = Path(__file__).resolve().parents[1]


class PdfReviewContinuationTests(unittest.TestCase):
    def test_02_5_labels_show_only_true_continuation(self):
        from PIL import Image, ImageDraw
        spec = importlib.util.spec_from_file_location(
            "page_conversion_review", ROOT / "tools/bootstrap/pdf-to-markdown/stages/02.5_page_conversion_review/render_review.py")
        renderer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(renderer)
        segments = [
            {"type": "caption", "heading_level": None, "continuation": True,
             "bbox": [10, 10, 100, 40]},
            {"type": "heading", "heading_level": 2, "continuation": False,
             "bbox": [10, 50, 100, 80]},
        ]
        with patch.object(ImageDraw.ImageDraw, "text") as draw_text:
            renderer.review_image(Image.new("RGB", (1200, 1600)), {"segments": segments})
        self.assertEqual([call.args[1] for call in draw_text.call_args_list], [
            "1. caption | continuation: true",
            "2. heading | heading_level: 2",
        ])


class PdfCoverContractTests(unittest.TestCase):
    def test_cover_supported_in_response_and_stored_artifact(self):
        entry = {"page": 1, "page_id": "page_0001",
                 "raster": {"pixel_width": 200, "pixel_height": 300}}
        value = {"page": 1, "image_width": 200, "image_height": 300,
                 "segments": [{"type": "cover", "continuation": False,
                               "heading_level": None, "md_text": "",
                               "bbox": [0, 0, 200, 300]}]}
        validate_page(value, entry)
        value["segments"][0]["segment_id"] = "page_0001_seg_001"
        validate_page(value, entry, stored=True)
        for kind in ("cover", "table", "graphic"):
            with self.subTest(kind=kind):
                value["segments"][0]["type"] = kind
                validate_page(value, entry, stored=True)
                for stored in (False, True):
                    candidate = {**value, "segments": [dict(value["segments"][0])]}
                    if not stored:
                        candidate["segments"][0].pop("segment_id")
                    validate_page(candidate, entry, stored=stored)
                    candidate["segments"][0]["md_text"] = "Generated content"
                    with self.assertRaisesRegex(ValueError, "md_text must be empty"):
                        validate_page(candidate, entry, stored=stored)


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
    def test_fragment_ocr_runs_only_for_selected_textless_page(self):
        spec = importlib.util.spec_from_file_location(
            "prepare_ocr_fragment", ROOT / "tools/bootstrap/pdf-to-markdown/stages/00_text_layer/prepare_text_layer.py")
        stage = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(stage)
        config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml",
                             known_stages=PDF_STAGES, required_stages=())
        config["render"]["dpi"] = 72
        with tempfile.TemporaryDirectory() as directory, pymupdf.open() as ocr:
            source, workspace = Path(directory) / "source.pdf", Path(directory) / "workspace"
            ocr.new_page(width=200, height=200).insert_text((20, 40), "Recovered text", render_mode=3)
            payload = ocr.tobytes()
            with pymupdf.open() as document:
                document.new_page(width=200, height=200)
                document.new_page(width=200, height=200).insert_text((20, 40), "Native text")
                document.new_page(width=200, height=200)
                document.save(source)
            with patch.object(stage, "tesseract_settings", return_value=("eng", "unused", {"engine": "tesseract"})), \
                    patch.object(stage, "tesseract_ocr", return_value=payload) as recognize:
                manifest = stage.prepare_text_layer(source, workspace, config, "2-3")
            recognize.assert_called_once()
            self.assertEqual([entry["provenance"] for entry in manifest["pages"]], ["native", "ocr"])
            with pymupdf.open(workspace / manifest["pdf_file"]) as prepared:
                self.assertEqual(len(prepared), 2)
                self.assertEqual([page.get_text().strip() for page in prepared], ["Native text", "Recovered text"])

    def test_fragment_pdf_contains_only_selected_pages_and_preprocess_maps_source_numbers(self):
        def load_stage(name, relative):
            spec = importlib.util.spec_from_file_location(name, ROOT / relative)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            return module

        stage = load_stage("prepare_fragment", "tools/bootstrap/pdf-to-markdown/stages/00_text_layer/prepare_text_layer.py")
        preprocess = load_stage("preprocess_fragment", "tools/bootstrap/pdf-to-markdown/stages/01_preprocess/preprocess.py")
        config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml",
                             known_stages=PDF_STAGES, required_stages=())
        config["render"]["dpi"] = 72
        with tempfile.TemporaryDirectory() as directory:
            source, workspace = Path(directory) / "source.pdf", Path(directory) / "workspace"
            with pymupdf.open() as document:
                for number in range(1, 6):
                    page = document.new_page(width=200, height=200)
                    page.insert_text((20, 40), f"Source page {number}")
                document.save(source)
            original_hash = file_hash(source)
            with patch.object(stage, "tesseract_ocr", side_effect=AssertionError("Native pages must not use OCR")), \
                    patch.object(pymupdf.Page, "get_pixmap", side_effect=AssertionError("Stage 00 must not render native pages for validation")):
                manifest = stage.prepare_text_layer(source, workspace, config, "2,5")
            prepared = workspace / manifest["pdf_file"]
            with pymupdf.open(prepared) as document:
                self.assertEqual(len(document), 2)
                self.assertEqual([page.get_text().strip() for page in document], ["Source page 2", "Source page 5"])
            self.assertEqual(file_hash(source), original_hash)
            self.assertEqual([entry["source_index"] for entry in manifest["pages"]], [1, 4])
            self.assertEqual([entry["prepared_index"] for entry in manifest["pages"]], [0, 1])
            registry = {"id": "00", "dir": "00_text_layer", "artifact_contract": "text_layer"}
            state = {"source": manifest["source"], "stages": {}}
            complete_stage(state, registry, manifest["ocr_procedure"], workspace, workspace / "14_link_toc")
            preprocess.preprocess_pdf(prepared, workspace, dpi=72, page_ranges="2,5")
            page_data = json.loads((workspace / "01_preprocess/page_0005.json").read_text())
            self.assertNotIn("pdf_sha256", manifest)
            self.assertNotIn("pdf_sha256", page_data)
            self.assertEqual(page_data["source_index"], 4)
            self.assertEqual(page_data["prepared_index"], 1)

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
        contracts = patch("common.pdf_artifacts.validate_stage_artifacts")
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
            identity = stage_identity(stage, self.config, self.source, artifact_predecessor(self.state, self.pipeline.STAGE_REGISTRY, stage), ROOT / "tools/bootstrap/pdf-to-markdown")
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
        self.assertEqual(self.pipeline.resolve_stage_idx("02"), self.stage_idx("02"))
        self.assertEqual(self.pipeline.resolve_stage_idx("2.5"), self.stage_idx("02.5"))
        self.assertIsNone(self.pipeline.resolve_stage_idx("02k"))
        self.assertEqual(self.pipeline.resolve_stage_idx("02.5"), self.stage_idx("02.5"))
        before = json.loads((self.workspace / ".conversion-state.json").read_text())["stages"]["02"]
        def worker(stage, *args):
            self.assertEqual(stage["id"], "02.5")
            retained = json.loads((self.workspace / ".conversion-state.json").read_text())["stages"]
            self.assertEqual(set(retained), {s["id"] for s in self.pipeline.STAGE_REGISTRY} - {"02.5"})
            self.assertTrue((self.workspace / "03_build_raw_stream").exists())
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "02.5", "--to-stage", "02.5"], worker)
        after = json.loads((self.workspace / ".conversion-state.json").read_text())["stages"]
        self.assertEqual(after["02"], before)
        self.assertEqual(after["02.5"]["status"], "completed")
        self.assertIn("03", after)

    def test_partial_page_conversion_does_not_create_final_output(self):
        (self.output / "artifact.json").unlink()
        self.output.rmdir()
        state_path = self.workspace / ".conversion-state.json"
        state = json.loads(state_path.read_text())
        del state["stages"]["14"]
        state_path.write_text(json.dumps(state))
        executed = []

        def worker(stage, *args):
            self.assertFalse(self.output.exists(), "Stage 14 directory must wait for Stage 14 execution")
            executed.append(stage["id"])
            self.write_artifacts(stage)
            return True

        self.run_pipeline(["--from-stage", "02", "--to-stage", "02.5"], worker)
        self.assertEqual(executed, ["02", "02.5"])
        self.assertFalse(self.output.exists())

    def test_missing_input_of_later_selected_branch_rejects_before_cleanup(self):
        state_path = self.workspace / ".conversion-state.json"
        state = json.loads(state_path.read_text())
        del state["stages"]["02"]
        for stage in self.pipeline.STAGE_REGISTRY[self.stage_idx("02.5"):]:
            state["stages"].pop(stage["id"], None)
        state_path.write_text(json.dumps(state))
        before = (self.workspace / "02.5_page_conversion_review/artifact.json").read_bytes()
        with self.assertRaisesRegex(ValueError, "Stage 02.5"):
            self.run_pipeline(["--from-stage", "02.5", "--to-stage", "02.5"],
                              lambda *args: self.fail("Worker must not run before all external inputs validate"))
        self.assertEqual((self.workspace / "02.5_page_conversion_review/artifact.json").read_bytes(), before)

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
        self.assertEqual(list(state["stages"]), ["00", "01", "02", "02.5", "03", "04", "05"])

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
    def test_raw_stream_consumes_page_objects_in_json_order(self):
        script = ROOT / "tools/bootstrap/pdf-to-markdown/stages/03_build_raw_stream/build_stream.py"
        spec = importlib.util.spec_from_file_location("raw_stream_worker", script)
        worker = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(worker)
        entry = {"page": 7, "width": 200, "height": 400}
        objects = {"image_width": 600, "image_height": 1200, "segments": [
            {"segment_id": "page_0007_seg_001", "type": "heading", "heading_level": 2,
             "continuation": True, "md_text": "## Continued", "bbox": [30, 900, 300, 990]},
            {"segment_id": "page_0007_seg_002", "type": "footnote", "heading_level": None,
             "continuation": False, "md_text": "*Source note*", "bbox": [60, 120, 360, 240]},
        ]}
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            with patch.object(worker, "validate_conversion", return_value=[(entry, objects)]) as validate, \
                 patch.object(worker, "extract_assets_for_nodes", side_effect=lambda root, nodes, **kwargs: nodes):
                worker.build_raw_stream(workspace, {})
            validate.assert_called_once_with(workspace)
            nodes = json.loads((workspace / "03_build_raw_stream/raw_stream.json").read_text())
            self.assertEqual([n["md_text"] for n in nodes], ["## Continued", "*Source note*"])
            self.assertEqual(nodes[0]["raw_text"], "## Continued")
            self.assertEqual(nodes[0]["bbox"], [10, 300, 100, 330])
            self.assertEqual(nodes[0]["bbox_pixels"], objects["segments"][0]["bbox"])
            self.assertTrue(nodes[0]["continuation"])
            self.assertEqual(nodes[1]["type"], "footnote")

    def test_raw_stream_invalid_input_does_not_write_output(self):
        script = ROOT / "tools/bootstrap/pdf-to-markdown/stages/03_build_raw_stream/build_stream.py"
        spec = importlib.util.spec_from_file_location("raw_stream_worker", script)
        worker = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(worker)
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            with patch.object(worker, "validate_conversion", side_effect=ValueError("Invalid Stage 02")):
                with self.assertRaisesRegex(ValueError, "Invalid Stage 02"):
                    worker.build_raw_stream(workspace, {})
            self.assertEqual(list(workspace.iterdir()), [])

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
        self.registry = [{"id": "00", "dir": "00_text_layer"}, {"id": "01", "dir": "01_preprocess"}, {"id": "02", "dir": "02_page_conversion"}]
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
        self.config["llm"]["stages"]["02_page_conversion"]["reasoning_effort"] = "high"
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

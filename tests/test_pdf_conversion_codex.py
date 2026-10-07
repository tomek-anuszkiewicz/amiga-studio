"""Offline PDF execution, restarts and preservation of stage-owned artifacts."""

from pathlib import Path
import importlib.util
import contextlib
import io
import json
import subprocess
import sys
import tempfile
import unittest
import pymupdf
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools/bootstrap"))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools/bootstrap/pdf-to-markdown"))
from conversion import load_config
from conversion.config import PDF_STAGES
import yaml
from common.pdf_artifacts import read_json, write_json

ROOT = Path(__file__).resolve().parents[1]


class PdfNxpGeometryTests(unittest.TestCase):
    def test_landscape_page_keeps_unrotated_logo_size(self):
        spec = importlib.util.spec_from_file_location(
            "filter_page_content", ROOT / "tools/bootstrap/pdf-to-markdown/stages/02.8_filter_page_content/filter_page_content.py")
        stage = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(stage)
        # Page 63 has a landscape canvas, but its standalone logo is unrotated.
        segment = {"type": "graphic", "bbox": [30, 37, 288, 131]}
        page = {"image_width": 3300, "image_height": 2550}
        self.assertTrue(stage.nxp_geometry_matches(segment, page, (257, 94)))


class PdfReviewContinuationTests(unittest.TestCase):
    def test_callout_assembly_does_not_repeat_batched_body_nodes(self):
        spec = importlib.util.spec_from_file_location(
            "format_prose", ROOT / "tools/bootstrap/pdf-to-markdown/stages/09_transform_prose/format_prose.py")
        prose = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(prose)
        nodes = [
            {"type": "callout", "raw_text": "**WARNING**"},
            {"type": "callout_text", "raw_text": "First paragraph.",
             "rendered_markdown": "First paragraph.\n\nSecond paragraph."},
            {"type": "callout_text", "raw_text": "Second paragraph.",
             "rendered_markdown": "", "continuation_status": "continuation"},
        ]
        self.assertEqual(prose.assemble_callouts(nodes), 1)
        self.assertEqual(nodes[0]["rendered_markdown"].count("Second paragraph."), 1)
        self.assertTrue(nodes[0]["rendered_markdown"].startswith("> [!WARNING]"))
        self.assertEqual([node["rendered_markdown"] for node in nodes[1:]], ["", ""])
        standalone = [
            {"type": "callout_text", "raw_text": "**WARNING:** First paragraph.",
             "rendered_markdown": "**WARNING:** First paragraph.\n\nSecond paragraph."},
            {"type": "callout_text", "raw_text": "Second paragraph.",
             "rendered_markdown": "", "continuation_status": "continuation"},
        ]
        self.assertEqual(prose.assemble_callouts(standalone), 1)
        self.assertTrue(standalone[0]["rendered_markdown"].startswith("> [!WARNING]"))
        self.assertEqual(standalone[1]["rendered_markdown"], "")

    def test_02_5_groups_prose_by_union_obstacles_only(self):
        spec = importlib.util.spec_from_file_location(
            "page_conversion_review", ROOT / "tools/bootstrap/pdf-to-markdown/stages/02.5_page_conversion_review/render_review.py")
        renderer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(renderer)
        # Empty union space permits joining regardless of alignment or distance.
        segments = [{"type": "prose", "bbox": box} for box in
                    [[10, 100, 150, 140], [180, 300, 400, 350], [20, 600, 80, 700]]]
        annotations = renderer.review_annotations(segments)
        self.assertEqual([item["source_ordinals"] for item in annotations], [[1, 2, 3]])
        self.assertEqual(annotations[0]["bbox"], [10, 100, 400, 700])
        # A blocker ends the current run; the next prose can start a new run.
        hidden = {"type": "header", "bbox": [160, 200, 170, 220]}
        blocked = renderer.review_annotations(segments + [hidden])
        self.assertEqual([item["source_ordinals"] for item in blocked], [[1], [2, 3]])

    def test_02_5_joins_same_column_prose_with_short_paragraphs(self):
        spec = importlib.util.spec_from_file_location(
            "page_conversion_review", ROOT / "tools/bootstrap/pdf-to-markdown/stages/02.5_page_conversion_review/render_review.py")
        renderer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(renderer)
        # Page 31: introduction and STATE 0-6 share a left-aligned flow, while
        # paragraph lengths and heights vary and some gaps exceed a line height.
        boxes = [[289, 1374, 1457, 1416], [290, 1438, 1896, 1525],
                 [290, 1571, 1768, 1613], [290, 1654, 1800, 1700],
                 [290, 1745, 1902, 1831], [290, 1874, 1908, 2193],
                 [292, 2243, 1211, 2286], [292, 2330, 1211, 2372]]
        segments = [{"type": "prose", "bbox": box, "segment_id": f"prose_{i}",
                     "continuation": i == 7, "md_text": f"Paragraph {i}."}
                    for i, box in enumerate(boxes)]
        before = json.dumps(segments)
        annotations = renderer.review_annotations(segments)
        self.assertEqual(len(annotations), 1)
        self.assertEqual(annotations[0]["bbox"], [289, 1374, 1908, 2372])
        self.assertEqual(annotations[0]["source_ordinals"], list(range(1, 9)))
        self.assertEqual(annotations[0]["source_ids"], [f"prose_{i}" for i in range(8)])
        self.assertTrue(annotations[0]["continuation"])
        self.assertEqual(json.dumps(segments), before)
        # A hidden object elsewhere in source order still blocks the union.
        hidden = {"type": "thumb_index", "bbox": [500, 1540, 600, 1560]}
        blocked = renderer.review_annotations(segments + [hidden])
        self.assertEqual([item["source_ordinals"] for item in blocked], [[1, 2], list(range(3, 9))])

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


class PdfOcrValidationTests(unittest.TestCase):
    def test_pipeline_directory_reuses_shared_pdf_and_restart_keeps_it(self):
        with tempfile.TemporaryDirectory() as directory:
            book = Path(directory)
            source = book / "source.pdf"
            with pymupdf.open() as document:
                for number in range(1, 4):
                    document.new_page(width=200, height=200).insert_text((20, 40), f"Page {number}")
                document.save(source)
            command = [sys.executable, str(ROOT / "tools/bootstrap/pdf-to-markdown/pipeline.py"),
                       "--config", str(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml")]
            first, second = book / "workspace/first", book / "workspace/second"
            result = subprocess.run(command + ["--pdf", str(book), "--workspace", str(first),
                                    "--page-ranges", "1-2", "--from-stage", "00", "--to-stage", "01"],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            prepared = book / "source-ocr.pdf"
            before, mtime = prepared.read_bytes(), prepared.stat().st_mtime_ns
            result = subprocess.run(command + ["--pdf", str(book), "--workspace", str(second),
                                    "--page-ranges", "2-3", "--from-stage", "00", "--to-stage", "01"],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("[skip] Stage 00", result.stdout)
            result = subprocess.run(command + ["--pdf", str(source), "--workspace", str(second), "--from-stage", "01", "--to-stage", "01"],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(prepared.read_bytes(), before)
            self.assertEqual(prepared.stat().st_mtime_ns, mtime)
            self.assertEqual(sorted(p.name for p in (second / "01_preprocess").glob("*.png")),
                             ["page_0001.png", "page_0002.png", "page_0003.png"])
            for suffix in ("json", "png"):
                self.assertEqual((first / f"01_preprocess/page_0002.{suffix}").read_bytes(),
                                 (second / f"01_preprocess/page_0002.{suffix}").read_bytes())

    def test_full_document_ocr_includes_textless_pages_outside_fragment(self):
        spec = importlib.util.spec_from_file_location(
            "prepare_ocr_fragment", ROOT / "tools/bootstrap/pdf-to-markdown/stages/00_text_layer/prepare_text_layer.py")
        stage = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(stage)
        config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml",
                             known_stages=PDF_STAGES, required_stages=())
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
                prepared_path = stage.prepare_text_layer(source, workspace, config, "2-3")
            self.assertEqual(recognize.call_count, 2)
            with pymupdf.open(prepared_path) as prepared:
                self.assertEqual(len(prepared), 3)
                self.assertEqual([page.get_text().strip() for page in prepared],
                                 ["Recovered text", "Native text", "Recovered text"])
            self.assertEqual(prepared_path.resolve(), source.with_stem("source-ocr"))

    def test_shared_full_pdf_gives_identical_page_inputs_across_fragments(self):
        def load_stage(name, relative):
            spec = importlib.util.spec_from_file_location(name, ROOT / relative)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            return module

        stage = load_stage("prepare_fragment", "tools/bootstrap/pdf-to-markdown/stages/00_text_layer/prepare_text_layer.py")
        preprocess = load_stage("preprocess_fragment", "tools/bootstrap/pdf-to-markdown/stages/01_preprocess/preprocess.py")
        config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml",
                             known_stages=PDF_STAGES, required_stages=())
        with tempfile.TemporaryDirectory() as directory:
            source, workspace = Path(directory) / "source.pdf", Path(directory) / "workspace"
            with pymupdf.open() as document:
                for number in range(1, 6):
                    page = document.new_page(width=200, height=200)
                    page.insert_text((20, 40), f"Source page {number}")
                document.save(source)
            original_hash = stage.file_hash(source)
            with patch.object(stage, "tesseract_ocr", side_effect=AssertionError("Native pages must not use OCR")), \
                    patch.object(pymupdf.Page, "get_pixmap", side_effect=AssertionError("Stage 00 must not render native pages for validation")):
                prepared_path = stage.prepare_text_layer(source, workspace, config, "2,5")
            prepared = prepared_path
            with pymupdf.open(prepared) as document:
                self.assertEqual(len(document), 5)
                self.assertEqual([page.get_text().strip() for page in document],
                                 [f"Source page {number}" for number in range(1, 6)])
            self.assertEqual(stage.file_hash(source), original_hash)
            preprocess.preprocess_pdf(prepared, workspace, dpi=72, page_ranges="2,5")
            page_data = json.loads((workspace / "01_preprocess/page_0005.json").read_text())
            self.assertNotIn("pdf_sha256", page_data)
            self.assertEqual(page_data["source_index"], 4)
            self.assertEqual(page_data["prepared_index"], 4)
            original_bytes, original_mtime = prepared.read_bytes(), prepared.stat().st_mtime_ns
            other = Path(directory) / "other-workspace"
            with patch.object(stage, "text_blocks", side_effect=AssertionError("Existing PDF must skip page processing")), \
                    patch.object(stage, "tesseract_settings", side_effect=AssertionError("Existing PDF must skip Tesseract setup")):
                reused = stage.prepare_text_layer(source, other, config, "1-2")
            self.assertEqual(prepared.read_bytes(), original_bytes)
            self.assertEqual(prepared.stat().st_mtime_ns, original_mtime)
            preprocess.preprocess_pdf(prepared, other, dpi=72, page_ranges="1-2")
            for suffix in ("json", "png"):
                self.assertEqual((workspace / f"01_preprocess/page_0002.{suffix}").read_bytes(),
                                 (other / f"01_preprocess/page_0002.{suffix}").read_bytes())

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

    def test_failed_ocr_pdf_publishes_no_recovery_or_output(self):
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
            self.assertFalse(source.with_stem("source-ocr").exists())


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
        self.pdf.write_bytes(b"Worker fixture")
        self.config["input"] = {"source_pdf": "../sample.pdf", "pages": list(range(5, 11))}
        (self.workspace / "config.yaml").write_text(yaml.safe_dump(self.config))
        for stage in self.pipeline.STAGE_REGISTRY:
            self.write_artifacts(stage)

    def write_artifacts(self, stage):
        directory = self.workspace / stage["dir"]
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "artifact.json").write_text('{"generated":true}')
        self.pipeline.update_status(self.workspace / "stage_status.json", stage["id"], "success")

    def stage_idx(self, stage_id):
        return next(index for index, stage in enumerate(self.pipeline.STAGE_REGISTRY) if stage["id"] == stage_id)

    def run_pipeline(self, flags, worker, *, page_ranges="5-10"):
        argv = ["pipeline.py", "--pdf", str(self.pdf), "--workspace", str(self.workspace), "--config", str(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml")]
        if page_ranges:
            argv.extend(["--page-ranges", page_ranges])
        argv.extend(flags)
        with patch.object(sys, "argv", argv), patch.object(self.pipeline, "run_stage", side_effect=worker):
            self.pipeline.main()

    def test_named_review_stage_restarts_by_registry_order(self):
        self.assertEqual(self.pipeline.resolve_stage_idx("02"), self.stage_idx("02"))
        self.assertEqual(self.pipeline.resolve_stage_idx("2.5"), self.stage_idx("02.5"))
        self.assertIsNone(self.pipeline.resolve_stage_idx("02k"))
        self.assertEqual(self.pipeline.resolve_stage_idx("02.5"), self.stage_idx("02.5"))
        before = read_json(self.workspace / "stage_status.json")["02"]
        def worker(stage, *args):
            self.assertEqual(stage["id"], "02.5")
            retained = read_json(self.workspace / "stage_status.json")
            self.assertEqual(set(retained), {"00", "01", "02", "02.4"})
            for later in self.pipeline.STAGE_REGISTRY[self.stage_idx("02.5"):]:
                self.assertFalse((self.workspace / later["dir"]).exists())
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "02.5", "--to-stage", "02.5"], worker)
        after = read_json(self.workspace / "stage_status.json")
        self.assertEqual(after["02"], before)
        self.assertEqual(after["02.5"]["status"], "success")
        self.assertNotIn("03", after)

    def test_partial_page_conversion_does_not_create_final_output(self):
        (self.output / "artifact.json").unlink()
        self.output.rmdir()
        state_path = self.workspace / "stage_status.json"
        state = json.loads(state_path.read_text())
        del state["14"]
        state_path.write_text(json.dumps(state))
        executed = []

        def worker(stage, *args):
            self.assertFalse(self.output.exists(), "Stage 14 directory must wait for Stage 14 execution")
            executed.append(stage["id"])
            self.write_artifacts(stage)
            return True

        self.run_pipeline(["--from-stage", "02", "--to-stage", "02.5"], worker)
        self.assertEqual(executed, ["02", "02.4", "02.5"])
        self.assertFalse(self.output.exists())

    def test_missing_input_of_later_selected_branch_rejects_before_cleanup(self):
        state_path = self.workspace / "stage_status.json"
        state = json.loads(state_path.read_text())
        del state["02"]
        for stage in self.pipeline.STAGE_REGISTRY[self.stage_idx("02.5"):]:
            state.pop(stage["id"], None)
        state_path.write_text(json.dumps(state))
        before = (self.workspace / "02.5_page_conversion_review/artifact.json").read_bytes()
        with self.assertRaisesRegex(ValueError, "Stage 02.5"):
            self.run_pipeline(["--from-stage", "02.5", "--to-stage", "02.5"],
                              lambda *args: self.fail("Worker must not run before all external inputs validate"))
        self.assertEqual((self.workspace / "02.5_page_conversion_review/artifact.json").read_bytes(), before)

    def test_restart_five_clears_dependents_and_preserves_prior_outputs(self):
        before = (self.workspace / "04_stream_reduction/artifact.json").read_bytes()
        other_output = Path(self.tmp.name) / "other-workspace/14_link_toc/accepted.md"
        other_output.parent.mkdir(parents=True)
        other_output.write_text("Independent conversion")
        def worker(stage, *args):
            self.assertEqual(stage["id"], "05")
            for dependent in self.pipeline.STAGE_REGISTRY[self.stage_idx("05"):]:
                self.assertFalse((self.workspace / dependent["dir"]).exists())
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "5", "--to-stage", "5"], worker)
        self.assertEqual((self.workspace / "04_stream_reduction/artifact.json").read_bytes(), before)
        self.assertEqual(other_output.read_text(), "Independent conversion")
        state = read_json(self.workspace / "stage_status.json")
        self.assertEqual(list(state), ["00", "01", "02", "02.4", "02.5", "02.8", "02.81", "02.82", "02.9", "03", "04", "05"])


    def test_explicit_restart_rebuilds_missing_output_and_later_stages(self):
        (self.workspace / "05_chapter_partition/artifact.json").unlink()
        executed = []
        def worker(stage, *args):
            executed.append(stage["id"])
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "05"], worker)
        self.assertEqual(executed, [stage["id"] for stage in self.pipeline.STAGE_REGISTRY[self.stage_idx("05"):]])

    def test_explicit_page_range_is_run_scoped(self):
        def worker(stage, *args):
            self.assertEqual(args[-1], "6,7,8,9,10")
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "5", "--to-stage", "5", "--page-ranges", "6-10"],
                          worker)
        config = yaml.safe_load((self.workspace / "config.yaml").read_text())
        self.assertNotIn("input", config)

    def test_interrupted_cleanup_has_already_invalidated_completion(self):
        with patch.object(self.pipeline, "clean_downstream_stages", side_effect=OSError("Interrupted cleanup")):
            with self.assertRaisesRegex(OSError, "Interrupted cleanup"):
                self.run_pipeline(["--from-stage", "5", "--to-stage", "5"],
                                  lambda *args: self.fail("Worker must not run"))
        state = read_json(self.workspace / "stage_status.json")
        self.assertNotIn("05", state)
        self.assertNotIn("14", state)
        self.assertIn("04", state)

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

    def test_missing_retained_artifact_does_not_block_restart(self):
        (self.workspace / "03_build_raw_stream/artifact.json").unlink()
        executed = []
        def worker(stage, *args):
            executed.append(stage["id"])
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "5", "--to-stage", "5"], worker)
        self.assertEqual(executed, ["05"])

    def test_omitted_page_range_ignores_previous_config_selection(self):
        (self.workspace / "01_preprocess/artifact.json").unlink()
        pdf = Path(self.tmp.name) / "sample.pdf"
        executed = []
        def worker(stage, *args):
            executed.append(stage["id"])
            if stage["id"] == "01":
                self.assertIsNone(args[-1])
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "01", "--pdf", str(pdf)], worker, page_ranges=None)
        self.assertEqual(executed[0], "01")
        state = read_json(self.workspace / "stage_status.json")
        self.assertNotIn("input", yaml.safe_load((self.workspace / "config.yaml").read_text()))

    def test_resume_option_is_rejected_without_cleanup(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
            self.run_pipeline(["--resume"], lambda *args: self.fail("Worker must not run"))
        self.assertEqual(error.exception.code, 2)
        self.assertTrue((self.output / "artifact.json").exists())

    def test_individual_stage_uses_matching_interval(self):
        self.run_pipeline(["--from-stage", "02.5", "--to-stage", "02.5"],
                          lambda stage, *args: self.write_artifacts(stage) or True)
        self.assertEqual(set(read_json(self.workspace / "stage_status.json")), {"00", "01", "02", "02.4", "02.5"})
        self.assertFalse(self.output.exists())


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
            with patch.object(worker, "read_conversion", return_value=[(entry, objects)]) as validate, \
                 patch.object(worker, "extract_assets_for_nodes", side_effect=lambda root, nodes, **kwargs: nodes):
                worker.build_raw_stream(workspace, {})
            validate.assert_called_once_with(workspace, "02.81_transform_page_tables")
            nodes = json.loads((workspace / "03_build_raw_stream/raw_stream.json").read_text())
            self.assertEqual([n["md_text"] for n in nodes], ["## Continued", "*Source note*"])
            self.assertEqual(nodes[0]["raw_text"], "## Continued")
            self.assertEqual(nodes[0]["bbox"], [10, 300, 100, 330])
            self.assertEqual(nodes[0]["bbox_pixels"], objects["segments"][0]["bbox"])
            self.assertTrue(nodes[0]["continuation"])
            self.assertEqual(nodes[1]["type"], "footnote")

    def test_raw_stream_read_failure_does_not_write_output(self):
        script = ROOT / "tools/bootstrap/pdf-to-markdown/stages/03_build_raw_stream/build_stream.py"
        spec = importlib.util.spec_from_file_location("raw_stream_worker", script)
        worker = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(worker)
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            with patch.object(worker, "read_conversion", side_effect=OSError("Unreadable Stage 02")):
                with self.assertRaisesRegex(OSError, "Unreadable Stage 02"):
                    worker.build_raw_stream(workspace, {})
            self.assertEqual(list(workspace.iterdir()), [])


    def test_all_inference_stage_pairs_are_explicit(self):
        config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml", known_stages=PDF_STAGES, required_stages=PDF_STAGES)
        self.assertEqual(set(config["llm"]["stages"]), PDF_STAGES)
        self.assertNotIn("00_text_layer", PDF_STAGES)
        self.assertNotIn("01_preprocess", PDF_STAGES)


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


class StageCompletionTests(unittest.TestCase):
    def load(self, name, relative):
        spec = importlib.util.spec_from_file_location(name, ROOT / "tools/bootstrap/pdf-to-markdown" / relative)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_page_worker_receives_only_current_cli_range(self):
        pipeline = self.load("selection_pipeline", "pipeline.py")
        stage = next(item for item in pipeline.STAGE_REGISTRY if item["id"] == "02.4")
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            args = (stage, ROOT / "tools/bootstrap/pdf-to-markdown", workspace,
                    None, None, workspace / "config.yaml")
            with patch.object(pipeline.subprocess, "run") as run:
                self.assertTrue(pipeline.run_stage(*args, page_ranges="64"))
                command = run.call_args.args[0]
                self.assertEqual(command[command.index("--page-ranges") + 1], "64")
                self.assertTrue(pipeline.run_stage(*args))
                self.assertNotIn("--page-ranges", run.call_args.args[0])

    def test_worker_and_asset_copy_failures_record_failed_execution(self):
        pipeline = self.load("execution_pipeline", "pipeline.py")
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            stage = next(item for item in pipeline.STAGE_REGISTRY if item["id"] == "05")
            (workspace / "04_stream_reduction/assets").mkdir(parents=True)
            args = (stage, ROOT / "tools/bootstrap/pdf-to-markdown", workspace, None, None, workspace / "config.yaml")
            with patch.object(pipeline.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "worker")):
                self.assertFalse(pipeline.run_stage(*args))
            self.assertEqual(read_json(workspace / "stage_status.json")["05"]["status"], "failed")
            with patch.object(pipeline.subprocess, "run"), patch.object(pipeline.shutil, "copytree", side_effect=OSError("copy failed")):
                self.assertFalse(pipeline.run_stage(*args))
            status = read_json(workspace / "stage_status.json")["05"]
            self.assertEqual(status["status"], "failed")
            self.assertIn("copy failed", status["details"])

    def test_automatic_chapter_workers_preserve_metadata_and_prior_outputs(self):
        from unittest.mock import MagicMock
        stages = [
            ("06_detect_continuations", "detect_continuations.py", "process_chapter_continuations"),
            ("07_transform_tables", "transform_tables.py", "process_tables"),
            ("08_transform_graphics", "transform_graphics.py", "process_graphics"),
            ("09_transform_prose", "format_prose.py", "process_prose"),
        ]
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml", known_stages=PDF_STAGES)
            predecessor = workspace / "05_chapter_partition"
            predecessor.mkdir()
            for slug in ("preface", "toc"):
                chapter = {"index": 0, "slug": slug, "title": slug.title(), "target_md_file": f"00 - {slug}.md",
                           "nodes": [{"node_id": "node_00001", "page": 1, "type": "table", "raw_text": "First rows"},
                                     {"node_id": "node_00002", "page": 2, "type": "table", "raw_text": "Later rows"},
                                     {"node_id": "node_00003", "page": 2, "type": "graphic", "raw_text": "Diagram", "png_path": "figure.png"},
                                     {"node_id": "node_00004", "page": 2, "type": "prose", "raw_text": "Source text"}]}
                write_json(predecessor / f"00_{slug}.json", chapter)
            for stage, script, function in stages:
                worker = self.load(stage, f"stages/{stage}/{script}")
                before = {path.name: path.read_bytes() for path in predecessor.glob("*.json")}
                fake = MagicMock()
                fake.selected.model = "offline-fixture"
                fake.generate_text.return_value = "Formatted text"
                fake.generate_vision.return_value = "Diagram description"
                fake.generate_json.return_value = {"is_continuation": True, "type": "schematic", "caption": None}
                with patch.object(worker, "CodexClient") as client:
                    client.return_value.__enter__.return_value = fake
                    getattr(worker, function)(workspace, config)
                output = workspace / stage
                for filename, value in before.items():
                    self.assertEqual((predecessor / filename).read_bytes(), value)
                    previous, current = json.loads(value), read_json(output / filename)
                    self.assertEqual({key: current[key] for key in ("index", "slug", "title", "target_md_file")},
                                     {key: previous[key] for key in ("index", "slug", "title", "target_md_file")})
                if stage.startswith("09"):
                    self.assertEqual(fake.generate_text.call_count, 2)
                elif stage.startswith("06") or stage.startswith("08"):
                    self.assertEqual(fake.generate_json.call_count, 2)
                else:
                    self.assertEqual(fake.generate_text.call_count, 2)
                predecessor = output
            proofreader = self.load("chapter_proofreader", "stages/10_proofread_stream/proofread_stream.py")
            before = {path.name: path.read_bytes() for path in predecessor.glob("*.json")}
            fake.generate_text.return_value = "Shared title"
            with patch.object(proofreader, "CodexClient") as client:
                client.return_value.__enter__.return_value = fake
                proofreader.process_proofread_stream(workspace, predecessor, workspace / "10_proofread_stream", config)
            for filename, value in before.items():
                self.assertEqual((predecessor / filename).read_bytes(), value)
            preface = read_json(workspace / "10_proofread_stream/00_preface.json")
            toc = read_json(workspace / "10_proofread_stream/00_toc.json")
            self.assertEqual(preface["title"], "Shared title")
            self.assertNotEqual(preface["target_md_file"], toc["target_md_file"])
            emitter = self.load("chapter_emitter", "stages/11_emit_markdown/emit_markdown.py")
            emitter.emit_markdown(workspace, workspace / "11_emit_markdown", config)
            self.assertEqual(len(list((workspace / "11_emit_markdown").glob("*.md"))), 2)


if __name__ == "__main__":
    unittest.main()

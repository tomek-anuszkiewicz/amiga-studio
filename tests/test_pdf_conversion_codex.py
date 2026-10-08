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
from common.pdf_config import load_config
import yaml
from common.pdf_artifacts import read_json, write_json
from common.pdf_artifacts import compact_objects
from conversion import CodexClient
from conversion.cache import ResponseCache, identity
from unittest.mock import Mock

ROOT = Path(__file__).resolve().parents[1]


class PdfArtifactIdentityTests(unittest.TestCase):

    def test_compact_artifacts_preserve_source_and_meaningful_fields(self):
        obj = {"type": "graphic", "bbox": [1, 2, 3, 4], "segment_id": "g",
               "continuation": False, "heading_level": None, "md_text": "",
               "rotation": 0, "source_segment_ids": ["s"],
               "replacement_stage": "02.44", "image_only": True,
               "metadata": {"rotation": 0, "other": "retained"}}
        meaningful = {**obj, "rotation": 90.5, "md_text": "Text", "heading_level": 2,
                      "continuation": True, "metadata": {"rotation": 15}}
        value = {"segments": [obj, meaningful, {**obj, "md_text": None}],
                 "blocks": [{"type": 0, "bbox": [0, 0, 1, 1], "text": "OCR"}]}
        before = json.dumps(value, ensure_ascii=False)
        compact = compact_objects(value)
        self.assertEqual(json.dumps(value, ensure_ascii=False), before)
        self.assertEqual(compact["segments"][0], {
            "type": "graphic", "bbox": [1, 2, 3, 4], "segment_id": "g",
            "metadata": {"other": "retained"}})
        self.assertEqual(compact["segments"][2], compact["segments"][0])
        self.assertEqual(compact["blocks"], value["blocks"])
        for key in ("rotation", "md_text", "heading_level", "continuation", "metadata"):
            self.assertEqual(compact["segments"][1][key], meaningful[key])

    def test_compact_artifact_prompt_replays_preloaded_cache(self):
        stage = "02_page_conversion"
        config = {"llm": {"stages": {stage: {"model": "gpt-6.1-sol", "reasoning_effort": "medium"}}}}
        schema = {"type": "object"}
        # This identity is the compact alias prepared before artifact migration.
        seeded_prompt = 'Frozen page:\n{"segments": [{"type": "graphic", "bbox": [1, 2, 3, 4]}]}'
        result = {"status": "completed", "text": '{"rotation": 0}', "usage": {"inputTokens": 12}}
        with tempfile.TemporaryDirectory() as directory:
            cache = ResponseCache(Path(directory) / "cache")
            transport = Mock()
            transport.run.side_effect = AssertionError("Preloaded cache must avoid inference")
            with CodexClient(config, stage=stage, cache=cache, transport=transport) as client:
                cache.save(identity(client.selected, seeded_prompt, [], schema), result)
                path = Path(directory) / "page.json"
                write_json(path, {"segments": [{"type": "graphic", "bbox": [1, 2, 3, 4],
                                              "rotation": 0, "md_text": None,
                                              "continuation": False, "heading_level": None,
                                              "source_segment_ids": ["s"],
                                              "replacement_stage": "02.44", "image_only": True}]})
                prompt = "Frozen page:\n" + json.dumps(read_json(path), ensure_ascii=False)
                self.assertEqual(prompt, seeded_prompt)
                self.assertEqual(client.generate_json(prompt, schema=schema), {"rotation": 0})
                transport.run.assert_not_called()
                self.assertEqual((client.call_count, client.cached_call_count), (0, 1))

    def test_read_preserves_extracted_text_prompt_data(self):
        value = {"blocks": [{"type": 0, "bbox": [1, 2, 3, 4], "text": "Source"}]}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "page.json"
            path.write_text(json.dumps(value, ensure_ascii=False), encoding="utf-8")
            self.assertEqual(json.dumps(read_json(path), ensure_ascii=False),
                             json.dumps(value, ensure_ascii=False))

    def test_write_omits_explicit_object_defaults(self):
        value = {"segments": [{"type": "graphic", "bbox": [1, 2, 3, 4],
                              "continuation": False, "heading_level": None,
                              "md_text": "", "rotation": 0}]}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "page.json"
            write_json(path, value)
            self.assertEqual(path.read_text(encoding="utf-8"),
                             json.dumps({"segments": [{"type": "graphic", "bbox": [1, 2, 3, 4]}]},
                                        indent=2, ensure_ascii=False))


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
        config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml")
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
        config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml")
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
        config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml")
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
        self.output = self.workspace / "02.81_transform_page_tables"
        self.config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml")
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

    def test_filter_stage_restarts_by_registry_order(self):
        self.assertEqual(self.pipeline.resolve_stage_idx("02"), self.stage_idx("02"))
        self.assertEqual(self.pipeline.resolve_stage_idx("2.80"), self.stage_idx("02.80"))
        self.assertIsNone(self.pipeline.resolve_stage_idx("02k"))
        self.assertIsNone(self.pipeline.resolve_stage_idx("02.90"))
        self.assertIsNone(self.pipeline.resolve_stage_idx("2.9"))
        self.assertEqual(self.pipeline.STAGE_REGISTRY[-1]["id"], "02.81")
        self.assertEqual(self.pipeline.resolve_stage_idx("02.80"), self.stage_idx("02.80"))
        before = read_json(self.workspace / "stage_status.json")["02"]
        def worker(stage, *args):
            self.assertEqual(stage["id"], "02.80")
            retained = read_json(self.workspace / "stage_status.json")
            self.assertEqual(set(retained), {"00", "01", "02"})
            for later in self.pipeline.STAGE_REGISTRY[self.stage_idx("02.80"):]:
                self.assertFalse((self.workspace / later["dir"]).exists())
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "02.80", "--to-stage", "02.80"], worker)
        after = read_json(self.workspace / "stage_status.json")
        self.assertEqual(after["02"], before)
        self.assertEqual(after["02.80"]["status"], "success")
        self.assertNotIn("02.81", after)

    def test_partial_page_conversion_does_not_create_final_output(self):
        (self.output / "artifact.json").unlink()
        self.output.rmdir()
        state_path = self.workspace / "stage_status.json"
        state = json.loads(state_path.read_text())
        del state["02.81"]
        state_path.write_text(json.dumps(state))
        executed = []

        def worker(stage, *args):
            self.assertFalse(self.output.exists(), "Table output must wait for Stage 02.81 execution")
            executed.append(stage["id"])
            self.write_artifacts(stage)
            return True

        self.run_pipeline(["--from-stage", "02", "--to-stage", "02.80"], worker)
        self.assertEqual(executed, ["02", "02.80"])
        self.assertFalse(self.output.exists())

    def test_missing_input_of_later_selected_branch_rejects_before_cleanup(self):
        state_path = self.workspace / "stage_status.json"
        state = json.loads(state_path.read_text())
        del state["02"]
        for stage in self.pipeline.STAGE_REGISTRY[self.stage_idx("02.80"):]:
            state.pop(stage["id"], None)
        state_path.write_text(json.dumps(state))
        before = (self.workspace / "02.80_filter_page_content/artifact.json").read_bytes()
        with self.assertRaisesRegex(ValueError, "Stage 02.80"):
            self.run_pipeline(["--from-stage", "02.80", "--to-stage", "02.80"],
                              lambda *args: self.fail("Worker must not run before all external inputs validate"))
        self.assertEqual((self.workspace / "02.80_filter_page_content/artifact.json").read_bytes(), before)

    def test_restart_tables_clears_dependents_and_preserves_prior_outputs(self):
        before = (self.workspace / "02.80_filter_page_content/artifact.json").read_bytes()
        other_output = Path(self.tmp.name) / "other-workspace/02.81_transform_page_tables/accepted.md"
        other_output.parent.mkdir(parents=True)
        other_output.write_text("Independent conversion")
        def worker(stage, *args):
            self.assertEqual(stage["id"], "02.81")
            for dependent in self.pipeline.STAGE_REGISTRY[self.stage_idx("02.81"):]:
                self.assertFalse((self.workspace / dependent["dir"]).exists())
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "02.81", "--to-stage", "02.81"], worker)
        self.assertEqual((self.workspace / "02.80_filter_page_content/artifact.json").read_bytes(), before)
        self.assertEqual(other_output.read_text(), "Independent conversion")
        state = read_json(self.workspace / "stage_status.json")
        self.assertEqual(list(state), ["00", "01", "02", "02.80", "02.81"])


    def test_explicit_restart_rebuilds_missing_output_and_later_stages(self):
        (self.workspace / "02.81_transform_page_tables/artifact.json").unlink()
        executed = []
        def worker(stage, *args):
            executed.append(stage["id"])
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "02.81"], worker)
        self.assertEqual(executed, [stage["id"] for stage in self.pipeline.STAGE_REGISTRY[self.stage_idx("02.81"):]])

    def test_explicit_page_range_is_run_scoped(self):
        def worker(stage, *args):
            self.assertEqual(args[-1], "6,7,8,9,10")
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "02.81", "--to-stage", "02.81", "--page-ranges", "6-10"],
                          worker)
        config = yaml.safe_load((self.workspace / "config.yaml").read_text())
        self.assertNotIn("input", config)

    def test_interrupted_cleanup_has_already_invalidated_completion(self):
        with patch.object(self.pipeline, "clean_downstream_stages", side_effect=OSError("Interrupted cleanup")):
            with self.assertRaisesRegex(OSError, "Interrupted cleanup"):
                self.run_pipeline(["--from-stage", "02.81", "--to-stage", "02.81"],
                                  lambda *args: self.fail("Worker must not run"))
        state = read_json(self.workspace / "stage_status.json")
        self.assertNotIn("02.81", state)
        self.assertIn("02.80", state)

    def test_reset_tables_does_not_delete_preserved_assets(self):
        asset = self.workspace / "02.80_filter_page_content/assets/source.png.txt"
        asset.parent.mkdir()
        asset.write_text("Preserved source evidence")
        self.pipeline.clean_downstream_stages(self.workspace, self.stage_idx("02.81"), self.workspace / "stage_status.json")
        self.assertTrue(asset.exists())


    def test_missing_retained_artifact_does_not_block_restart(self):
        (self.workspace / "02.80_filter_page_content/artifact.json").unlink()
        executed = []
        def worker(stage, *args):
            executed.append(stage["id"])
            self.write_artifacts(stage)
            return True
        self.run_pipeline(["--from-stage", "02.81", "--to-stage", "02.81"], worker)
        self.assertEqual(executed, ["02.81"])

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
        self.run_pipeline(["--from-stage", "02.80", "--to-stage", "02.80"],
                          lambda stage, *args: self.write_artifacts(stage) or True)
        self.assertEqual(set(read_json(self.workspace / "stage_status.json")), {"00", "01", "02", "02.80"})
        self.assertFalse(self.output.exists())


class PdfConfigurationTests(unittest.TestCase):


    def test_all_inference_stage_pairs_are_explicit(self):
        config = load_config(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml")
        self.assertEqual(set(config["llm"]["stages"]), {"02_page_conversion", "02.81_transform_page_tables"})


    def test_missing_predecessor_does_not_clear_downstream(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            workspace = root / "workspace"
            output = workspace / "02.81_transform_page_tables"
            output.mkdir(parents=True)
            sentinel = output / "accepted.md"
            sentinel.write_text("Accepted output")
            result = subprocess.run([sys.executable, str(ROOT / "tools/bootstrap/pdf-to-markdown/pipeline.py"),
                                     "--config", str(ROOT / "tools/bootstrap/pdf-to-markdown/config.yaml"),
                                     "--workspace", str(workspace), "--pdf", str(root / "source.pdf"), "--from-stage", "02.81", "--to-stage", "02.81"], capture_output=True, text=True)
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
        stage = next(item for item in pipeline.STAGE_REGISTRY if item["id"] == "02.81")
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            args = (stage, ROOT / "tools/bootstrap/pdf-to-markdown", workspace,
                    None, workspace / "config.yaml")
            with patch.object(pipeline.subprocess, "run") as run:
                self.assertTrue(pipeline.run_stage(*args, page_ranges="64"))
                command = run.call_args.args[0]
                self.assertEqual(command[command.index("--page-ranges") + 1], "64")
                self.assertEqual(read_json(workspace / "stage_status.json")["02.81"]["page_ranges"], "64")
                self.assertTrue(pipeline.run_stage(*args))
                self.assertNotIn("--page-ranges", run.call_args.args[0])
                self.assertIsNone(read_json(workspace / "stage_status.json")["02.81"]["page_ranges"])

    def test_worker_failures_record_failed_execution(self):
        pipeline = self.load("execution_pipeline", "pipeline.py")
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            stage = next(item for item in pipeline.STAGE_REGISTRY if item["id"] == "02.81")
            (workspace / "02.80_filter_page_content/assets").mkdir(parents=True)
            args = (stage, ROOT / "tools/bootstrap/pdf-to-markdown", workspace, None, workspace / "config.yaml")
            with patch.object(pipeline.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "worker")):
                self.assertFalse(pipeline.run_stage(*args))
            self.assertEqual(read_json(workspace / "stage_status.json")["02.81"]["status"], "failed")
            with patch.object(pipeline.subprocess, "run", side_effect=OSError("worker launch failed")):
                self.assertFalse(pipeline.run_stage(*args))
            status = read_json(workspace / "stage_status.json")["02.81"]
            self.assertEqual(status["status"], "failed")
            self.assertIn("worker launch failed", status["details"])


if __name__ == "__main__":
    unittest.main()

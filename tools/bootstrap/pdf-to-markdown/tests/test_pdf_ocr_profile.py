"""The text-layer backend checks configured language data and records preparation."""

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[4]
SPEC = importlib.util.spec_from_file_location(
    "prepare_text_layer", ROOT / "tools/bootstrap/pdf-to-markdown/stages/01_text_layer/prepare_text_layer.py")
STAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(STAGE)


class OcrProfileTests(unittest.TestCase):
    def setUp(self):
        self.cache = tempfile.TemporaryDirectory()
        self.addCleanup(self.cache.cleanup)
        cache = patch("common.pdf_cache.DEFAULT_CACHE", Path(self.cache.name))
        cache.start()
        self.addCleanup(cache.stop)

    def test_rejects_changed_language_data_before_ocr(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / "eng.traineddata").write_bytes(b"different model")
            with patch.object(STAGE.pymupdf, "get_tessdata", return_value=directory):
                with self.assertRaisesRegex(RuntimeError, "eng.traineddata SHA-256"):
                    STAGE.tesseract_settings(STAGE.preparation_settings({}))

    def test_restores_cached_pdf_and_actual_preparation_record_without_ocr(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.pdf"
            with STAGE.pymupdf.open() as document:
                document.new_page().insert_text((72, 72), "Existing native text")
                document.save(source)
            output = STAGE.prepare_text_layer(source, Path(directory) / "workspace", {})
            record_path = output.with_suffix(".json")
            record = json.loads(record_path.read_text(encoding="utf-8"))
            self.assertEqual(record["source_sha256"], STAGE.file_digest(source))
            self.assertEqual(record["prepared_sha256"], STAGE.file_digest(output))
            self.assertEqual(record["profile"], STAGE.preparation_settings({}))
            self.assertEqual(record["ocr_pages"], [])
            prepared_bytes, record_bytes = output.read_bytes(), record_path.read_bytes()
            output.unlink()
            record_path.unlink()
            with patch.object(STAGE.pymupdf, "open", side_effect=AssertionError("Must use cache")):
                STAGE.prepare_text_layer(source, Path(directory) / "other", {})
            self.assertEqual(output.read_bytes(), prepared_bytes)
            self.assertEqual(record_path.read_bytes(), record_bytes)

    def test_source_and_ocr_profile_changes_invalidate_existing_preparation(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.pdf"
            workspace = Path(directory) / "workspace"
            with STAGE.pymupdf.open() as document:
                document.new_page().insert_text((72, 72), "First source")
                document.save(source)
            output = STAGE.prepare_text_layer(source, workspace, {})
            record_path = output.with_suffix(".json")
            first = json.loads(record_path.read_text(encoding="utf-8"))
            changed = {"rendering": {"dpi": 72}, "stages": {
                "01_text_layer": {"traineddata_sha256": "changed-model-hash"}}}
            STAGE.prepare_text_layer(source, workspace, changed)
            second = json.loads(record_path.read_text(encoding="utf-8"))
            self.assertEqual(second["profile"], STAGE.preparation_settings(changed))
            self.assertNotEqual(first["profile"], second["profile"])
            with STAGE.pymupdf.open() as document:
                document.new_page().insert_text((72, 72), "Changed source")
                source.write_bytes(document.tobytes())
            STAGE.prepare_text_layer(source, workspace, changed)
            third = json.loads(record_path.read_text(encoding="utf-8"))
            self.assertNotEqual(second["source_sha256"], third["source_sha256"])
            with STAGE.pymupdf.open(output) as document:
                self.assertIn("Changed source", document[0].get_text())


if __name__ == "__main__":
    unittest.main()

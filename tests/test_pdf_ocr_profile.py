"""The text-layer backend checks configured language data and records preparation."""

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "prepare_text_layer", ROOT / "tools/bootstrap/pdf-to-markdown/stages/01_text_layer/prepare_text_layer.py")
STAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(STAGE)


class OcrProfileTests(unittest.TestCase):
    def test_rejects_changed_language_data_before_ocr(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory) / "eng.traineddata").write_bytes(b"different model")
            with patch.object(STAGE.pymupdf, "get_tessdata", return_value=directory):
                with self.assertRaisesRegex(RuntimeError, "eng.traineddata SHA-256"):
                    STAGE.tesseract_settings(STAGE.preparation_settings({}))

    def test_records_actual_preparation_without_relabeling_existing_pdf(self):
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
            record_path.unlink()
            with patch.object(STAGE, "preparation_settings", side_effect=AssertionError("Must skip")):
                STAGE.prepare_text_layer(source, Path(directory) / "other", {})
            self.assertFalse(record_path.exists())


if __name__ == "__main__":
    unittest.main()

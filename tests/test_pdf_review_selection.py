"""Configured physical-page selection on retained all-page predecessors."""

import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
PDF = ROOT / "tools/bootstrap/pdf-to-markdown"
sys.path.insert(0, str(ROOT / "tools/bootstrap"))
sys.path.insert(0, str(PDF))


def stage_module(directory, script):
    spec = importlib.util.spec_from_file_location(script, PDF / "stages" / directory / f"{script}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ReviewSelectionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.workspace = Path(self.temporary.name)
        for directory in ("01_preprocess", "02_page_conversion"):
            (self.workspace / directory).mkdir()
        for page in (63, 64):
            identity = f"page_{page:04d}"
            value = {"page": page, "segments": [{"segment_id": "s1", "type": "prose",
                      "md_text": "NOTE: Advisory.", "bbox": [0, 0, 10, 10]}]}
            (self.workspace / "02_page_conversion" / f"{identity}_segments.json").write_text(json.dumps(value))
            (self.workspace / "01_preprocess" / f"{identity}.json").write_text(json.dumps({"page": page, "page_id": identity}))
            Image.new("RGB", (10, 10), "white").save(self.workspace / "01_preprocess" / f"{identity}.png")

    def test_page_conversion_requests_only_configured_page(self):
        stage = stage_module("02_page_conversion", "convert_page")
        for path in (self.workspace / "01_preprocess").glob("*.json"):
            value = json.loads(path.read_text())
            value["raster"] = {"pixel_width": 10, "pixel_height": 10}
            path.write_text(json.dumps(value))
        with patch.object(stage, "CodexClient") as client:
            transport = client.return_value.__enter__.return_value
            transport.generate_json.return_value = {"page": 64, "segments": []}
            stage.convert_pages(self.workspace, {"input": {"pages": [64]}})
        self.assertEqual(transport.generate_json.call_count, 1)
        self.assertEqual(transport.generate_json.call_args.kwargs["image_path"].name, "page_0064.png")
        self.assertEqual([p.name for p in (self.workspace / "02_page_conversion").glob("*_segments.json")
                          if json.loads(p.read_text())["segments"] == []], ["page_0064_segments.json"])

    def test_filter_selects_pages_before_toc_boundary(self):
        stage = stage_module("02.8_filter_page_content", "filter_page_content")
        # An unselected later TOC must not discard selected page 64's prose.
        path = self.workspace / "02_page_conversion/page_0065_segments.json"
        path.write_text(json.dumps({"page": 65, "segments": [{"type": "toc_heading"}]}))
        config_path = self.workspace / "config.yaml"
        config_path.write_text("input:\n  pages: [64]\n")
        with patch.object(sys, "argv", ["filter_page_content.py", "--workspace", str(self.workspace),
                                        "--config", str(config_path)]):
            stage.main()
        output = self.workspace / "02.8_filter_page_content"
        self.assertEqual([p.name for p in output.iterdir()], ["page_0064_segments.json"])

    def table_pages(self, directory):
        target = self.workspace / directory
        target.mkdir(exist_ok=True)
        for page in (63, 64):
            identity = f"page_{page:04d}_table"
            table = {"segment_id": identity, "type": "table", "md_text": "source",
                     "bbox": [0, 0, 10, 10], "table_format": "unconverted"}
            (target / f"page_{page:04d}_segments.json").write_text(json.dumps({"page": page, "segments": [table]}))

    def test_table_conversion_requests_only_configured_page(self):
        stage = stage_module("02.81_transform_page_tables", "transform_page_tables")
        self.table_pages("02.8_filter_page_content")
        with patch.object(stage, "CodexClient") as client:
            transport = client.return_value.__enter__.return_value
            transport.generate_json.return_value = {"format": "unconverted", "md_text": "source"}
            stage.transform_tables(self.workspace, {"input": {"pages": [64]}})
        self.assertEqual(transport.generate_json.call_count, 1)
        output = self.workspace / stage.STAGE
        self.assertEqual([p.name for p in output.glob("*_segments.json")], ["page_0064_segments.json"])

    def test_table_review_renders_only_configured_page(self):
        stage = stage_module("02.82_table_conversion_review", "render_table_review")
        self.table_pages(stage.STAGE)
        with patch.object(stage, "sync_playwright") as backend, patch.object(stage, "comparison_html", return_value="local") as comparison:
            browser = backend.return_value.__enter__.return_value.chromium.launch.return_value
            page = browser.new_page.return_value
            page.evaluate.return_value = 100
            stage.render_reviews(self.workspace, {"input": {"pages": [64]}})
        self.assertEqual(comparison.call_count, 1)
        self.assertEqual(comparison.call_args.args[2]["page"], 64)
        self.assertEqual(page.screenshot.call_count, 1)
        self.assertEqual(Path(page.screenshot.call_args.kwargs["path"]).name, "page_0064_table_review.png")

    def test_callout_requests_only_configured_page(self):
        stage = stage_module("02.4_reclassify_callouts", "reclassify_callouts")
        with patch.object(stage, "CodexClient") as client:
            client.return_value.generate_json.return_value = {"replacements": []}
            stage.reclassify_callouts(self.workspace, {"input": {"pages": [64]}})
        calls = client.return_value.generate_json.call_args_list
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0].kwargs["image_path"].name, "page_0064.png")

    def test_review_selects_configured_page_and_resolves_override(self):
        stage = stage_module("02.5_page_conversion_review", "render_review")
        override = self.workspace / "02.4_reclassify_callouts"
        override.mkdir()
        (self.workspace / "stage_status.json").write_text(json.dumps({"02.4": {"status": "success"}}))
        (override / "page_0064_segments.json").write_text(json.dumps({"page": 64, "segments": []}))
        config_path = self.workspace / "config.yaml"
        config_path.write_text("input:\n  pages: [64]\n")
        with patch.object(stage, "review_image", return_value=Image.new("RGB", (10, 10))) as render:
            with patch.object(sys, "argv", ["render_review.py", "--workspace", str(self.workspace),
                                            "--config", str(config_path)]):
                stage.main()
        self.assertEqual(render.call_count, 1)
        self.assertEqual(render.call_args.args[1]["segments"], [])
        output = self.workspace / "02.5_page_conversion_review"
        self.assertEqual(sorted(path.name for path in output.iterdir()),
                         ["page_0064_review.json", "page_0064_review.png"])


if __name__ == "__main__":
    unittest.main()

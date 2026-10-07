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

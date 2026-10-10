"""Run-scoped physical-page selection on retained all-page predecessors."""

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
        for directory in ("02_preprocess", "03_page_conversion"):
            (self.workspace / directory).mkdir()
        for page in (63, 64):
            identity = f"page_{page:04d}"
            value = {"page": page, "segments": [{"segment_id": "s1", "type": "prose",
                      "md_text": "NOTE: Advisory.", "bbox": [0, 0, 10, 10]}]}
            (self.workspace / "03_page_conversion" / f"{identity}_segments.json").write_text(json.dumps(value))
            (self.workspace / "02_preprocess" / f"{identity}.json").write_text(json.dumps({"page": page, "page_id": identity}))
            Image.new("RGB", (10, 10), "white").save(self.workspace / "02_preprocess" / f"{identity}.png")

    def test_page_conversion_requests_only_selected_page(self):
        stage = stage_module("03_page_conversion", "convert_page")
        for path in (self.workspace / "02_preprocess").glob("*.json"):
            value = json.loads(path.read_text())
            value["raster"] = {"pixel_width": 10, "pixel_height": 10}
            value["blocks"] = [{"block_id": 0, "text": "A\n", "bbox": [0, 0, 10, 10],
                                "char_bboxes": [[0, 0, 10, 10], [0, 0, 10, 10]]}]
            path.write_text(json.dumps(value))
        with patch.object(stage, "CodexClient") as client:
            transport = client.return_value.__enter__.return_value
            transport.generate_json.return_value = {"page": 64, "segments": []}
            stage.convert_pages(self.workspace, {"input": {"pages": [63]}}, pages=[64])
        self.assertEqual(transport.generate_json.call_count, 1)
        self.assertNotIn('schema', transport.generate_json.call_args.kwargs)
        self.assertNotIn('char_bboxes', transport.generate_json.call_args.args[0])
        prompt = (PDF / 'stages/03_page_conversion/prompt.md').read_text(encoding='utf-8')
        source = json.loads((self.workspace / '02_preprocess/page_0064.json').read_text())
        for block in source['blocks']:
            block.pop('char_bboxes')
        self.assertEqual(transport.generate_json.call_args.args[0],
                         prompt + '\n\nSource data (complete extracted-text JSON):\n' + json.dumps(source, ensure_ascii=False))
        self.assertIn('char_bboxes', (self.workspace / '02_preprocess/page_0064.json').read_text())
        self.assertEqual(transport.generate_json.call_args.kwargs["image_path"].name, "page_0064.png")
        self.assertEqual([p.name for p in (self.workspace / "03_page_conversion").glob("*_segments.json")
                          if json.loads(p.read_text())["segments"] == []], ["page_0064_segments.json"])

    def test_filter_preserves_editor_roles_on_selected_pages(self):
        stage = stage_module("04_filter_page_content", "filter_page_content")
        selected = self.workspace / "03_page_conversion/page_0064_segments.json"
        value = json.loads(selected.read_text())
        # Older dedicated roles must neither discard earlier prose nor whole pages.
        roles = ["cover", "toc_heading", "toc", "index_heading", "index",
                 "list_of_tables_heading", "list_of_tables",
                 "list_of_figures_heading", "list_of_figures"]
        retained = value["segments"] + [
            {"segment_id": f"s{index}", "type": role, "bbox": [0, 0, 10, 10]}
            for index, role in enumerate(roles, 2)]
        value["segments"] = retained + [
            {"type": "header", "bbox": [0, 0, 10, 10]},
            {"type": "footer", "bbox": [0, 0, 10, 10]}]
        selected.write_text(json.dumps(value))
        # An unselected page remains outside the filter input.
        path = self.workspace / "03_page_conversion/page_0065_segments.json"
        path.write_text(json.dumps({"page": 65, "segments": [{"type": "toc_heading"}]}))
        config_path = self.workspace / "config.yaml"
        config_path.write_text("input:\n  pages: [63]\n")
        with patch.object(sys, "argv", ["filter_page_content.py", "--workspace", str(self.workspace),
                                        "--config", str(config_path), "--page-ranges", "64"]):
            stage.main()
        output = self.workspace / "04_filter_page_content"
        self.assertEqual([p.name for p in output.iterdir()], ["page_0064_segments.json"])
        self.assertEqual(json.loads((output / selected.name).read_text())["segments"], retained)

    def table_pages(self, directory):
        target = self.workspace / directory
        target.mkdir(exist_ok=True)
        for page in (63, 64):
            identity = f"page_{page:04d}_table"
            table = {"segment_id": identity, "type": "table", "md_text": "source",
                     "bbox": [0, 0, 10, 10], "table_format": "unconverted"}
            (target / f"page_{page:04d}_segments.json").write_text(json.dumps({"page": page, "segments": [table]}))

    def test_table_conversion_saves_only_changed_pages(self):
        stage = stage_module("05_transform_page_tables", "transform_page_tables")
        self.table_pages("04_filter_page_content")
        prose = self.workspace / "04_filter_page_content/page_0065_segments.json"
        prose.write_text(json.dumps({"page": 65, "segments": [{"type": "prose", "md_text": "Keep me"}]}))
        (self.workspace / "02_preprocess/page_0065.json").write_text(json.dumps({"page": 65, "page_id": "page_0065"}))
        with patch.object(stage, "CodexClient") as client:
            client.return_value.__enter__.return_value.generate_json.side_effect = [
                {"format": "markdown", "md_text": "| A |\n| --- |\n| B |"},
                {"format": "unconverted", "md_text": ""}]
            stage.transform_tables(self.workspace, {})
        self.assertEqual([p.name for p in (self.workspace / stage.STAGE).glob("*_segments.json")],
                         ["page_0063_segments.json"])


    def test_table_conversion_requests_only_selected_page(self):
        stage = stage_module("05_transform_page_tables", "transform_page_tables")
        self.table_pages("04_filter_page_content")
        with patch.object(stage, "CodexClient") as client:
            transport = client.return_value.__enter__.return_value
            transport.generate_json.return_value = {
                "format": "html", "md_text": '<table><tr><td colspan="2">A</td></tr></table>'}
            stage.transform_tables(self.workspace, {"input": {"pages": [63]}}, pages=[64])
        self.assertEqual(transport.generate_json.call_count, 1)
        output = self.workspace / stage.STAGE
        self.assertEqual([p.name for p in output.glob("*_segments.json")], ["page_0064_segments.json"])
        page = json.loads((output / "page_0064_segments.json").read_text())
        table = page["segments"][0]
        self.assertEqual(table["md_text"], transport.generate_json.return_value["md_text"])
        self.assertNotIn('schema', transport.generate_json.call_args.kwargs)
        self.assertEqual(transport.generate_json.call_args.args[0],
                         (PDF / 'stages/05_transform_page_tables/prompt.md').read_text(encoding='utf-8'))
        self.assertEqual(table["table_format"], "html")
        for key in ("table_rag_text", "table_group_segment_ids", "table_source_asset"):
            self.assertNotIn(key, table)
        self.assertFalse((output / "assets").exists())
        self.assertFalse(list(self.workspace.glob(".05-*")))


if __name__ == "__main__":
    unittest.main()

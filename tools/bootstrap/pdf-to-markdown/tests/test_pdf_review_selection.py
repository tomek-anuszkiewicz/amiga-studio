"""Run-scoped physical-page selection on retained all-page predecessors."""

import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from PIL import Image

ROOT = Path(__file__).resolve().parents[4]
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
                      "md_text": "NOTE: Advisory.", "role": "note", "bbox": [0, 0, 10, 10]}]}
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
            stage.convert_pages(self.workspace, {"llm": {"concurrency": 1}, "input": {"pages": [63]}}, pages=[64])
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
            {"segment_id": f"s{index}", "type": role, "bbox": [0, 0, 10, 10], 'role': None}
            for index, role in enumerate(roles, 2)]
        value["segments"] = retained + [
            {"type": "header", "bbox": [0, 0, 10, 10], 'role': None},
            {"type": "footer", "bbox": [0, 0, 10, 10], 'role': None},
            {"type": "thumb_index", "bbox": [0, 0, 10, 10], 'role': None}]
        selected.write_text(json.dumps(value))
        # An unselected page remains outside the filter input.
        path = self.workspace / "03_page_conversion/page_0065_segments.json"
        path.write_text(json.dumps({"page": 65, "segments": [{"type": "toc_heading", 'role': None}]}))
        config_path = self.workspace / "config.yaml"
        config_path.write_text("input:\n  pages: [63]\n")
        with patch.object(sys, "argv", ["filter_page_content.py", "--workspace", str(self.workspace),
                                        "--config", str(config_path), "--page-ranges", "64"]):
            stage.main()
        output = self.workspace / "04_filter_page_content"
        self.assertEqual([p.name for p in output.iterdir()], ["page_0064_segments.json"])
        self.assertEqual(json.loads((output / selected.name).read_text())["segments"], retained)

    def test_callout_stage_preserves_final_page_data_and_source_regions(self):
        stage = stage_module('07_repair_callouts', 'repair_callouts')
        self.table_pages('04_filter_page_content')
        path = self.workspace / '04_filter_page_content/page_0064_segments.json'
        value = json.loads(path.read_text())
        value['segments'] += [
            {'segment_id': 'label', 'type': 'callout', 'md_text': '**NOTE**', 'bbox': [0, 1, 10, 3], 'role': None},
            {'segment_id': 'body', 'type': 'callout_text', 'md_text': 'First\n\nSecond', 'bbox': [0, 3, 10, 9], 'role': None},
            {'segment_id': 'orphan', 'type': 'callout_text', 'md_text': 'Keep me', 'bbox': [0, 9, 10, 10], 'role': None}]
        (self.workspace / '05_transform_page_tables').mkdir()
        replacement = self.workspace / '05_transform_page_tables' / path.name
        value['segments'][0].update(role='legend', table_format='markdown', md_text='| A |\n|---|\n| B |')
        replacement.write_text(json.dumps(value))
        stage.repair_callouts(self.workspace, pages=[64])
        output = self.workspace / '07_repair_callouts'
        self.assertEqual([p.name for p in output.iterdir()], [path.name])
        final = json.loads((output / path.name).read_text())
        self.assertEqual(final['segments'][0], value['segments'][0])
        callout = final['segments'][1]
        self.assertEqual(callout['segment_id'], 'label')
        self.assertEqual(callout['md_text'], '> [!NOTE] **NOTE**\n>\n> First\n> \n> Second')
        self.assertEqual(callout['bbox'], [0, 1, 10, 9])
        self.assertEqual(callout['pdf_regions'], [
            {'page': 64, 'bbox': [0, 1, 10, 3]}, {'page': 64, 'bbox': [0, 3, 10, 9]}])
        self.assertEqual(final['segments'][2], {**value['segments'][3], 'role': None})
        before = json.dumps(final)
        fixed, warnings = stage.repair_page(final)
        self.assertEqual(fixed, 0)
        self.assertEqual(len(warnings), 1)
        self.assertEqual(json.dumps(final), before)

    def table_pages(self, directory):
        target = self.workspace / directory
        target.mkdir(exist_ok=True)
        for page in (63, 64):
            identity = f"page_{page:04d}_table"
            table = {"segment_id": identity, "type": "table", "md_text": "source",
                     "bbox": [0, 0, 10, 10], "table_format": "unconverted", "role": "legend"}
            (target / f"page_{page:04d}_segments.json").write_text(json.dumps({"page": page, "segments": [table]}))

    def test_table_conversion_saves_only_changed_pages(self):
        stage = stage_module("05_transform_page_tables", "transform_page_tables")
        self.table_pages("04_filter_page_content")
        prose = self.workspace / "04_filter_page_content/page_0065_segments.json"
        prose.write_text(json.dumps({"page": 65, "segments": [{"type": "prose", "md_text": "Keep me", 'role': None}]}))
        (self.workspace / "02_preprocess/page_0065.json").write_text(json.dumps({"page": 65, "page_id": "page_0065"}))
        with patch.object(stage, "CodexClient") as client:
            client.return_value.__enter__.return_value.generate_json.side_effect = [
                {"format": "markdown", "md_text": "| A |\n| --- |\n| B |"},
                {"format": "unconverted", "md_text": ""}]
            stage.transform_tables(self.workspace, {"llm": {"concurrency": 1}})
        self.assertEqual([p.name for p in (self.workspace / stage.STAGE).glob("*_segments.json")],
                         ["page_0063_segments.json"])
        converted = json.loads((self.workspace / stage.STAGE / 'page_0063_segments.json').read_text())
        self.assertEqual(converted['segments'][0]['role'], 'legend')

    def test_callout_repair_does_not_discard_a_different_body_role(self):
        stage = stage_module('07_repair_callouts', 'repair_callouts')
        label = {'segment_id': 'label', 'type': 'callout', 'role': None,
                 'md_text': 'NOTE', 'bbox': [0, 0, 10, 2]}
        body = {'segment_id': 'body', 'type': 'callout_text', 'role': 'note',
                'md_text': 'Keep both roles', 'bbox': [0, 2, 10, 5]}
        page = {'page': 1, 'segments': [label, body]}
        fixed, warnings = stage.repair_page(page)
        self.assertEqual(fixed, 0)
        self.assertEqual(page['segments'], [label, body])
        self.assertIn('role differs', warnings[0])


    def test_table_conversion_requests_only_selected_page(self):
        stage = stage_module("05_transform_page_tables", "transform_page_tables")
        self.table_pages("04_filter_page_content")
        with patch.object(stage, "CodexClient") as client:
            transport = client.return_value.__enter__.return_value
            transport.generate_json.return_value = {
                "format": "html", "md_text": '<table><tr><td colspan="2">A</td></tr></table>'}
            stage.transform_tables(self.workspace, {"llm": {"concurrency": 1}, "input": {"pages": [63]}}, pages=[64])
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

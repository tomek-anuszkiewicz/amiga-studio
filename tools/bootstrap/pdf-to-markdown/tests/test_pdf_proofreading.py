"""Stage 08 page text handoff and selected-page isolation."""

import importlib.util
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from unittest.mock import patch
from PIL import Image

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "tools/bootstrap/pdf-to-markdown"))
from common.pdf_artifacts import read_json, write_json

STAGE_PATH = ROOT / "tools/bootstrap/pdf-to-markdown/stages/08_proofreading/proofread.py"
spec = importlib.util.spec_from_file_location("pdf_proofread", STAGE_PATH)
stage = importlib.util.module_from_spec(spec)
spec.loader.exec_module(stage)


class ProofreadingTests(unittest.TestCase):
    def test_selected_page_changes_only_text_and_keeps_neighbors_read_only(self):
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            (workspace / "02_preprocess").mkdir()
            (workspace / "07_repair_callouts").mkdir()
            pages = [
                (1, "Earlier context", "page_0001_seg_001"),
                (2, "HARDW ARE and AUDOVOL", "page_0002_seg_001"),
                (3, "Later context", "page_0003_seg_001"),
            ]
            for number, content, identifier in pages:
                page_id = f"page_{number:04d}"
                write_json(workspace / "02_preprocess" / f"{page_id}.json",
                           {"page": number, "page_id": page_id})
                write_json(workspace / "07_repair_callouts" / f"{page_id}_segments.json",
                           {"page": number, "source": {"unchanged": True},
                            "segments": [{"segment_id": identifier, "type": "prose", "role": None,
                                          "bbox": [1, 2, 3, 4], "md_text": content}]})
            Image.new("RGB", (8, 8), "white").save(workspace / "02_preprocess/page_0002.png")
            seen = []

            class Client:
                def __init__(self, config, **kwargs):
                    self.kwargs = kwargs
                    self.call_count = 0
                    self.cached_call_count = 0

                def __enter__(self):
                    return self

                def __exit__(self, *args):
                    pass

                def generate_json(self, prompt, *, image_path=None):
                    self_test.assertEqual(self.kwargs["operation"], "pdf_page_proofreading")
                    if image_path is not None:
                        self_test.assertEqual(image_path.name, "page_0002.png")
                        self_test.assertIn("AUDOVOL needs visual source evidence", prompt)
                        self.call_count += 1
                        return {"changes": [], "unchanged": [
                            {"segment_id": "page_0002_seg_001",
                             "reason": "The source visibly prints AUDOVOL."}]}
                    payload = json.loads(prompt.split("PAGE INPUT JSON:\n", 1)[1])
                    seen.append(payload)
                    self.cached_call_count += 1
                    return {"changes": [{"segment_id": "page_0002_seg_001",
                                         "md_text": "HARDWARE and AUDOVOL"}],
                            "unresolved": [{"segment_id": "page_0002_seg_001",
                                            "reason": "AUDOVOL needs visual source evidence"}]}

            self_test = self
            output = io.StringIO()
            with redirect_stdout(output):
                stage.proofread(workspace, {"llm": {"concurrency": 1}}, pages=[2], client_factory=Client)
            self.assertIn("[proofreading] page_0002: requesting page correction", output.getvalue())
            self.assertIn("[proofreading] page_0002: response received; calls=0, cache_hits=1", output.getvalue())
            self.assertIn("[proofreading] page_0002: requesting page correction with image; image=page_0002.png",
                          output.getvalue())
            self.assertIn("[proofreading] page_0002: image response received; calls=1, cache_hits=0", output.getvalue())
            self.assertIn("[proofreading] page_0002: done", output.getvalue())
            self.assertNotIn("unresolved=", output.getvalue())
            self.assertNotIn("unchanged_after_image", output.getvalue())
            self.assertNotIn("checking cache or Codex", output.getvalue())
            self.assertEqual(len(seen), 1)
            self.assertEqual(seen[0]["previous_tail"]["segments"][0]["md_text"], "Earlier context")
            self.assertEqual(seen[0]["following_head"]["segments"][0]["md_text"], "Later context")
            self.assertNotIn("bbox", seen[0]["target_page"]["segments"][0])
            self.assertNotIn("blocks", seen[0]["target_page"])
            output = workspace / "08_proofreading"
            self.assertEqual({path.name for path in output.iterdir()},
                             {"page_0002_segments.json"})
            original = read_json(workspace / "07_repair_callouts/page_0002_segments.json")
            corrected = read_json(output / "page_0002_segments.json")
            self.assertEqual(corrected["source"], original["source"])
            self.assertEqual(corrected["segments"][0]["bbox"], original["segments"][0]["bbox"])
            self.assertEqual(corrected["segments"][0]["md_text"], "HARDWARE and AUDOVOL")

    def test_invalid_neighbor_or_unknown_target_cannot_change_page(self):
        page = {"segments": [{"segment_id": "target", "md_text": "Original"}]}
        with self.assertRaisesRegex(ValueError, "unknown or duplicate"):
            stage.apply_changes(page, {"changes": [{"segment_id": "neighbor", "md_text": "Changed"}],
                                       "unresolved": []})
        self.assertEqual(page["segments"][0]["md_text"], "Original")

    def test_empty_change_list_keeps_target_text_and_unresolved_report(self):
        page = {"segments": [{"segment_id": "target", "md_text": "AUDOVOL", "bbox": [1, 2, 3, 4]}]}
        count, unresolved = stage.apply_changes(page, {"changes": [], "unresolved": [
            {"segment_id": "target", "reason": "Needs visual source evidence"}]})
        self.assertEqual(count, 0)
        self.assertEqual(len(unresolved), 1)
        self.assertEqual(page["segments"][0],
                         {"segment_id": "target", "md_text": "AUDOVOL", "bbox": [1, 2, 3, 4]})

    def test_image_response_requires_every_flagged_id_without_mutating_page(self):
        page = {"segments": [{"segment_id": "target", "md_text": "Original"},
                             {"segment_id": "other", "md_text": "Uncertain"}]}
        with self.assertRaisesRegex(ValueError, "every initial unresolved segment"):
            stage.apply_image_response(
                page, {"changes": [{"segment_id": "target", "md_text": "Changed"}],
                       "unchanged": []}, {"target", "other"})
        self.assertEqual(page["segments"][0]["md_text"], "Original")

    def test_no_unresolved_item_skips_image_request(self):
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            (workspace / "02_preprocess").mkdir()
            (workspace / "07_repair_callouts").mkdir()
            write_json(workspace / "02_preprocess/page_0001.json", {"page": 1, "page_id": "page_0001"})
            write_json(workspace / "07_repair_callouts/page_0001_segments.json",
                       {"page": 1, "segments": [{"segment_id": "page_0001_seg_001",
                                                "type": "prose", "role": None,
                                                "md_text": "Original"}]})

            class Client:
                def __init__(self, *args, **kwargs):
                    self.call_count = 0
                    self.cached_call_count = 0

                def __enter__(self):
                    return self

                def __exit__(self, *args):
                    pass

                def generate_json(self, prompt, *, image_path=None):
                    if image_path is not None:
                        raise AssertionError("Unexpected image request")
                    self.call_count += 1
                    return {"changes": [], "unresolved": []}

            with redirect_stdout(io.StringIO()):
                stage.proofread(workspace, {"llm": {"concurrency": 1}}, client_factory=Client)
            output = workspace / "08_proofreading"
            self.assertTrue((output / "page_0001_segments.json").is_file())
            self.assertFalse((output / "page_0001_proofreading.json").exists())
            self.assertFalse((output / "proofreading_summary.md").exists())

    def test_unchanged_page_is_written(self):
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            (workspace / "02_preprocess").mkdir()
            (workspace / "07_repair_callouts").mkdir()
            write_json(workspace / "02_preprocess/page_0001.json", {"page": 1, "page_id": "page_0001"})
            page = {"page": 1, "segments": [{"segment_id": "page_0001_seg_001", "type": "prose",
                                            "role": None, "md_text": "Uncertain technical value"}]}
            write_json(workspace / "07_repair_callouts/page_0001_segments.json", page)
            Image.new("RGB", (8, 8), "white").save(workspace / "02_preprocess/page_0001.png")

            class Client:
                def __init__(self, *args, **kwargs):
                    self.call_count = 0
                    self.cached_call_count = 0

                def __enter__(self):
                    return self

                def __exit__(self, *args):
                    pass

                def generate_json(self, prompt, *, image_path=None):
                    self.call_count += 1
                    if image_path is not None:
                        return {"changes": [], "unchanged": [
                            {"segment_id": "page_0001_seg_001", "reason": "Still uncertain − after image"}]}
                    return {"changes": [], "unresolved": [{"segment_id": "page_0001_seg_001",
                                                           "reason": "Uncertain − value"}]}

            output = io.TextIOWrapper(io.BytesIO(), encoding="cp1252")
            with patch.object(stage.sys, "stdout", output):
                stage.proofread(workspace, {"llm": {"concurrency": 1}}, client_factory=Client)
            output.flush()
            self.assertIn("[proofreading] page_0001: requesting page correction", output.buffer.getvalue().decode("cp1252"))
            self.assertIn("[proofreading] page_0001: response received; calls=1, cache_hits=0", output.buffer.getvalue().decode("cp1252"))
            self.assertIn("[proofreading] page_0001: image response received; calls=1, cache_hits=0", output.buffer.getvalue().decode("cp1252"))
            self.assertIn("[proofreading] page_0001: done", output.buffer.getvalue().decode("cp1252"))
            output.close()
            self.assertEqual(read_json(workspace / "08_proofreading/page_0001_segments.json"), page)
            self.assertFalse((workspace / "08_proofreading/proofreading_summary.md").exists())


if __name__ == "__main__":
    unittest.main()

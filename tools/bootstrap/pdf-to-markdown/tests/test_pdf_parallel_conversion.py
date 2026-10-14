"""Parallel table completion must not race full-page artifact writes."""

import importlib.util
from pathlib import Path
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

from PIL import Image

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "tools/bootstrap"))
sys.path.insert(0, str(ROOT / "tools/bootstrap/pdf-to-markdown"))
from common.pdf_artifacts import read_json, write_json
from conversion.parallel import convert_parallel


class ParallelTableTests(unittest.TestCase):
    def test_same_page_tables_are_saved_together_in_source_order(self):
        self.run_tables(fail=False)

    def test_failed_table_does_not_publish_a_partial_page(self):
        self.run_tables(fail=True)

    def run_tables(self, *, fail):
        spec = importlib.util.spec_from_file_location(
            "parallel_tables", ROOT / "tools/bootstrap/pdf-to-markdown/stages/05_transform_page_tables/transform_page_tables.py")
        stage = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(stage)
        barrier = threading.Barrier(2)
        second_finished = threading.Event()

        class Client:
            def __enter__(self):
                return self

            def __exit__(self, *_):
                pass

            def generate_json(self, prompt, *, image_path):
                barrier.wait(timeout=3)
                if image_path.stem.endswith("001"):
                    if not second_finished.wait(3):
                        raise TimeoutError("Second table did not complete")
                else:
                    second_finished.set()
                    if fail:
                        raise RuntimeError("rate limit")
                return {"format": "markdown", "md_text": image_path.stem}

        def parallel(config, **options):
            options["client_factory"] = lambda *_, **__: Client()
            return convert_parallel(config, **options)

        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            (workspace / "02_preprocess").mkdir()
            (workspace / "04_filter_page_content").mkdir()
            write_json(workspace / "02_preprocess/page_0001.json", {"page_id": "page_0001"})
            with Image.new("RGB", (20, 20), "white") as image:
                image.save(workspace / "02_preprocess/page_0001.png")
            segments = [{"segment_id": f"page_0001_seg_{index:03d}", "type": "table",
                         "role": "legend", "bbox": [0, 0, 10, 10], "md_text": "source"}
                        for index in (1, 2)]
            write_json(workspace / "04_filter_page_content/page_0001_segments.json",
                       {"segments": segments})
            with patch.object(stage, "convert_parallel", side_effect=parallel), \
                    patch.object(stage, "write_json", wraps=write_json) as save:
                if fail:
                    with self.assertRaisesRegex(RuntimeError, "rate limit"):
                        stage.transform_tables(workspace, {"llm": {"concurrency": 2}})
                else:
                    stage.transform_tables(workspace, {"llm": {"concurrency": 2}})
                self.assertEqual(save.call_count, 0 if fail else 1)
            output = workspace / "05_transform_page_tables/page_0001_segments.json"
            if fail:
                self.assertFalse(output.exists())
            else:
                page = read_json(output)
                self.assertEqual([s["md_text"] for s in page["segments"]],
                                 ["page_0001_seg_001", "page_0001_seg_002"])
                self.assertTrue(all(s["role"] == "legend" for s in page["segments"]))
            self.assertFalse(list(workspace.glob(".05-*")))


if __name__ == "__main__":
    unittest.main()

"""Regression for distinct geometry when an advisory splits a text object."""

from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools/bootstrap/pdf-to-markdown"))
from common.pdf_callouts import apply_replacements, request_objects


class CalloutGeometryTests(unittest.TestCase):
    def test_split_preserves_each_model_box_and_source_page(self):
        original = {"segment_id": "s1", "type": "prose",
                    "md_text": "NOTE\nAdvisory body.\nOrdinary remainder.",
                    "bbox": [20, 30, 400, 180]}
        page = {"segments": [original]}
        replacements = [
            {"type": "callout", "md_text": "NOTE", "bbox": [160, 30, 220, 50]},
            {"type": "callout_text", "md_text": "Advisory body.", "bbox": [20, 60, 400, 100]},
            {"type": "prose", "md_text": "Ordinary remainder.", "bbox": [20, 120, 400, 180]},
        ]
        response = {"replacements": [{"source_segment_ids": ["s1"],
                    "replacement_segments": [dict(item, source_segment_ids=["s1"])
                                             for item in replacements]}]}

        corrected, reports = apply_replacements(page, request_objects(page, ["NOTE"]), response)

        self.assertEqual(reports[0]["status"], "applied")
        self.assertEqual([segment["bbox"] for segment in corrected["segments"]],
                         [item["bbox"] for item in replacements])
        self.assertEqual(page["segments"], [original])
        self.assertEqual(original["bbox"], [20, 30, 400, 180])
        self.assertTrue(all(segment["source_segment_ids"] == ["s1"]
                            for segment in corrected["segments"]))


if __name__ == "__main__":
    unittest.main()

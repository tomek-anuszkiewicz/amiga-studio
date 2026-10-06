"""Linked assembly attachments must reach the HTML transcription input."""

import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


PIPELINE_PATH = Path(__file__).resolve().parents[1] / "tools/bootstrap/html-to-markdown/pipeline.py"
spec = importlib.util.spec_from_file_location("html_assembly_pipeline", PIPELINE_PATH)
pipeline = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pipeline)


class AssemblySourceTests(unittest.TestCase):
    def test_missing_linked_source_stops_conversion(self):
        with tempfile.TemporaryDirectory() as temporary:
            page = Path(temporary) / "clock.html"
            with self.assertRaises(FileNotFoundError):
                pipeline.embed_assembly_sources('<p><a href="missing.s">code</a></p>', page)

    def test_linked_source_reaches_transcription_in_single_and_crawl_conversion(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            page = root / "clock.html"
            page.write_text('<h3>Clock</h3><p>Read <a href="readclock.s">code snippet</a>.</p>', encoding="utf-8")
            source = '; Clock <listing> & registers\nReadClock:\n\tlea\t$dc0030,a0\n\trts\n'
            (root / "readclock.s").write_text(source, encoding="utf-8")
            output = root / "output"
            output.mkdir()

            def transcribe(html, title, prompt_file, client):
                soup = pipeline.BeautifulSoup(html, "html.parser")
                listing = soup.find("code", class_="language-m68k")
                self.assertIsNotNone(listing, "Linked .s content was omitted from transcription input")
                self.assertEqual(listing.get_text(), source)
                self.assertFalse(soup.find("a", href="readclock.s"))
                return '---\ntitle: Clock\n---\n# Clock\n'

            with patch.object(pipeline, "extract_assets"), patch.object(pipeline, "convert_with_llm", side_effect=transcribe):
                for convert, input_path in ((pipeline.convert_single_html, page), (pipeline.convert_crawl_directory, root)):
                    with self.subTest(convert=convert.__name__):
                        convert(input_path, output, "Clock", None)


if __name__ == "__main__":
    unittest.main()

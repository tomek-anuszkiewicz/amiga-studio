"""Check that adjacent image regions produce one source-faithful crop."""

from pathlib import Path
import sys
import tempfile
import unittest

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.model import load_input, write_json
from review.service import ReviewState


class GraphicMergeTests(unittest.TestCase):
    def test_adjacent_graphics_form_one_page_region(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'pdf/pages/page_0001.png'
            source.parent.mkdir(parents=True)
            Image.new('RGB', (100, 120), 'white').save(source)
            assets = root / 'working/assets'
            assets.mkdir(parents=True)
            for name in ('first.png', 'second.png'):
                Image.new('RGB', (20, 20), 'white').save(assets / name)
            book = {'toc': {'depth': 2}, 'chapters': [{
                'id': 'chapter', 'title': 'Example', 'markdown_file': 'example.md',
                'elements': [
                    {'id': 'first', 'type': 'graphic', 'role': None, 'visible': True,
                     'pdf_regions': [{'page': 1, 'bbox': [10, 20, 50, 60]}],
                     'asset': 'assets/first.png', 'md_text': 'Stored source text'},
                    {'id': 'second', 'type': 'graphic', 'role': 'legend', 'visible': True,
                     'pdf_regions': [{'page': 1, 'bbox': [8, 70, 35, 90]}],
                     'asset': 'assets/second.png'}]}]}
            write_json(root / 'working/book.json', book)
            state = ReviewState(root)
            revision = load_input(root / 'working')[2]
            state.merge_graphics({'id': 'first', 'next_id': 'second',
                                  'expected_revision': revision})
            merged = load_input(root / 'working')[0]['chapters'][0]['elements']
            self.assertEqual(len(merged), 1)
            self.assertEqual(merged[0]['id'], 'first')
            self.assertEqual(merged[0]['md_text'], 'Stored source text')
            self.assertEqual(merged[0]['pdf_regions'],
                             [{'page': 1, 'bbox': [10, 20, 50, 60]},
                              {'page': 1, 'bbox': [8, 70, 35, 90]}])
            with Image.open(root / 'working' / merged[0]['asset']) as image:
                self.assertEqual(image.size, (40, 60))
            with self.assertRaisesRegex(ValueError, 'Stale edit command'):
                state.merge_graphics({'id': 'first', 'next_id': 'second',
                                      'expected_revision': revision})


if __name__ == '__main__':
    unittest.main()

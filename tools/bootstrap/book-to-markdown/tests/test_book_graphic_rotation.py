"""Verify image rotation changes the Working asset and preserves its source."""

from pathlib import Path
import sys
import tempfile
import unittest

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.model import load_input, write_json
from review.service import ReviewState


class GraphicRotationTests(unittest.TestCase):
    def test_rotations_preserve_pixels_and_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            asset = root / 'working/assets/source.png'
            asset.parent.mkdir(parents=True)
            image = Image.new('RGB', (2, 3))
            image.putdata([(255, 0, 0), (0, 255, 0), (0, 0, 255),
                           (255, 255, 0), (0, 0, 0), (255, 255, 255)])
            image.save(asset)
            book = {'toc': {'depth': 2}, 'chapters': [{
                'id': 'chapter', 'title': 'Example', 'markdown_file': 'example.md',
                'elements': [{'id': 'image', 'type': 'graphic', 'role': None,
                              'visible': True, 'pdf_regions': [],
                              'asset': 'assets/source.png'}]}]}
            write_json(root / 'working/book.json', book)
            state = ReviewState(root)
            original = image.tobytes()
            for direction, transpose in [('clockwise', Image.Transpose.ROTATE_270),
                                         ('counterclockwise', Image.Transpose.ROTATE_90),
                                         ('half_turn', Image.Transpose.ROTATE_180)]:
                revision = load_input(root / 'working')[2]
                state.rotate_graphic({'id': 'image', 'direction': direction,
                                      'expected_revision': revision})
                rotated = load_input(root / 'working')[0]['chapters'][0]['elements'][0]
                with Image.open(root / 'working' / rotated['asset']) as actual:
                    with image.transpose(transpose) as expected:
                        self.assertEqual(actual.size, expected.size)
                        self.assertEqual(actual.tobytes(), expected.tobytes())
                image = image.transpose(transpose)
                with self.assertRaisesRegex(ValueError, 'Stale edit command'):
                    state.rotate_graphic({'id': 'image', 'direction': direction,
                                          'expected_revision': revision})
            with Image.open(asset) as source:
                self.assertEqual(source.tobytes(), original)


if __name__ == '__main__':
    unittest.main()

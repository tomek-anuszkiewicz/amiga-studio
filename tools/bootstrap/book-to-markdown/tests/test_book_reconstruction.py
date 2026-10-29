"""Rebuild review inputs from an accepted PDF-backed recipe."""

import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.reconstruct import import_accepted_book
from common.pdf_config import preparation_settings


class BookReconstructionTests(unittest.TestCase):
    def test_reconstructs_authored_recipe_pages_and_graphics(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source.pdf'
            source.write_bytes(b'matching source')
            (root / 'source.json').write_text(json.dumps({
                'sha256': hashlib.sha256(source.read_bytes()).hexdigest()}), encoding='utf-8')
            recipe = {'toc': {'depth': 2}, 'chapters': [{'id': 'chapter-001',
                'title': 'Chapter', 'markdown_file': 'chapter.md', 'elements': [
                    {'id': 'text', 'type': 'prose', 'role': None, 'visible': True,
                     'pdf_regions': [{'page': 1, 'bbox': [0, 0, 4, 4]}],
                     'md_text': '{{text page=1 block=0 chars=0:5}}'},
                    {'id': 'image', 'type': 'graphic', 'role': None, 'visible': True,
                     'pdf_regions': [{'page': 1, 'bbox': [1, 1, 3, 3]}],
                     'asset': 'assets/image.png'}]}]}
            original = json.dumps(recipe).encode()
            (root / 'book.json').write_bytes(original)
            old_ocr = {'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
                       'prepared_sha256': 'old', 'profile': preparation_settings({}), 'ocr_pages': []}
            (root / 'source-ocr.json').write_text(json.dumps(old_ocr), encoding='utf-8')

            def prepare(command, check):
                self.assertTrue(check)
                self.assertEqual(command[-4:], ['--from-stage', '01', '--to-stage', '02'])
                output = root / 'pdf-to-markdown/02_preprocess'
                output.mkdir(parents=True)
                (output.parent / 'config.yaml').write_text('rendering:\n  dpi: 300\n', encoding='utf-8')
                (output / 'page_0001.json').write_text(json.dumps({
                    'blocks': [{'block_id': 0, 'text': 'Hello'}]}), encoding='utf-8')
                Image.new('RGB', (4, 4), 'red').save(output / 'page_0001.png')
                (root / 'source-ocr.pdf').write_bytes(b'new prepared PDF bytes')
                current_ocr = {**old_ocr, 'prepared_sha256': hashlib.sha256(
                    (root / 'source-ocr.pdf').read_bytes()).hexdigest()}
                (root / 'source-ocr.json').write_text(json.dumps(current_ocr), encoding='utf-8')

            with patch('review.reconstruct.subprocess.run', side_effect=prepare):
                self.assertEqual(import_accepted_book(root), 2)
            self.assertEqual((root / 'working/book.json').read_bytes(), original)
            self.assertEqual((root / 'book.json').read_bytes(), original)
            self.assertEqual(json.loads((root / 'source-ocr.json').read_text(encoding='utf-8')), old_ocr)
            self.assertFalse((root / 'accepted').exists())
            self.assertIn('Hello', (root / 'working/chapter.md').read_text(encoding='utf-8'))
            with Image.open(root / 'working/assets/image.png') as image:
                self.assertEqual(image.size, (2, 2))
            with self.assertRaisesRegex(ValueError, 'Review output already exists'):
                import_accepted_book(root)


if __name__ == '__main__':
    unittest.main()

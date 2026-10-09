"""Discard restores a coherent bundle and protects Working on replacement failure."""
from copy import deepcopy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

REVIEW = Path(__file__).resolve().parents[1] / 'tools/bootstrap/book-to-markdown'
sys.path.insert(0, str(REVIEW))
from review.model import export_markdown, load_input, write_json
from review.service import ReviewState


class DiscardBundleTest(unittest.TestCase):
    def prepare(self, root):
        book = {'toc': {'depth': 2}, 'chapters': [{'id': 'chapter', 'title': 'Example',
            'markdown_file': 'example.md', 'elements': [{'id': 'image', 'type': 'graphic',
            'asset': 'assets/image.png', 'pdf_regions': []}]}]}
        for version, content in [('accepted', b'accepted asset'), ('working', b'edited asset')]:
            directory = root / version
            (directory / 'assets').mkdir(parents=True)
            (directory / 'assets/image.png').write_bytes(content)
            write_json(directory / 'book.json', book)
            fingerprint = load_input(directory)[2]
            renders = directory / 'renders' / version
            (renders / 'assets').mkdir(parents=True)
            (renders / 'assets/image.png').write_bytes(content)
            (renders / 'chapter.html').write_text(version, encoding='utf-8')
            write_json(directory / 'rendered.json', {**deepcopy(book), 'revision': version,
                'input_fingerprint': fingerprint, 'settings': {'presentation': 'html'}})
            export_markdown(book, directory)
        (root / 'working/obsolete.md').write_text('Removed chapter', encoding='utf-8')
        state = ReviewState(root)
        state.pending = False
        command = {'expected_revision': load_input(root / 'working')[2], 'accepted_revision': 'accepted'}
        return state, command

    def test_discard_restores_assets_exports_and_keeps_old_render_urls(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state, command = self.prepare(root)
            before = {p.relative_to(root / 'accepted'): p.read_bytes()
                      for p in (root / 'accepted').rglob('*') if p.is_file()}
            self.assertTrue(state.status()['can_discard'])
            state.discard(command)
            self.assertEqual(load_input(root / 'working'), load_input(root / 'accepted'))
            self.assertFalse((root / 'working/obsolete.md').exists())
            self.assertEqual((root / 'working/renders/working/assets/image.png').read_bytes(), b'edited asset')
            self.assertEqual((root / 'working/assets/image.png').read_bytes(), b'accepted asset')
            for relative, data in before.items():
                self.assertEqual((root / 'accepted' / relative).read_bytes(), data)
                restored = (root / 'working' / relative).read_bytes()
                if relative.name == 'book.json':
                    self.assertEqual(json.loads(restored), json.loads(data))
                else:
                    self.assertEqual(restored, data)
            self.assertFalse(state.status()['can_discard'])
            self.assertEqual(state.status()['changes'], [])

    def test_failed_final_rename_restores_working(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state, command = self.prepare(root)
            before = {p.relative_to(root / 'working'): p.read_bytes()
                      for p in (root / 'working').rglob('*') if p.is_file()}
            rename = Path.rename

            def fail_publication(path, target):
                if path.name.startswith('.working-prepare-'):
                    raise OSError('Simulated replacement failure')
                return rename(path, target)

            with patch.object(Path, 'rename', fail_publication):
                with self.assertRaisesRegex(OSError, 'Simulated replacement failure'):
                    state.discard(command)
            after = {p.relative_to(root / 'working'): p.read_bytes()
                     for p in (root / 'working').rglob('*') if p.is_file()}
            self.assertEqual(after, before)
            self.assertTrue(state.status()['can_discard'])
            self.assertFalse((root / '.working-backup').exists())
            self.assertEqual(list(root.glob('.working-prepare-*')), [])

    def test_discard_element_restores_properties_and_preserves_other_edits(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state, _ = self.prepare(root)
            book = load_input(root / 'working')[0]
            original = deepcopy(book['chapters'][0]['elements'][0])
            book['chapters'][0]['elements'][0].update(type='prose', visible=False, md_text='Edited text')
            other = {'id': 'other', 'type': 'prose', 'md_text': 'Keep this edit', 'pdf_regions': []}
            book['chapters'][0]['elements'].append(other)
            book['toc']['depth'] = 3
            write_json(root / 'working/book.json', book)
            accepted_before = (root / 'accepted/book.json').read_bytes()
            state.discard_element({'id': 'image', 'expected_revision': load_input(root / 'working')[2],
                                   'accepted_revision': 'accepted'})
            restored = json.loads((root / 'working/book.json').read_text(encoding='utf-8'))
            self.assertEqual(restored['chapters'][0]['elements'], [original, other])
            self.assertEqual(restored['toc']['depth'], 3)
            self.assertEqual((root / 'accepted/book.json').read_bytes(), accepted_before)
            self.assertTrue(state.pending)

    def test_discard_element_rejects_stale_versions_without_writing(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state, command = self.prepare(root)
            before = (root / 'working/book.json').read_bytes()
            for revisions in ({**command, 'expected_revision': 'stale'},
                              {**command, 'accepted_revision': 'stale'}):
                with self.assertRaises(ValueError):
                    state.discard_element({'id': 'image', **revisions})
                self.assertEqual((root / 'working/book.json').read_bytes(), before)

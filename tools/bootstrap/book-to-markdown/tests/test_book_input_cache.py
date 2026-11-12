"""Reuse unchanged text while detecting input changes during bundle replacement."""

from copy import deepcopy
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

REVIEW = Path(__file__).resolve().parents[4] / 'tools/bootstrap/book-to-markdown'
sys.path.insert(0, str(REVIEW))
from review import model
from review.model import InputCache, load_input, read_json, write_json
from review.service import ReviewState


class BookInputCacheTest(unittest.TestCase):
    def prepare(self, root):
        for page, text in [(1, 'First'), (2, 'Other')]:
            write_json(root / 'pdf-to-markdown/02_preprocess' / f'page_{page:04d}.json',
                       {'blocks': [{'block_id': 0, 'text': text}]})
        book = {'toc': {'depth': 2}, 'chapters': [
            {'id': 'chapter', 'title': 'Chapter', 'markdown_file': 'chapter.md', 'elements': [
                {'id': f'p{page}', 'type': 'prose', 'role': None, 'pdf_regions': [],
                 'md_text': '{{text page=' + str(page) + ' block=0 chars=0:5}}'} for page in (1, 2)]}]}
        book['chapters'][0]['elements'].append(
            {'id': 'image', 'type': 'graphic', 'role': None, 'pdf_regions': [], 'asset': 'assets/image.png'})
        for version in ('accepted', 'working'):
            directory = root / version
            (directory / 'assets').mkdir(parents=True)
            (directory / 'assets/image.png').write_bytes(version.encode())
            write_json(directory / 'book.json', book)
            data, assets, fingerprint = load_input(directory)
            render = directory / 'renders' / version
            (render / 'assets').mkdir(parents=True)
            (render / 'assets/image.png').write_bytes(assets['assets/image.png'])
            (render / 'chapter.html').write_text(version, encoding='utf-8')
            write_json(directory / 'rendered.json', {
                **data, 'input_fingerprint': fingerprint, 'revision': version,
                'settings': {'presentation': 'html'}})
        write_json(root / 'book.json', book)
        state = ReviewState(root)
        state.pending = False
        return state

    def test_only_changed_block_or_source_page_is_resolved_again(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state = self.prepare(root)
            cache = state.input_cache
            cache.load(root / 'working')
            book = read_json(root / 'working/book.json')
            changed = '**' + book['chapters'][0]['elements'][0]['md_text'] + '**'
            book['chapters'][0]['elements'][0]['md_text'] = changed
            write_json(root / 'working/book.json', book)
            with patch.object(model, 'resolve_template', wraps=model.resolve_template) as resolve:
                result = cache.load(root / 'working')
            self.assertEqual([call.args[0] for call in resolve.call_args_list], [changed])
            self.assertEqual(result[0]['chapters'][0]['elements'][0]['resolved_md_text'], '**First**')

            write_json(root / 'pdf-to-markdown/02_preprocess/page_0002.json',
                       {'blocks': [{'block_id': 0, 'text': 'After'}]})
            with patch.object(model, 'resolve_template', wraps=model.resolve_template) as resolve:
                result = cache.load(root / 'working')
            self.assertEqual([call.args[0] for call in resolve.call_args_list],
                             [book['chapters'][0]['elements'][1]['md_text']])
            self.assertEqual(result[0]['chapters'][0]['elements'][1]['resolved_md_text'], 'After')
            self.assertEqual(result, load_input(root / 'working'))

    def test_discard_reuses_text_and_still_detects_changed_restored_assets(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state = self.prepare(root)
            status = state.status()
            with patch.object(model, 'resolve_template', wraps=model.resolve_template) as resolve:
                state.discard({'expected_revision': status['authored_revision'],
                               'accepted_revision': state.accepted['revision']})
                restored = state.input_cache.load(root / 'working')
            self.assertEqual(resolve.call_count, 0)
            self.assertEqual(restored, load_input(root / 'accepted'))
            self.assertEqual(restored[1]['assets/image.png'], b'accepted')
            self.assertFalse(state.status()['can_discard'])
            (root / 'working/assets/image.png').write_bytes(b'new external edit')
            self.assertNotEqual(state.input_cache.load(root / 'working')[2], restored[2])
            self.assertFalse(state.status()['can_accept'])

    def test_obsolete_and_invalid_edits_do_not_accumulate_cached_text(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state = self.prepare(root)
            cache = state.input_cache
            cache.load(root / 'working')
            original_size = len(cache.text)
            book = read_json(root / 'working/book.json')
            for index in range(10):
                book['chapters'][0]['elements'][0]['md_text'] = f'Edit {index}'
                write_json(root / 'working/book.json', book)
                cache.load(root / 'working')
                self.assertEqual(len(cache.text), original_size)
                invalid = deepcopy(book)
                invalid['chapters'][0]['elements'][0].update(md_text=f'Invalid edit {index}', role='invalid')
                with self.assertRaises(ValueError):
                    cache.load(root / 'working', invalid)
                self.assertEqual(len(cache.text), original_size)

    def test_discard_uses_cached_asset_bytes_after_publishing_the_bundle(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state = self.prepare(root)
            status = state.status()
            state.input_cache.load(root / 'accepted')
            read_bytes = Path.read_bytes
            opened_assets = []

            def observe_reads(path):
                if path.suffix == '.png':
                    opened_assets.append(path)
                return read_bytes(path)

            with patch.object(Path, 'read_bytes', observe_reads):
                state.discard({'expected_revision': status['authored_revision'],
                               'accepted_revision': state.accepted['revision']})
                self.assertFalse(state.status()['can_discard'])
            self.assertEqual(opened_assets, [])

    def test_published_cache_does_not_hide_an_asset_edit_during_rename(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state = self.prepare(root)
            status = state.status()
            rename = Path.rename

            def edit_after_publication(path, destination):
                result = rename(path, destination)
                if path.name.startswith('.working-prepare-'):
                    (root / 'working/assets/image.png').write_bytes(b'external edit during publication')
                return result

            with patch.object(Path, 'rename', edit_after_publication):
                state.discard({'expected_revision': status['authored_revision'],
                               'accepted_revision': state.accepted['revision']})
            self.assertFalse(state.status()['can_accept'])
            self.assertEqual(state.input_cache.load(root / 'working'), load_input(root / 'working'))

    def test_accept_rejects_source_changes_during_copy_with_a_warm_cache(self):
        self.check_replacement_race('accept', 'working', 'Source changed')

    def test_discard_rejects_working_changes_during_copy_with_a_warm_cache(self):
        self.check_replacement_race('discard', 'accepted', 'Working changed')

    def check_replacement_race(self, action, source, message):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state = self.prepare(root)
            status = state.status()
            state.input_cache.load(root / 'accepted')
            accepted = (root / 'accepted/book.json').read_bytes()
            published = (root / 'book.json').read_bytes()
            copytree = shutil.copytree
            edited = False

            def edit_after_copy(path, destination, *args, **kwargs):
                nonlocal edited
                result = copytree(path, destination, *args, **kwargs)
                if Path(path) == root / source / 'renders' / source and not edited:
                    edited = True
                    book = read_json(root / 'working/book.json')
                    book['toc']['depth'] = 3
                    write_json(root / 'working/book.json', book)
                return result

            command = {'revision': state.working['revision']} if action == 'accept' else {
                'expected_revision': status['authored_revision'], 'accepted_revision': state.accepted['revision']}
            with patch.object(shutil, 'copytree', edit_after_copy):
                with self.assertRaisesRegex(ValueError, message):
                    getattr(state, action)(command)
            self.assertTrue(edited)
            self.assertEqual((root / 'accepted/book.json').read_bytes(), accepted)
            self.assertEqual((root / 'book.json').read_bytes(), published)
            self.assertEqual(read_json(root / 'working/book.json')['toc']['depth'], 3)
            self.assertEqual(list(root.glob('.*-prepare-*')), [])

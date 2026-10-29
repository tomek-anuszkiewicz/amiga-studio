"""Editing chapter labels and filenames preserves source recipes and bundle safety."""

from http.server import ThreadingHTTPServer
from pathlib import Path
import sys
import tempfile
import threading
import unittest

from browser_session import browser_page

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.model import export_markdown, load_input, write_json
from review.source_cleanup import preview
from review.source_validation import validate_book
from review.service import Handler, ReviewState


class RenameChaptersTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'First Manual'}]})
        self.original_title = '{{text page=1 block=0 chars=0:5}}'
        self.book = {'toc': {'depth': 2}, 'chapters': [
            {'id': 'first', 'title': self.original_title, 'markdown_file': 'first.md',
             'elements': [{'id': 'heading-1', 'type': 'heading', 'heading_level': 1,
                           'role': None, 'visible': True,
                           'md_text': '# ' + self.original_title,
                           'pdf_regions': [{'page': 1, 'bbox': [0, 0, 100, 100]}]}]},
            {'id': 'second', 'title': 'Second', 'markdown_file': 'second.md',
             'elements': []}]}
        write_json(self.root / 'working/book.json', self.book)
        self.state = ReviewState(self.root)

    def command(self, first_title, first_file, second_file='second.md'):
        return {'expected_revision': load_input(self.root / 'working')[2],
                'toc': {'title': 'Table of Contents',
                        'markdown_file': 'table-of-contents.md'},
                'chapters': [{'id': 'first', 'title': first_title,
                              'markdown_file': first_file},
                             {'id': 'second', 'title': 'Second',
                              'markdown_file': second_file}]}

    def test_title_aligns_to_pdf_and_filename_changes_without_editing_elements(self):
        self.state.rename_chapters(self.command('First Manual', 'first-manual.md'))
        book = load_input(self.root / 'working')[0]
        self.assertEqual(book['chapters'][0]['resolved_title'], 'First Manual')
        self.assertEqual(book['chapters'][0]['title'],
                         '{{text page=1 block=0 chars=0:12}}')
        self.assertEqual(book['chapters'][0]['markdown_file'], 'first-manual.md')
        self.assertEqual(book['chapters'][0]['elements'][0]['md_text'],
                         '# ' + self.original_title)

    def test_title_uses_matching_visible_heading_page(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0002.json',
                   {'blocks': [{'block_id': 0, 'text': 'Visual Heading'}]})
        self.book['chapters'][0]['elements'][0]['visible'] = False
        self.book['chapters'][0]['elements'].append({
            'id': 'heading-2', 'type': 'heading', 'heading_level': 1,
            'role': None, 'visible': True,
            'md_text': '# {{text page=2 block=0 chars=0:14}}',
            'pdf_regions': [{'page': 2, 'bbox': [0, 0, 100, 100]}]})
        write_json(self.root / 'working/book.json', self.book)
        self.state.rename_chapters(self.command('Visual Heading', 'visual-heading.md'))
        book = load_input(self.root / 'working')[0]
        self.assertEqual(book['chapters'][0]['title'],
                         '{{text page=2 block=0 chars=0:14}}')

    def test_invalid_or_duplicate_filename_keeps_entire_book_unchanged(self):
        baseline = (self.root / 'working/book.json').read_bytes()
        for filename in ('second.md', '../outside.md', 'CON.md', 'bad?.md'):
            with self.subTest(filename=filename), self.assertRaises(ValueError):
                self.state.rename_chapters(self.command('First Manual', filename))
            self.assertEqual((self.root / 'working/book.json').read_bytes(), baseline)

    def test_generated_contents_rename_updates_export_and_link_targets(self):
        command = self.command(self.original_title, 'first.md')
        command['toc'] = {'title': 'Contents', 'markdown_file': 'contents.md'}
        self.state.rename_chapters(command)
        book = load_input(self.root / 'working')[0]
        self.assertEqual(book['toc']['title'], 'Contents')
        self.assertEqual(book['toc']['markdown_file'], 'contents.md')
        export_markdown(book, self.root / 'working')
        self.assertIn('# Contents', (self.root / 'working/contents.md').read_text(encoding='utf-8'))
        self.assertIn('first.md#^element-heading--1',
                      (self.root / 'working/contents.md').read_text(encoding='utf-8'))
        self.state.toc({'expected_revision': load_input(self.root / 'working')[2], 'depth': 3})
        self.assertEqual(load_input(self.root / 'working')[0]['toc']['title'], 'Contents')
        self.assertEqual(load_input(self.root / 'working')[0]['toc']['markdown_file'], 'contents.md')

    def test_contents_filename_cannot_collide_with_chapter(self):
        command = self.command(self.original_title, 'first.md')
        command['toc']['markdown_file'] = 'second.md'
        baseline = (self.root / 'working/book.json').read_bytes()
        with self.assertRaises(ValueError):
            self.state.rename_chapters(command)
        self.assertEqual((self.root / 'working/book.json').read_bytes(), baseline)

    def test_edited_contents_title_is_checked_and_can_be_bound_to_pdf(self):
        command = self.command(self.original_title, 'first.md')
        command['toc']['title'] = 'First Manual'
        self.state.rename_chapters(command)
        book = load_input(self.root / 'working')[0]
        issues = validate_book(book, self.root / 'pdf-to-markdown')['issues']
        self.assertTrue(any(issue['chapter_id'] == 'toc' and issue['field'] == 'title'
                            for issue in issues))
        finding = next(item for item in preview(book, self.root / 'pdf-to-markdown')
                       if item['path'] == '/toc/title')
        self.assertEqual(finding['status'], 'matched')
        self.state.apply_source_cleanup({'expected_revision': load_input(self.root / 'working')[2],
                                         'selections': [finding['id']]})
        self.assertEqual(load_input(self.root / 'working')[0]['toc']['title'],
                         '{{text page=1 block=0 chars=0:12}}')

    def test_missing_chapter_or_stale_revision_keeps_book_unchanged(self):
        baseline = (self.root / 'working/book.json').read_bytes()
        command = self.command('First Manual', 'renamed.md')
        for bad in ({**command, 'chapters': command['chapters'][:1]},
                    {**command, 'expected_revision': 'stale'}):
            with self.assertRaises(ValueError):
                self.state.rename_chapters(bad)
            self.assertEqual((self.root / 'working/book.json').read_bytes(), baseline)

    def test_browser_action_edits_contents_and_chapter_in_one_save(self):
        server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        server.daemon_threads = True
        server.state = self.state
        threading.Thread(target=server.serve_forever, daemon=True).start()
        rendering = threading.Thread(target=self.state.render_loop, daemon=True)
        rendering.start()
        try:
            with browser_page(viewport={'width': 1500, 'height': 900}) as page:
                errors = []
                page.on('pageerror', lambda error: errors.append(str(error)))
                page.goto(f'http://127.0.0.1:{server.server_port}')
                page.wait_for_function('state?.working && !state.pending && !loading')
                page.locator('#book-actions').click()
                page.locator('[data-book-task="rename_chapters"]').click()
                self.assertEqual(page.locator('.rename-chapter-row').count(), 3)
                page.locator('.rename-chapter-row').nth(0).locator('textarea').fill('Contents')
                page.locator('.rename-chapter-row').nth(0).locator('input').fill('contents.md')
                page.locator('.rename-chapter-row').nth(1).locator('textarea').fill('First Manual')
                page.locator('.rename-chapter-row').nth(1).locator('input').fill('first-manual.md')
                page.locator('#rename-chapters-save').click()
                page.wait_for_function('state?.working?.toc.title === "Contents" && !state.pending && !loading')
                book = load_input(self.root / 'working')[0]
                self.assertEqual(book['chapters'][0]['resolved_title'], 'First Manual')
                self.assertEqual(book['chapters'][0]['markdown_file'], 'first-manual.md')
                self.assertEqual(book['toc']['markdown_file'], 'contents.md')
                self.assertTrue((self.root / 'working/contents.md').is_file())
                self.assertTrue((self.root / 'working/first-manual.md').is_file())
                self.assertFalse((self.root / 'working/first.md').exists())
                self.assertFalse(errors)
        finally:
            self.state.stopping.set()
            with self.state.condition:
                self.state.condition.notify_all()
            server.shutdown()
            server.server_close()
            rendering.join(20)


if __name__ == '__main__':
    unittest.main()

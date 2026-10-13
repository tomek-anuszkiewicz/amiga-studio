"""Chapter boundaries and Discard Changes preserve coherent authored bundles."""

from http.server import ThreadingHTTPServer
import json
from pathlib import Path
import sys
import tempfile
import threading
import unittest

from playwright.sync_api import sync_playwright

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.model import changes, load_input, write_json
from review.service import Handler, ReviewState


class NewChapterTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        blocks = [{'block_id': i, 'text': title} for i, title in enumerate(('Introduction', 'Second', 'Tail'))]
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json', {'blocks': blocks})
        refs = ['{{text page=1 block=' + str(b['block_id']) + ' chars=0:' + str(len(b['text'])) + '}}' for b in blocks]
        def heading(eid, reference):
            return {'id': eid, 'type': 'heading', 'heading_level': 1, 'md_text': '# ' + reference,
                    'role': None, 'pdf_regions': [], 'visible': True}
        def prose(eid, text):
            return {'id': eid, 'type': 'prose', 'md_text': text,
                    'role': None, 'pdf_regions': [], 'visible': True}
        self.book = {'toc': {'depth': 2}, 'chapters': [
            {'id': 'original', 'title': refs[0], 'markdown_file': 'original.md', 'elements': [
                heading('intro', refs[0]), prose('before', 'Opening text.'),
                heading('division', refs[1]), prose('body', 'Keep this text.') ]},
            # Deliberate filename collision with the chapter derived from Second.
            {'id': 'following', 'title': refs[2], 'markdown_file': 'second.md', 'elements': [
                heading('tail', refs[2]), prose('last', 'Following chapter.')]}]}
        write_json(self.root / 'working/book.json', self.book)
        self.state = ReviewState(self.root)

    def command(self, eid='division'):
        return {'id': eid, 'expected_revision': load_input(self.root / 'working')[2]}

    def test_boundary_preserves_text_and_following_chapter_and_avoids_filename_collision(self):
        before = load_input(self.root / 'working')[0]
        self.state.start_chapter(self.command())
        after = load_input(self.root / 'working')[0]
        self.assertEqual([e for c in before['chapters'] for e in c['elements']],
                         [e for c in after['chapters'] for e in c['elements']])
        self.assertEqual(after['chapters'][2], before['chapters'][1])
        created = after['chapters'][1]
        self.assertEqual(created['resolved_title'], 'Second')
        self.assertEqual(created['title'], '{{text page=1 block=1 chars=0:6}}')
        self.assertEqual([e['id'] for e in after['chapters'][0]['elements']], ['intro', 'before'])
        self.assertEqual([e['id'] for e in created['elements']], ['division', 'body'])
        self.assertNotIn(created['markdown_file'], {'original.md', 'second.md'})
        diff = changes(before, after)
        self.assertEqual({item['id'] for item in diff}, {'generated-toc', 'division'})
        boundary = next(item for item in diff if item['id'] == 'division')
        self.assertEqual(boundary['kind'], 'chapter_boundary')
        self.assertEqual(set(boundary['chapter_ids']), {'original', created['id']})
        created['elements'][1]['md_text'] = 'A separate text edit.'
        created['elements'][1]['resolved_md_text'] = 'A separate text edit.'
        self.assertEqual({item['id'] for item in changes(before, after)}, {'generated-toc', 'division', 'body'})

    def test_first_missing_or_stale_element_does_not_write(self):
        before = (self.root / 'working/book.json').read_bytes()
        for command in (self.command('intro'), self.command('missing'),
                        {**self.command(), 'expected_revision': 'stale'}):
            with self.assertRaises(ValueError):
                self.state.start_chapter(command)
            self.assertEqual((self.root / 'working/book.json').read_bytes(), before)

    def test_ui_create_discard_accept_and_discard_again_restore_complete_chapter_state(self):
        server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        server.daemon_threads = True
        server.state = self.state
        threading.Thread(target=server.serve_forever, daemon=True).start()
        rendering = threading.Thread(target=self.state.render_loop, daemon=True)
        rendering.start()
        def authored():
            return json.loads((self.root / 'working/book.json').read_text(encoding='utf-8'))
        def accepted_files():
            directory = self.root / 'accepted'
            return {p.relative_to(directory): p.read_bytes() for p in directory.rglob('*') if p.is_file()}
        try:
            with sync_playwright() as p:
                browser = p.chromium.launch(channel='msedge')
                page = browser.new_page(viewport={'width': 1600, 'height': 950})
                errors = []
                page.on('pageerror', lambda e: errors.append(str(e)))
                page.goto(f'http://127.0.0.1:{server.server_port}')
                page.wait_for_function('state?.accepted && state.can_accept && !loading')
                baseline = authored()
                accepted = accepted_files()
                def split(eid, count):
                    page.locator(f'#working [data-element="{eid}"]').click(button='right')
                    page.locator('#menu-new-chapter').click()
                    page.wait_for_function(f'state?.working.chapters.length === {count} && state.can_discard && !loading')
                    return authored()['chapters']
                page.locator('#working [data-element="intro"]').click(button='right')
                self.assertFalse(page.locator('#menu-new-chapter').is_enabled())
                page.keyboard.press('Escape')
                created = split('division', 3)[1]
                filename = created['markdown_file']
                self.assertEqual(accepted_files(), accepted)
                self.assertIn('Second', (self.root / 'working' / filename).read_text(encoding='utf-8'))
                self.assertNotIn('Second', (self.root / 'working/original.md').read_text(encoding='utf-8'))
                self.assertIn(filename + '#element-division', (self.root / 'working/table-of-contents.md').read_text(encoding='utf-8'))
                self.assertEqual((self.root / 'working/second.md').read_bytes(), accepted[Path('second.md')])
                self.assertEqual(page.locator('#chapter option').count(), 4)
                for version in ('working', 'accepted'):
                    self.assertEqual(page.locator(f'#{version} [data-element].changed').count(), 2)
                    self.assertTrue(page.locator(f'#{version} [data-element="division"]').evaluate('(e) => e.classList.contains("changed")'))
                    self.assertFalse(page.locator(f'#{version} [data-element="body"]').evaluate('(e) => e.classList.contains("changed")'))
                    self.assertTrue(page.locator(f'#{version} [data-chapter="original"] > .chapter-toggle').evaluate('(e) => e.classList.contains("changed")'))
                self.assertTrue(page.locator(f'#working [data-chapter="{created["id"]}"] > .chapter-toggle').evaluate('(e) => e.classList.contains("changed")'))
                self.assertFalse(page.locator('#working [data-chapter="following"] > .chapter-toggle').evaluate('(e) => e.classList.contains("changed")'))
                Path('.agent/tmp').mkdir(parents=True, exist_ok=True)
                page.screenshot(path='.agent/tmp/book-new-chapter-markers.png')
                page.locator('#discard').click()
                page.wait_for_function('state?.working.chapters.length === 2 && !state.can_discard && !loading')
                self.assertEqual(authored(), baseline)
                self.assertEqual(load_input(self.root / 'working'), load_input(self.root / 'accepted'))
                self.assertFalse((self.root / 'working' / filename).exists())
                self.assertEqual(accepted_files(), accepted)
                self.assertEqual(page.locator('#chapter option').count(), 3)
                self.assertEqual(page.locator('[data-element].changed, .chapter-toggle.changed').count(), 0)
                for name in ('original.md', 'second.md', 'table-of-contents.md'):
                    self.assertEqual((self.root / 'working' / name).read_bytes(), accepted[Path(name)])
                split('division', 3)
                page.locator('#accept').click()
                page.wait_for_function('state?.accepted.chapters.length === 3 && !state.can_discard && !loading')
                self.assertEqual(page.locator('[data-element].changed, .chapter-toggle.changed').count(), 0)
                accepted_split = accepted_files()
                accepted_model = authored()
                extra = split('body', 4)[2]['markdown_file']
                self.assertTrue((self.root / 'working' / extra).exists())
                page.locator('#discard').click()
                page.wait_for_function('state?.working.chapters.length === 3 && !state.can_discard && !loading')
                self.assertEqual(authored(), accepted_model)
                self.assertFalse((self.root / 'working' / extra).exists())
                self.assertEqual(accepted_files(), accepted_split)
                self.assertFalse(errors)
                browser.close()
        finally:
            self.state.stopping.set()
            with self.state.condition:
                self.state.condition.notify_all()
            server.shutdown()
            server.server_close()
            rendering.join(20)

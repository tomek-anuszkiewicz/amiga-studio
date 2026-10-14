"""Hide a source page atomically from every review panel's context menu."""
from http.server import ThreadingHTTPServer
from pathlib import Path
import sys
import tempfile
import threading
import unittest

from PIL import Image
from playwright.sync_api import sync_playwright

REVIEW = Path(__file__).resolve().parents[4] / 'tools/bootstrap/book-to-markdown'
sys.path.insert(0, str(REVIEW))
from review.model import load_input, write_json
from review.service import Handler, ReviewState


class HidePageReviewTest(unittest.TestCase):
    def test_page_command_from_all_panels_and_pdf_background(self):
        book = {'toc': {'depth': 2}, 'chapters': [{'id': 'chapter', 'title': 'Example',
            'markdown_file': 'example.md', 'elements': [
                {'id': 'first', 'type': 'prose', 'md_text': 'First page text.',
                 'pdf_regions': [{'page': 1, 'bbox': [20, 20, 180, 60]}], 'role': None},
                {'id': 'spanning', 'type': 'prose', 'md_text': 'Text spanning two pages.',
                 'pdf_regions': [{'page': 1, 'bbox': [20, 80, 180, 120]},
                                 {'page': 2, 'bbox': [20, 20, 180, 60]}], 'role': None},
                {'id': 'other', 'type': 'prose', 'md_text': 'Other page text.',
                 'pdf_regions': [{'page': 2, 'bbox': [20, 80, 180, 120]}], 'role': None}]}]}
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'pdf/pages').mkdir(parents=True)
            for number in (1, 2):
                Image.new('RGB', (200, 200), 'white').save(root / f'pdf/pages/page_{number:04d}.png')
            write_json(root / 'working/book.json', book)
            state = ReviewState(root)
            server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
            server.daemon_threads = True
            server.state = state
            worker = threading.Thread(target=state.render_loop)
            http = threading.Thread(target=server.serve_forever)
            worker.start()
            http.start()
            try:
                with sync_playwright() as p:
                    browser = p.chromium.launch(channel='msedge')
                    page = browser.new_page(viewport={'width': 1600, 'height': 950})
                    errors = []
                    page.on('pageerror', lambda error: errors.append(str(error)))
                    url = f'http://127.0.0.1:{server.server_port}'
                    page.goto(url)
                    page.wait_for_function('state?.can_accept && !loading')
                    for panel in ('working', 'accepted', 'pdf', 'background'):
                        with self.subTest(panel=panel):
                            if panel == 'background':
                                page.locator('#pdf .page[data-page="1"]').click(button='right', position={'x': 5, 'y': 5})
                            elif panel == 'pdf':
                                page.locator('#pdf [data-id="first"]').click(button='right')
                            else:
                                page.locator(f'#{panel} [data-element="first"]').click(button='right')
                            self.assertTrue(page.locator('#menu-hide-page').is_enabled())
                            self.assertIn('page 1', page.locator('#menu-hide-page').get_attribute('title'))
                            stale = state.status()['authored_revision']
                            page.locator('#menu-hide-page').click(timeout=5000)
                            page.wait_for_function('!loading && state.can_discard && state.working.chapters[0].elements[0].visible === false')
                            restored = load_input(root / 'working')[0]['chapters'][0]['elements']
                            self.assertEqual([e.get('visible', True) for e in restored], [False, False, True])
                            self.assertFalse(page.locator('#pdf .page[data-page="1"]').is_visible())
                            self.assertTrue(page.locator('#pdf .page[data-page="2"]').is_visible())
                            self.assertTrue(page.locator('#accepted [data-element="first"]').is_visible())
                            response = page.request.post(url + '/api/hide-page', data={'page': 2, 'expected_revision': stale})
                            self.assertEqual(response.status, 409)
                            page.locator('#show-hidden').check()
                            page.locator('#working [data-element="first"]').click(button='right')
                            self.assertFalse(page.locator('#menu-hide-page').is_enabled())
                            page.keyboard.press('Escape')
                            page.locator('#show-hidden').uncheck()
                            page.locator('#discard').click()
                            page.wait_for_function('!loading && state.can_accept && !state.changes.length')
                    self.assertEqual(errors, [])
                    browser.close()
            finally:
                state.stopping.set()
                with state.condition:
                    state.condition.notify_all()
                server.shutdown()
                server.server_close()
                worker.join(timeout=15)
                http.join(timeout=5)

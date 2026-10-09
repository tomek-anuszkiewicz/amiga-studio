"""Visibility editing through the real HTTP queue and browser review UI."""
from http.server import ThreadingHTTPServer
from pathlib import Path
import sys
import tempfile
import threading
import unittest

from PIL import Image
from playwright.sync_api import sync_playwright

REVIEW = Path(__file__).resolve().parents[1] / 'tools/bootstrap/book-to-markdown'
sys.path.insert(0, str(REVIEW))
from review.model import load_input, write_json
from review.service import Handler, ReviewState


class VisibilityReviewTest(unittest.TestCase):
    def test_hide_preview_accept_and_unhide(self):
        book = {'toc': {'depth': 2}, 'chapters': [{'id': 'chapter', 'title': 'Example',
            'markdown_file': 'example.md', 'elements': [
                {'id': 'heading', 'type': 'heading', 'heading_level': 2,
                 'md_text': '## Example heading', 'pdf_regions': [{'page': 1, 'bbox': [20, 20, 180, 60]}]},
                {'id': 'prose', 'type': 'prose', 'md_text': 'Ordinary visible paragraph.', 'pdf_regions': [{'page': 2, 'bbox': [20, 20, 180, 60]}]}]}]}
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'pdf/pages').mkdir(parents=True)
            for number in (1, 2, 3):
                Image.new('RGB', (200, 300), 'white').save(root / f'pdf/pages/page_{number:04d}.png')
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
                with sync_playwright() as playwright:
                    browser = playwright.chromium.launch(channel='msedge')
                    page = browser.new_page(viewport={'width': 1600, 'height': 950})
                    errors = []
                    page.on('pageerror', lambda error: errors.append(str(error)))
                    page.goto(f'http://127.0.0.1:{server.server_port}/')
                    page.wait_for_function('state?.can_accept && !loading')
                    self.assertFalse(page.locator("#discard").is_enabled())
                    heading = page.locator('#working [data-element="heading"]')
                    heading.click(button='right')
                    self.assertEqual(page.locator('#menu-discard').inner_text(), 'Discard')
                    self.assertFalse(page.locator('#menu-discard').is_enabled())
                    page.keyboard.press('Escape')
                    page.locator('#working [data-element="prose"]').click(button='right')
                    page.locator('#menu-visibility').click()
                    page.wait_for_function('!loading && state.can_accept && state.working.chapters[0].elements[1].visible === false')
                    heading.click(button='right')
                    page.locator('#menu-heading').click()
                    page.locator('#level').select_option('4')
                    page.locator('#save-heading').click()
                    page.wait_for_function('!loading && state.can_accept && state.working.chapters[0].elements[0].heading_level === 4')
                    heading.click(button='right')
                    self.assertTrue(page.locator('#menu-discard').is_enabled())
                    page.locator('#menu-discard').click()
                    page.wait_for_function('!loading && state.can_accept && state.working.chapters[0].elements[0].heading_level === 2')
                    self.assertFalse(load_input(root / 'working')[0]['chapters'][0]['elements'][1]['visible'])
                    self.assertEqual([change['id'] for change in state.status()['changes']], ['prose'])
                    page.locator('#show-hidden').check()
                    page.locator('#pdf [data-id="prose"]').click(button='right')
                    page.locator('#menu-discard').click()
                    page.wait_for_function('!loading && state.can_accept && state.changes.length === 0')
                    page.locator('#show-hidden').uncheck()
                    page.locator('#accepted [data-element="heading"]').click(button='right')
                    self.assertFalse(page.locator('#menu-discard').is_enabled())
                    page.keyboard.press('Escape')
                    heading.click()
                    self.assertFalse(page.locator('#heading-dialog').is_visible())
                    page.locator('#element-actions').click()
                    self.assertEqual(page.locator('#menu-visibility').inner_text(), 'Hide Element')
                    self.assertTrue(page.locator('#menu-heading').is_visible())
                    page.locator('#menu-heading').click()
                    self.assertTrue(page.locator('#heading-dialog').is_visible())
                    page.locator('#heading-dialog button[value="cancel"]').click()
                    stale_revision = state.status()['authored_revision']
                    heading.click(button='right')
                    page.locator('#menu-visibility').click()
                    page.wait_for_function('!loading && state.can_accept && state.working.chapters[0].elements[0].visible === false')
                    self.assertFalse(heading.is_visible())
                    pdf_heading = page.locator('#pdf [data-id="heading"]')
                    self.assertEqual(pdf_heading.count(), 1)
                    self.assertFalse(pdf_heading.is_visible())
                    pdf_page = page.locator('#pdf .page[data-page="1"]')
                    self.assertFalse(pdf_page.is_visible())
                    self.assertTrue(page.locator('#pdf .page[data-page="2"]').is_visible())
                    self.assertFalse(page.locator('#pdf .page[data-page="3"]').is_visible())
                    self.assertTrue(page.locator('#accepted [data-element="heading"]').is_visible())
                    hidden = load_input(root / 'working')[0]
                    self.assertFalse(hidden['chapters'][0]['elements'][0]['visible'])
                    requests = []
                    page.on('request', lambda request: requests.append(request.url))
                    page.locator('#show-hidden').check()
                    self.assertTrue(heading.is_visible())
                    self.assertTrue(pdf_heading.is_visible())
                    self.assertTrue(pdf_page.is_visible())
                    self.assertTrue(page.locator('#pdf .page[data-page="3"]').is_visible())
                    self.assertIn('hidden-element', heading.get_attribute('class'))
                    self.assertIn('radial-gradient', heading.evaluate('(n)=>getComputedStyle(n,"::after").backgroundImage'))
                    self.assertFalse(any('/api/' in url for url in requests))
                    self.assertEqual(load_input(root / 'working')[0], hidden)
                    heading.hover()
                    hint = page.locator('#element-hint').inner_text()
                    self.assertIn('Visible', hint)
                    self.assertNotIn('Show hidden', hint)
                    self.assertNotIn('Example heading', page.locator('#working [data-element="generated-toc"]').inner_text())
                    pdf_heading.click(button='right')
                    self.assertEqual(page.locator('#menu-visibility').inner_text(), 'Unhide Element')
                    self.assertTrue(page.locator('#menu-visibility').is_enabled())
                    page.keyboard.press('Escape')
                    heading.focus()
                    page.keyboard.press('Shift+F10')
                    self.assertEqual(page.locator('#menu-visibility').inner_text(), 'Unhide Element')
                    page.keyboard.press('Escape')
                    self.assertFalse(page.locator('#element-menu').is_visible())
                    page.locator('#accepted [data-element="heading"]').click(button='right')
                    self.assertFalse(page.locator('#menu-visibility').is_enabled())
                    page.keyboard.press('Escape')
                    response = page.request.post(f'http://127.0.0.1:{server.server_port}/api/visibility', data={
                        'id': 'heading', 'visible': True, 'expected_revision': stale_revision})
                    self.assertEqual(response.status, 409)
                    page.locator('#accept').click()
                    page.wait_for_function('!loading && state.accepted.chapters[0].elements[0].visible === false')
                    self.assertNotIn('Example heading', (root / 'accepted/example.md').read_text(encoding='utf-8'))
                    self.assertNotIn('Example heading', (root / 'accepted/table-of-contents.md').read_text(encoding='utf-8'))
                    heading.click(button='right')
                    Path('.agent/tmp').mkdir(parents=True, exist_ok=True)
                    page.screenshot(path='.agent/tmp/book-review-hidden-menu.png')
                    page.locator('#menu-visibility').click()
                    page.wait_for_function('!loading && state.can_accept && state.working.chapters[0].elements[0].visible === true')
                    self.assertNotIn('hidden-element', heading.get_attribute('class') or '')
                    self.assertIn('Example heading', page.locator('#working [data-element="generated-toc"]').inner_text())
                    page.locator('#show-hidden').uncheck()
                    self.assertTrue(heading.is_visible())
                    self.assertTrue(pdf_page.is_visible())
                    self.assertFalse(page.locator('#accepted [data-element="heading"]').is_visible())
                    page.locator('#toc-depth').select_option('3')
                    page.wait_for_function('!loading && state.can_discard && state.working.toc.depth === 3')
                    response = page.request.post(f'http://127.0.0.1:{server.server_port}/api/discard', data={
                        'expected_revision': 'stale', 'accepted_revision': state.accepted['revision']})
                    self.assertEqual(response.status, 409)
                    page.locator('#discard').click()
                    page.wait_for_function('!loading && !state.can_discard && state.changes.length === 0 && state.working.toc.depth === 2')
                    self.assertFalse(heading.is_visible())
                    self.assertFalse(pdf_page.is_visible())
                    self.assertFalse(page.locator('#discard').is_enabled())
                    self.assertEqual(load_input(root / 'working'), load_input(root / 'accepted'))
                    for filename in ('example.md', 'table-of-contents.md'):
                        self.assertEqual((root / 'working' / filename).read_bytes(),
                                         (root / 'accepted' / filename).read_bytes())
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

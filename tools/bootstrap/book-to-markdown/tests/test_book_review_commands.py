"""Move, change type and merge tables through the actual review HTTP/browser flow."""

from http.server import ThreadingHTTPServer
from pathlib import Path
import sys
import tempfile
import threading
import unittest

from PIL import Image
from playwright.sync_api import sync_playwright

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.model import load_input, write_json
from review.service import Handler, ReviewState


class ReviewCommandFlowTests(unittest.TestCase):
    def test_move_type_table_merge_reload_and_discard(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            reference = '{{text page=1 block=0 chars=0:10}}'
            write_json(root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                       {'blocks': [{'block_id': 0, 'text': 'First text'}]})
            def item(eid, kind, text, top):
                return {'id': eid, 'type': kind, 'role': None, 'visible': True, 'md_text': text,
                        'pdf_regions': [{'page': 1, 'bbox': [20, top, 180, top + 30]}],
                        **({'table_format': 'markdown'} if kind == 'table' else {})}
            book = {'toc': {'depth': 2}, 'chapters': [{'id': 'chapter', 'title': 'Example',
                    'markdown_file': 'example.md', 'elements': [
                        item('first', 'prose', reference, 20),
                        item('second', 'prose', 'Second text', 60),
                        item('table1', 'table', '| Pin | Name |\n| --- | --- |\n| 1 | DATA |', 100),
                        item('table2', 'table', '| Pin | Name |\n| --- | --- |\n| 2 | CLOCK |', 140)]}]}
            write_json(root / 'working/book.json', book)
            (root / 'pdf/pages').mkdir(parents=True)
            Image.new('RGB', (200, 300), 'white').save(root / 'pdf/pages/page_0001.png')
            state = ReviewState(root)
            server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
            server.daemon_threads = True
            server.state = state
            threading.Thread(target=server.serve_forever, daemon=True).start()
            rendering = threading.Thread(target=state.render_loop, daemon=True)
            rendering.start()
            try:
                with sync_playwright() as p:
                    browser = p.chromium.launch(channel='msedge')
                    page = browser.new_page(viewport={'width': 1600, 'height': 950})
                    errors = []
                    page.on('pageerror', lambda e: errors.append(str(e)))
                    page.goto(f'http://127.0.0.1:{server.server_port}')
                    page.wait_for_function('state?.accepted && state.can_accept && !loading')
                    baseline = load_input(root / 'working')
                    accepted = (root / 'accepted/book.json').read_bytes()
                    def menu(eid):
                        page.locator(f'#working [data-element="{eid}"]').click(button='right')
                    menu('first')
                    self.assertFalse(page.locator('#menu-move-up').is_enabled())
                    page.locator('#menu-move-down').click()
                    page.wait_for_function('!loading && state.can_accept && state.working.chapters[0].elements[0].id === "second"')
                    self.assertEqual([e['id'] for e in load_input(root / 'working')[0]['chapters'][0]['elements']],
                                     ['second', 'first', 'table1', 'table2'])
                    menu('first')
                    page.locator('#menu-move-up').click()
                    page.wait_for_function('!loading && state.can_accept && !state.changes.length')
                    self.assertEqual(load_input(root / 'working'), baseline)
                    menu('first')
                    page.locator('#menu-type').click()
                    page.locator('#type-menu').get_by_role('menuitemradio', name='heading', exact=True).click()
                    page.wait_for_function('!loading && state.can_accept && state.working.chapters[0].elements[0].type === "heading"')
                    changed = load_input(root / 'working')[0]['chapters'][0]['elements'][0]
                    self.assertEqual(changed['md_text'], '# ' + reference)
                    self.assertEqual(changed['heading_level'], 1)
                    self.assertEqual(page.locator('#working [data-element="first"] h1').inner_text(), 'First text')
                    menu('first')
                    page.locator('#menu-type').click()
                    page.locator('#type-menu').get_by_role('menuitemradio', name='prose', exact=True).click()
                    page.wait_for_function('!loading && state.can_accept && !state.changes.length')
                    page.reload()
                    page.wait_for_function('state?.can_accept && !loading')
                    self.assertEqual(load_input(root / 'working'), baseline)
                    menu('table1')
                    page.locator('#menu-merge-tables').click()
                    page.wait_for_function('!loading && state.can_accept && state.working.chapters[0].elements.length === 3')
                    merged = load_input(root / 'working')[0]['chapters'][0]['elements'][2]
                    self.assertIn('| 1 | DATA |', merged['resolved_md_text'])
                    self.assertIn('| 2 | CLOCK |', merged['resolved_md_text'])
                    self.assertEqual(merged['resolved_md_text'].count('| Pin | Name |'), 1)
                    self.assertEqual(merged['pdf_regions'], [book['chapters'][0]['elements'][i]['pdf_regions'][0] for i in (2, 3)])
                    self.assertEqual(page.locator('#working [data-element="table1"] tr').count(), 3)
                    self.assertEqual((root / 'accepted/book.json').read_bytes(), accepted)
                    page.reload()
                    page.wait_for_function('state?.can_discard && !loading')
                    self.assertEqual(page.locator('#working [data-element="table1"] tr').count(), 3)
                    page.locator('#discard').click()
                    page.wait_for_function('state?.can_accept && !state.can_discard && !loading')
                    self.assertEqual(load_input(root / 'working'), baseline)
                    self.assertEqual((root / 'accepted/book.json').read_bytes(), accepted)
                    self.assertEqual(page.locator('#working [data-element="table1"] tr').count(), 2)
                    self.assertEqual(page.locator('#working [data-element="table2"] tr').count(), 2)
                    self.assertFalse(errors)
                    browser.close()
            finally:
                state.stopping.set()
                with state.condition:
                    state.condition.notify_all()
                server.shutdown()
                server.server_close()
                rendering.join(20)

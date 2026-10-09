"""Bound change navigation and align panes when a changed element is hidden."""
from copy import deepcopy
from io import BytesIO
import json
from pathlib import Path
import unittest
from urllib.parse import urlsplit

from PIL import Image
from playwright.sync_api import sync_playwright

REVIEW = Path(__file__).resolve().parents[1] / 'tools/bootstrap/book-to-markdown/review'


class ChangeNavigationTest(unittest.TestCase):
    def test_navigation_bounds(self):
        self.check_navigation(True)

    def test_hidden_counterparts_are_scrolled(self):
        self.check_navigation(False)

    def check_navigation(self, check_bounds):
        elements = [{'id': f'p{i}', 'type': 'prose',
                     'pdf_regions': [{'page': i+1, 'bbox': [0, 0, 200, 100]}]} for i in range(30)]
        accepted = {'revision': 'before', 'toc': {'depth': 2}, 'chapters': [
            {'id': 'chapter', 'title': 'Example', 'elements': elements}],
            'render': {'chapters': [{'id': 'chapter', 'document': 'chapter.html'}]}}
        working = deepcopy(accepted)
        working['revision'] = 'after'
        for i in (8, 20):
            working['chapters'][0]['elements'][i]['visible'] = False
        status = {'working': working, 'accepted': accepted, 'can_accept': True,
                  'authored_revision': 'after', 'pending': False, 'error': None,
                  'changes': [{'id': f'p{i}'} for i in (8, 12, 20)], 'heading_types': [],
                  'pdf_pages': [{'page': i+1, 'width': 200, 'height': 100,
                                 'image': f'pdf/page_{i+1}.png'} for i in range(30)]}
        image = BytesIO()
        Image.new('RGB', (200, 100), 'white').save(image, format='PNG')
        markup = ''.join(f'<div data-element="p{i}"><div style="height:120px">Paragraph {i}</div></div>'
                         for i in range(30))

        def respond(route):
            path = urlsplit(route.request.url).path
            if path == '/api/status':
                route.fulfill(content_type='application/json', body=json.dumps(status))
            elif path == '/api/unsupported':
                route.fulfill(status=404, content_type='text/plain', body='Not found')
            elif path.endswith('.png'):
                route.fulfill(content_type='image/png', body=image.getvalue())
            elif path.endswith('chapter.html'):
                route.fulfill(content_type='text/html', body=markup)
            elif path == '/katex/katex.min.css':
                route.fulfill(content_type='text/css', body='')
            else:
                route.fulfill(path=str(REVIEW / ('viewer.html' if path == '/' else Path(path).name)))

        with sync_playwright() as p:
            browser = p.chromium.launch(channel='msedge')
            page = browser.new_page(viewport={'width': 1600, 'height': 950})
            page.add_init_script('window.EventSource=class {constructor(){}}')
            page.route('**/*', respond)
            page.goto('http://book-review.test/')
            page.wait_for_function('state && !loading')
            previous, next_button = page.locator('#previous'), page.locator('#next')
            if check_bounds:
                self.assertFalse(previous.is_enabled())
            self.assertTrue(next_button.is_enabled())

            def check_position(accepted_id, counterpart_id):
                page.evaluate('()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))')
                offsets = page.evaluate('''([acceptedId,counterpartId]) => {
                    return Object.fromEntries(Object.entries(panes).map(([name,pane])=>{
                        const id=name === 'accepted' ? acceptedId : counterpartId;
                        const node=visibleNodes(name,id)[0].node;
                        return [name,node.getBoundingClientRect().top-pane.getBoundingClientRect().top-.25*pane.clientHeight];
                    }));
                }''', [accepted_id, counterpart_id])
                for name, offset in offsets.items():
                    self.assertAlmostEqual(offset, 0, delta=2, msg=f'{name}: {offsets}')

            next_button.click()
            check_position('p8', 'p9')
            if check_bounds:
                self.assertFalse(previous.is_enabled())
            next_button.click()
            check_position('p12', 'p12')
            self.assertTrue(previous.is_enabled())
            next_button.click()
            check_position('p20', 'p21')
            if check_bounds:
                self.assertFalse(next_button.is_enabled())
            page.evaluate('nextChange(1)')
            self.assertEqual(page.evaluate('selected'), 'p20')
            previous.click()
            check_position('p12', 'p12')
            previous.click()
            check_position('p8', 'p9')
            if check_bounds:
                self.assertFalse(previous.is_enabled())
            page.evaluate('nextChange(-1)')
            self.assertEqual(page.evaluate('selected'), 'p8')
            status['changes'] = []
            page.evaluate('refresh()')
            page.wait_for_function('!loading && !state.changes.length')
            if check_bounds:
                self.assertFalse(previous.is_enabled())
            if check_bounds:
                self.assertFalse(next_button.is_enabled())
            error = page.evaluate('''async () => {
                try {await post('/api/unsupported',{});return null;}
                catch(error) {return error.message;}
            }''')
            self.assertIn('Restart Book Review server', error)
            self.assertNotIn('Unexpected token', error)
            browser.close()

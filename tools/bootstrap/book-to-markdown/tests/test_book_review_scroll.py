"""Keep logical reading position when visibility edits rebuild chapter HTML."""
from copy import deepcopy
import json
from io import BytesIO
from pathlib import Path
import sys
import unittest
from urllib.parse import urlsplit

from PIL import Image
from browser_session import browser_page

REVIEW = Path(__file__).resolve().parents[4] / 'tools/bootstrap/book-to-markdown/review'
sys.path.insert(0, str(REVIEW.parent))
from review.model import EDITABLE_TYPES, ELEMENT_ROLES, PROTECTED_TYPES


class ScrollReviewTest(unittest.TestCase):
    def test_hide_above_reading_position(self):
        self.check_hide('p11', 'p12')

    def test_hide_reading_anchor_uses_next_visible_element(self):
        self.check_hide('p12', 'p13')

    def test_hide_pdf_page_keeps_following_content_aligned(self):
        self.check_hide('p12', 'p13', 'pdf')

    def check_hide(self, hidden_id, expected_id, pane='working'):
        elements = [{'id': f'p{i}', 'type': 'prose', 'pdf_regions': [{'page': i+1, 'bbox': [0, 0, 200, 100]}], 'role': None} for i in range(30)]
        accepted = {'revision': 'before', 'toc': {'depth': 2}, 'chapters': [
            {'id': 'chapter', 'title': 'Example', 'elements': elements}],
            'render': {'chapters': [{'id': 'chapter', 'document': 'chapter.html'}]}}
        working = deepcopy(accepted)
        status = {'working': working, 'accepted': accepted, 'can_accept': True,
                  'authored_revision': 'before', 'pending': False, 'error': None,
                  'editable_types': EDITABLE_TYPES, 'protected_types': PROTECTED_TYPES, 'roles': ELEMENT_ROLES,
                  'changes': [], 'heading_types': [], 'pdf_pages': [
                      {'page': i+1, 'width': 200, 'height': 100, 'image': f'pdf/page_{i+1}.png'} for i in range(30)]}
        image=BytesIO()
        Image.new('RGB',(200,100),'white').save(image,format='PNG')
        markup = ''.join(f'<div data-element="p{i}"><div style="height:120px">Paragraph {i}</div></div>'
                         for i in range(30))

        def respond(route):
            path = urlsplit(route.request.url).path
            if path == '/api/status':
                route.fulfill(content_type='application/json', body=json.dumps(status))
            elif path == '/api/visibility':
                command = route.request.post_data_json
                next(e for e in working['chapters'][0]['elements'] if e['id'] == command['id'])['visible'] = False
                working['revision'] = status['authored_revision'] = 'after'
                status['changes'] = [{'id': command['id']}]
                route.fulfill(content_type='application/json', body='{"ok":true}')
            elif path.endswith('.png'):
                route.fulfill(content_type='image/png',body=image.getvalue())
            elif path.endswith('chapter.html'):
                route.fulfill(content_type='text/html', body=markup)
            elif path == '/katex/katex.min.css':
                route.fulfill(content_type='text/css', body='')
            else:
                route.fulfill(path=str(REVIEW / ('viewer.html' if path == '/' else Path(path).name)))

        with browser_page(viewport={'width': 1600, 'height': 950}) as page:
            page.add_init_script('window.EventSource=class {constructor(){}}')
            page.route('**/*', respond)
            page.goto('http://book-review.test/')
            page.wait_for_function('state && !loading')
            page.evaluate('pane=>{leader=pane;position(pane,{id:"p12",progress:0.2});sync(pane);}',pane)
            page.evaluate('()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))')
            target = f'#pdf [data-id="{hidden_id}"]' if pane == 'pdf' else f'#working [data-element="{hidden_id}"]'
            page.locator(target).click(button='right')
            page.locator('#menu-visibility').click()
            page.wait_for_function('state.working.revision === "after" && !loading')
            page.evaluate('()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))')
            actual = page.evaluate('''id => {
                const pane=panes.working, node=nodes.working.find(e=>e.id===id).node;
                return {offset:node.getBoundingClientRect().top-pane.getBoundingClientRect().top,
                        reference:.25*pane.clientHeight,anchor:anchor("working"),leader};
            }''', expected_id)
            expected_offset=actual['reference'] - (24 if hidden_id != 'p12' else 0)
            self.assertAlmostEqual(actual['offset'], expected_offset, delta=2, msg=str(actual))
            self.assertEqual(actual['leader'], pane)
            accepted_offset=page.locator(f'#accepted [data-element="{expected_id}"]').evaluate(
                '(n)=>n.getBoundingClientRect().top-document.getElementById("accepted").getBoundingClientRect().top')
            self.assertAlmostEqual(accepted_offset, expected_offset, delta=2)
            self.assertFalse(page.locator(f'#working [data-element="{hidden_id}"]').is_visible())

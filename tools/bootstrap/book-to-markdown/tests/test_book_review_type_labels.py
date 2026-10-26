"""Show Types renders type and role as separate, space-aware PDF labels."""

from copy import deepcopy
from io import BytesIO
import json
from pathlib import Path
import unittest
from urllib.parse import urlsplit

from PIL import Image
from browser_session import browser_page


REVIEW = Path(__file__).resolve().parents[4] / 'tools/bootstrap/book-to-markdown/review'


class TypeRoleLabelTest(unittest.TestCase):
    def test_role_sits_beside_or_below_type_according_to_available_space(self):
        elements = [
            {'id': 'heading', 'type': 'heading', 'heading_level': 2, 'role': 'caption',
             'pdf_regions': [{'page': 1, 'bbox': [40, 30, 200, 55]}]},
            {'id': 'prose', 'type': 'prose', 'role': 'footnote',
             'pdf_regions': [{'page': 1, 'bbox': [40, 100, 515, 200]}]},
            {'id': 'table', 'type': 'table', 'role': 'legend',
             'pdf_regions': [{'page': 1, 'bbox': [40, 240, 555, 340]}]},
        ]
        snapshot = {'revision': 'same', 'toc': {'depth': 2}, 'chapters': [
            {'id': 'chapter', 'title': 'Example', 'elements': elements}],
            'render': {'chapters': [{'id': 'chapter', 'document': 'chapter.html'}]}}
        status = {'working': snapshot, 'accepted': deepcopy(snapshot), 'can_accept': False,
                  'authored_revision': 'same', 'pending': False, 'error': None,
                  'changes': [], 'heading_types': [],
                  'pdf_pages': [{'page': 1, 'width': 600, 'height': 800,
                                 'image': 'pdf/page_0001.png'}]}
        picture = BytesIO()
        Image.new('RGB', (600, 800), 'white').save(picture, format='PNG')

        def respond(route):
            path = urlsplit(route.request.url).path
            if path == '/api/status':
                route.fulfill(content_type='application/json', body=json.dumps(status))
            elif path.endswith('.png'):
                route.fulfill(content_type='image/png', body=picture.getvalue())
            elif path.endswith('chapter.html'):
                route.fulfill(content_type='text/html', body=''.join(
                    f'<div data-element="{item["id"]}">{item["id"]}</div>' for item in elements))
            elif path == '/katex/katex.min.css':
                route.fulfill(content_type='text/css', body='')
            else:
                route.fulfill(path=str(REVIEW / ('viewer.html' if path == '/' else Path(path).name)))

        with browser_page(viewport={'width': 1600, 'height': 950}) as page:
            page.add_init_script('window.EventSource=class {constructor(){}}')
            page.route('**/*', respond)
            page.goto('http://book-review.test/')
            page.wait_for_function('state && !loading && nodes.pdf.length === 3')
            page.locator('#show-types').check()
            labels = page.evaluate('''() => Object.fromEntries(nodes.pdf.map(({id,node}) => {
                const type=getComputedStyle(node,'::before');
                const role=getComputedStyle(node,'::after');
                return [id,{classes:node.className,type:type.content,role:role.content,
                            typeColor:type.backgroundColor,roleColor:role.backgroundColor}];
            }))''')
            self.assertIn('role-label-inline', labels['heading']['classes'])
            self.assertIn('role-label-under-type', labels['prose']['classes'])
            self.assertIn('role-label-below', labels['table']['classes'])
            self.assertEqual(labels['heading']['type'], '"heading 2"')
            self.assertEqual(labels['heading']['role'], '"caption"')
            self.assertEqual(labels['prose']['role'], '"footnote"')
            self.assertEqual(labels['table']['role'], '"legend"')
            self.assertNotEqual(labels['heading']['typeColor'], labels['heading']['roleColor'])


if __name__ == '__main__':
    unittest.main()

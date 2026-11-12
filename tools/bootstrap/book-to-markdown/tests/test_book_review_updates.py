"""Keep large-book interactions local while preserving reading geometry."""

from contextlib import contextmanager
from copy import deepcopy
from hashlib import sha256
from io import BytesIO
import json
from pathlib import Path
import unittest
from urllib.parse import urlsplit

from PIL import Image
from browser_session import browser_page


REVIEW = Path(__file__).resolve().parents[4] / 'tools/bootstrap/book-to-markdown/review'
VIEWER = REVIEW / 'viewer.js'
FRAMES = '() => new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))'


@contextmanager
def large_book(*, hashed_resources=False):
    chapters = [
        {'id': f'c{chapter}', 'title': f'Chapter {chapter}', 'elements': [
            {'id': f'p{i}', 'type': 'prose', 'role': None,
             'pdf_regions': [{'page': i+1, 'bbox': [0, 0, 200, 100]}]}
            for i in range(chapter*200, (chapter+1)*200)]}
        for chapter in range(2)]
    snapshot = {'revision': 'before', 'toc': {'depth': 2}, 'chapters': chapters,
                'render': {'chapters': [
                    {'id': f'c{i}', 'key': f'c{i}-before',
                     'document': f'renders/before/c{i}.html'} for i in range(2)]}}
    status = {'sequence': 1, 'working': deepcopy(snapshot), 'accepted': deepcopy(snapshot),
              'authored_revision': 'before', 'can_accept': True, 'can_discard': False,
              'pending': False, 'error': None, 'heading_types': [], 'changes': [],
              'pdf_pages': [{'page': i+1, 'width': 200, 'height': 100,
                             'image': f'pdf/page_{i+1}.png'} for i in range(400)]}
    image = BytesIO()
    Image.new('RGB', (200, 100), 'white').save(image, format='PNG')
    replacement_image = BytesIO()
    Image.new('RGB', (300, 100), 'white').save(replacement_image, format='PNG')
    if hashed_resources:
        for version in ('accepted', 'working'):
            status[version]['asset_hashes'] = {'assets/example.png': sha256(image.getvalue()).hexdigest()}
            status[version]['settings'] = {'style': sha256((REVIEW / 'document.css').read_bytes()).hexdigest()}
    fetched = []

    def respond(route):
        path = urlsplit(route.request.url).path
        if path == '/api/status':
            fetched.append(path)
            route.fulfill(content_type='application/json', body=json.dumps(status))
        elif path.endswith('.png'):
            if '/assets/' in path:
                fetched.append(path)
            data = replacement_image if '/after-assets/' in path else image
            route.fulfill(content_type='image/png', body=data.getvalue())
        elif path.endswith('.html'):
            fetched.append(path)
            chapter = int(Path(path).stem[1:])
            bodies = []
            for i in range(chapter*200, (chapter+1)*200):
                text = 'Changed paragraph' if i == 12 and '/after/' in path else f'Paragraph {i}'
                body = f'<p style="height:120px">{text}</p>'
                if i == 10:
                    body += ('<details><summary>Source</summary><div style="height:400px">'
                             '<img src="assets/example.png"><a href="#local">Local link</a>'
                             '<p id="local">Local target</p></div></details>')
                bodies.append(f'<div data-element="p{i}">{body}</div>')
            route.fulfill(content_type='text/html', body=''.join(bodies))
        elif path == '/katex/katex.min.css':
            route.fulfill(content_type='text/css', body='')
        else:
            if path.endswith('document.css'):
                fetched.append(path)
            source = VIEWER if path == '/viewer.js' else REVIEW / (
                'viewer.html' if path == '/' else Path(path).name)
            route.fulfill(path=str(source))

    with browser_page(viewport={'width': 1600, 'height': 950}) as page:
        page.add_init_script('window.EventSource=class {constructor(){window.reviewEvents=this;}}')
        page.route('**/*', respond)
        page.goto('http://book-review.test/')
        page.wait_for_function('state && !loading && nodes.working.length === 400')
        page.evaluate(FRAMES)
        yield page, status, fetched


class IncrementalReviewTest(unittest.TestCase):
    def test_unchanged_reading_labels_and_hover_leave_toolbar_untouched(self):
        with large_book() as (page, _, _):
            page.evaluate('''() => {
              window.toolbarMutations=[];
              const observer=new MutationObserver(records=>toolbarMutations.push(...records.map(record=>record.target.id)));
              for(const id of ['pdf-location','accepted-location','working-location','previous','next'])
                observer.observe(document.getElementById(id),{attributes:true,childList:true});
              scheduleLocations();
            }''')
            page.evaluate(FRAMES)
            page.locator('#working [data-element="p0"]').hover()
            page.locator('#working [data-element="p1"]').hover()
            page.evaluate(FRAMES)
            self.assertEqual(page.evaluate('toolbarMutations'), [])

    def test_event_heartbeat_does_not_download_unchanged_book(self):
        with large_book() as (page, status, fetched):
            requests = fetched.count('/api/status')
            page.evaluate('''() => {
              for(let i=0;i<5;i++)reviewEvents.onmessage({data:String(state.sequence)});
            }''')
            page.evaluate(FRAMES)
            self.assertEqual(fetched.count('/api/status'), requests)
            status['sequence'] = 2
            page.evaluate('reviewEvents.onmessage({data:"2"})')
            page.wait_for_function('state.sequence === 2 && !loading')
            self.assertEqual(fetched.count('/api/status'), requests+1)

    def test_accepted_resources_are_reused_until_their_content_changes(self):
        with large_book(hashed_resources=True) as (page, status, fetched):
            original_source = page.locator('#accepted [data-element="p10"] img').get_attribute('src')
            self.assertTrue(original_source.startswith('blob:'))
            self.assertEqual(sum(path.endswith('example.png') for path in fetched), 1)
            self.assertEqual(sum(path.endswith('document.css') for path in fetched), 1)
            status['accepted']['revision'] = 'after'
            for layout in status['accepted']['render']['chapters']:
                layout['document'] = layout['document'].replace('/before/', '/after/')
            page.evaluate('refresh()')
            self.assertEqual(original_source, page.locator('#accepted [data-element="p10"] img').get_attribute('src'))
            self.assertEqual(sum(path.endswith('example.png') for path in fetched), 1)
            self.assertEqual(sum(path.endswith('document.css') for path in fetched), 1)

            image = BytesIO()
            Image.new('RGB', (300, 100), 'white').save(image, format='PNG')
            working = status['working']
            working['asset_hashes']['assets/example.png'] = sha256(image.getvalue()).hexdigest()
            working['revision'] = status['authored_revision'] = 'after-assets'
            working['render']['chapters'][0]['key'] = 'c0-after-assets'
            for layout in working['render']['chapters']:
                layout['document'] = layout['document'].replace('/before/', '/after-assets/')
            page.evaluate('refresh()')
            self.assertEqual(page.locator('#working [data-element="p10"] img').evaluate('(node)=>node.naturalWidth'), 300)
            self.assertNotEqual(original_source, page.locator('#working [data-element="p10"] img').get_attribute('src'))
            self.assertEqual(sum(path.endswith('example.png') for path in fetched), 2)

            page.evaluate('''() => {
              const revoke=URL.revokeObjectURL;window.released=[];
              URL.revokeObjectURL=url=>{released.push(url);revoke(url);};
            }''')
            status['accepted'] = deepcopy(working)
            page.evaluate('refresh()')
            self.assertIn(original_source, page.evaluate('released'))
            self.assertEqual(page.locator('#accepted [data-element="p10"] img').evaluate('(node)=>node.naturalWidth'), 300)
            self.assertEqual(sum(path.endswith('example.png') for path in fetched), 2)

    def test_hover_only_changes_matching_blocks(self):
        with large_book() as (page, _, _):
            page.evaluate('''() => {
              window.mutations=[];
              window.observer=new MutationObserver(records=>mutations.push(...records.map(record=>
                record.target.dataset.element || record.target.dataset.id)));
              for(const entries of Object.values(nodes))for(const entry of entries)
                observer.observe(entry.node,{attributes:true});
            }''')
            page.locator('#working [data-element="p0"]').hover()
            page.locator('#working [data-element="p1"]').hover()
            changed = page.evaluate('mutations')
            self.assertTrue(changed)
            self.assertLessEqual(set(changed), {'p0', 'p1'})
            for pane, attribute in [('working', 'data-element'), ('accepted', 'data-element'), ('pdf', 'data-id')]:
                self.assertIn('hover', page.locator(f'#{pane} [{attribute}="p1"]').get_attribute('class'))

    def test_edit_and_accept_retain_unchanged_content_and_rebase_resources(self):
        with large_book() as (page, status, fetched):
            page.evaluate('''() => {
              window.original=Object.fromEntries(['accepted','working'].map(version=>[version,{
                chapter:panes[version].children[1],
                nodes:new Map(nodes[version].map(entry=>[entry.id,entry.node]))}]));
              original.working.nodes.get('p10').querySelector('details').open=true;
            }''')
            page.evaluate(FRAMES)
            working = status['working']
            working['revision'] = status['authored_revision'] = 'after'
            working['render']['chapters'][0]['key'] = 'c0-after'
            for layout in working['render']['chapters']:
                layout['document'] = layout['document'].replace('/before/', '/after/')
            status['changes'] = [{'id': 'p12'}]
            page.evaluate('refresh()')
            self.assertEqual(page.locator('#working [data-element="p12"] p').inner_text(), 'Changed paragraph')
            self.assertTrue(page.evaluate('''() => original.working.nodes.get('p10') ===
              nodes.working.find(entry=>entry.id === 'p10').node && original.working.nodes.get('p11') ===
              nodes.working.find(entry=>entry.id === 'p11').node && original.working.chapter === panes.working.children[1]'''))
            self.assertTrue(page.locator('#working [data-element="p10"] details').evaluate('(node)=>node.open'))
            self.assertIn('/renders/after/assets/', page.locator('#working [data-element="p10"] img').get_attribute('src'))
            self.assertNotIn('/files/working/renders/after/c1.html', fetched)
            page.evaluate('''() => {
              const originalScroll=Element.prototype.scrollIntoView;
              Element.prototype.scrollIntoView=function(...args){window.localScrolled=this.id;return originalScroll.apply(this,args);};
            }''')
            page.locator('#working [data-element="p10"] a').evaluate('(link)=>link.click()')
            self.assertEqual(page.evaluate('localScrolled'), 'local')

            status['accepted'] = deepcopy(working)
            status['changes'] = []
            page.evaluate('refresh()')
            self.assertTrue(page.evaluate('''() => original.accepted.chapter === panes.accepted.children[1] &&
              original.accepted.nodes.get('p11') === nodes.accepted.find(entry=>entry.id === 'p11').node'''))
            self.assertEqual(page.locator('#accepted [data-element="p12"] p').inner_text(), 'Changed paragraph')
            self.assertIn('/renders/after/assets/', page.locator('#accepted [data-element="p10"] img').get_attribute('src'))
            self.assertNotIn('/files/accepted/renders/after/c1.html', fetched)
            self.assertFalse(page.locator('#working [data-element="p12"]').evaluate('(node)=>node.classList.contains("changed")'))

    def test_reading_anchor_matches_document_geometry_after_layout_changes(self):
        with large_book() as (page, _, _):
            page.evaluate('''() => {
              window.referenceAnchor=pane=>{
                const container=panes[pane], line=container.getBoundingClientRect().top+.25*container.clientHeight;
                let entries=nodes[pane].filter(entry=>entry.node.getClientRects().length &&
                  getComputedStyle(entry.node).visibility !== 'hidden');
                if(pane === 'pdf') {
                  const horizontal=entries.filter(entry=>{const rect=entry.node.getBoundingClientRect();
                    return pointerX>=rect.left && pointerX<=rect.right;});
                  if(horizontal.length)entries=horizontal;
                }
                const distance=entry=>{const rect=entry.node.getBoundingClientRect();return Math.max(rect.top-line,line-rect.bottom,0);};
                entries.sort((a,b)=>distance(a)-distance(b) ||
                  Number(a.source?.version !== 'working')-Number(b.source?.version !== 'working'));
                const entry=entries[0];if(!entry)return null;
                const rect=entry.node.getBoundingClientRect();
                return {id:entry.id,progress:Math.max(0,Math.min(1,(line-rect.top)/Math.max(1,rect.height)))};
              };
            }''')

            def check():
                page.evaluate(FRAMES)
                results = page.evaluate('''() => {
                  const results=[];
                  pointerX=panes.pdf.getBoundingClientRect().left+50;
                  for(const name of Object.keys(panes))for(const top of [0,1500,28000,55000]) {
                    panes[name].scrollTop=top;
                    results.push({pane:name,expected:referenceAnchor(name),actual:anchor(name)});
                  }
                  return results;
                }''')
                for result in results:
                    self.assertEqual(result['actual']['id'], result['expected']['id'], str(result))
                    self.assertAlmostEqual(result['actual']['progress'], result['expected']['progress'], places=5)

            check()
            page.locator('#working [data-element="p10"] details').evaluate('(node)=>node.open=true')
            check()
            page.evaluate('collapsed.add("c0");applyCollapse("c0")')
            check()
            page.evaluate('collapsed.delete("c0");applyCollapse("c0")')
            page.set_viewport_size({'width': 1300, 'height': 800})
            check()

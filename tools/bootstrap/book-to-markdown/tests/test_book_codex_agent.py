"""A Codex task can inspect and edit Working beyond one selected element."""

from pathlib import Path
from http.server import ThreadingHTTPServer
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from playwright.sync_api import sync_playwright
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.model import load_input, read_json, write_json
from review.service import Handler, ReviewState
from review.codex_agent import run_agent


class WorkingAgentTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        book = {'toc': {'depth': 2}, 'chapters': [
            {'id': 'chapter', 'title': 'Example', 'markdown_file': 'example.md',
             'elements': [
                 {'id': 'first', 'type': 'prose', 'role': None, 'visible': True,
                  'pdf_regions': [{'page': 1, 'bbox': [0, 0, 10, 10]}], 'md_text': 'First part'},
                 {'id': 'second', 'type': 'prose', 'role': None, 'visible': True,
                  'pdf_regions': [{'page': 2, 'bbox': [0, 0, 10, 10]}], 'md_text': 'continues.'},
             ]}]}
        write_json(self.root / 'working/book.json', book)
        write_json(self.root / 'accepted/book.json', book)
        self.state = ReviewState(self.root)
        self.revision = load_input(self.root / 'working')[2]
        patcher = patch('review.codex_agent.REPOSITORY', self.root)
        patcher.start()
        self.addCleanup(patcher.stop)

    def client(self, actions, callback=None, seen_images=None):
        owner = self
        class Client:
            def __init__(self, config, **kwargs):
                owner.assertEqual(kwargs['stage'], 'book_codex_agent')
            def __enter__(self):
                return self
            def __exit__(self, *args):
                pass
            def generate_json(self, prompt, image_path=None, validator=None):
                owner.assertIn('Working', prompt)
                if seen_images is not None:
                    seen_images.append(image_path)
                action = actions.pop(0)
                if callback:
                    callback()
                if validator:
                    validator(action)
                return action
        return Client

    def test_agent_requests_context_and_merges_into_working(self):
        accepted = (self.root / 'accepted/book.json').read_bytes()
        actions = [
            {'action': 'list', 'cursor': 0, 'limit': 10},
            {'action': 'read', 'ids': ['first', 'second']},
            {'action': 'editor', 'command': 'merge', 'args':
             {'id': 'first', 'next_id': 'second', 'text': 'First part continues.'}},
            {'action': 'finish', 'summary': 'Merged the continuation.'},
        ]
        result = run_agent(self.state, {'instruction': 'Join split paragraphs throughout Working',
                                        'expected_revision': self.revision},
                           client_factory=self.client(actions))
        self.assertEqual(result['summary'], 'Merged the continuation.')
        self.assertEqual(len(load_input(self.root / 'working')[0]['chapters'][0]['elements']), 1)
        self.assertEqual((self.root / 'accepted/book.json').read_bytes(), accepted)
        self.assertFalse(actions)

    def test_editor_merge_resolves_source_references_from_agent_text(self):
        references = ('{{text page=1 block=0 chars=0:10}}',
                      '{{text page=2 block=0 chars=0:10}}')
        for page, content in ((1, 'First part'), (2, 'continues.')):
            write_json(self.root / 'pdf-to-markdown/02_preprocess' / f'page_{page:04d}.json',
                       {'blocks': [{'block_id': 0, 'text': content}]})
        for bundle in ('working', 'accepted'):
            path = self.root / bundle / 'book.json'
            book = read_json(path)
            for element, reference in zip(book['chapters'][0]['elements'], references):
                element['md_text'] = reference
            write_json(path, book)
        self.revision = load_input(self.root / 'working')[2]
        actions = [
            {'action': 'read', 'ids': ['first', 'second']},
            {'action': 'editor', 'command': 'merge', 'args':
             {'id': 'first', 'next_id': 'second', 'text': references[0] + ' ' + references[1]}},
            {'action': 'finish', 'summary': 'Merged the continuation.'},
        ]
        run_agent(self.state, {'instruction': 'Join the page boundary paragraph',
                               'expected_revision': self.revision}, client_factory=self.client(actions))
        merged = load_input(self.root / 'working')[0]['chapters'][0]['elements'][0]
        self.assertEqual(merged['resolved_md_text'], 'First part continues.')
        self.assertEqual(merged['md_text'], references[0] + ' ' + references[1])
        self.assertEqual(len(load_input(self.root / 'working')[0]['chapters'][0]['elements']), 1)

    def test_agent_can_change_authored_text_without_element_content_guard(self):
        actions = [
            {'action': 'patch', 'operations': [
                {'op': 'replace', 'path': '/chapters/0/elements/0/md_text', 'value': 'Corrected part'}]},
            {'action': 'finish', 'summary': 'Corrected the text.'},
        ]
        run_agent(self.state, {'instruction': 'Correct the first phrase in Working',
                               'expected_revision': self.revision}, client_factory=self.client(actions))
        first = load_input(self.root / 'working')[0]['chapters'][0]['elements'][0]
        self.assertEqual(first['resolved_md_text'], 'Corrected part')
        self.assertEqual(load_input(self.root / 'accepted')[0]['chapters'][0]['elements'][0]['resolved_md_text'],
                         'First part')

    def test_external_edit_during_inference_rejects_agent_result(self):
        def external_edit():
            self.state.markdown({'id': 'second', 'text': 'Changed outside.',
                                 'expected_revision': self.revision})
        before = (self.root / 'accepted/book.json').read_bytes()
        with self.assertRaisesRegex(ValueError, 'Working changed while Codex was thinking'):
            run_agent(self.state, {'instruction': 'Edit Working', 'expected_revision': self.revision},
                      client_factory=self.client([{'action': 'finish', 'summary': 'Done'}], external_edit))
        self.assertEqual((self.root / 'accepted/book.json').read_bytes(), before)

    def test_agent_requests_pdf_image_on_next_turn(self):
        pages = self.root / 'pdf/pages'
        pages.mkdir(parents=True)
        Image.new('RGB', (20, 20), 'white').save(pages / 'page_0001.png')
        self.state = ReviewState(self.root)
        seen = []
        run_agent(self.state, {'instruction': 'Inspect page 1', 'expected_revision': self.revision},
                  client_factory=self.client([{'action': 'image', 'page': 1},
                                              {'action': 'finish', 'summary': 'Inspected page.'}],
                                             seen_images=seen))
        self.assertEqual(seen, [None, pages / 'page_0001.png'])

    def test_invalid_patch_leaves_working_unchanged(self):
        before = (self.root / 'working/book.json').read_bytes()
        actions = [{'action': 'patch', 'operations': [
            {'op': 'replace', 'path': '/chapters/0/elements/0/type', 'value': 'impossible'}]},
            {'action': 'finish', 'summary': 'Could not apply the invalid edit.'}]
        run_agent(self.state, {'instruction': 'Try an edit', 'expected_revision': self.revision},
                  client_factory=self.client(actions))
        self.assertEqual((self.root / 'working/book.json').read_bytes(), before)

    def test_stop_during_inference_discards_next_action(self):
        stopped = threading.Event()
        before = (self.root / 'working/book.json').read_bytes()
        action = {'action': 'patch', 'operations': [
            {'op': 'replace', 'path': '/chapters/0/elements/0/md_text', 'value': 'Too late'}]}
        with self.assertRaisesRegex(RuntimeError, 'stopped before applying'):
            run_agent(self.state, {'instruction': 'Edit Working', 'expected_revision': self.revision},
                      client_factory=self.client([action], stopped.set), cancelled=stopped)
        self.assertEqual((self.root / 'working/book.json').read_bytes(), before)

    def test_background_job_reports_progress_and_completion(self):
        actions = [{'action': 'list', 'cursor': 0}, {'action': 'finish', 'summary': 'Inspected Working.'}]
        with patch('review.codex_agent.CodexClient', self.client(actions)):
            started = self.state.start_codex_agent({'instruction': 'Inspect Working',
                                                    'expected_revision': self.revision})
            deadline = time.monotonic() + 5
            while self.state.status()['codex_agent']['state'] == 'running' and time.monotonic() < deadline:
                time.sleep(0.01)
            job = self.state.status()['codex_agent']
        self.assertEqual(job['id'], started['id'])
        self.assertEqual(job['state'], 'completed')
        self.assertEqual(job['summary'], 'Inspected Working.')

    def test_browser_starts_general_command_without_element_selection(self):
        actions = [
            {'action': 'patch', 'operations': [
                {'op': 'replace', 'path': '/chapters/0/elements/0/md_text', 'value': 'Edited by Codex'}]},
            {'action': 'finish', 'summary': 'Edited Working.'},
        ]
        server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        server.daemon_threads = True
        server.state = self.state
        threading.Thread(target=server.serve_forever, daemon=True).start()
        rendering = threading.Thread(target=self.state.render_loop, daemon=True)
        rendering.start()
        try:
            with patch('review.codex_agent.CodexClient', self.client(actions)):
                with sync_playwright() as playwright:
                    browser = playwright.chromium.launch(channel='msedge')
                    page = browser.new_page(viewport={'width': 1500, 'height': 900})
                    errors = []
                    page.on('pageerror', lambda error: errors.append(str(error)))
                    page.goto(f'http://127.0.0.1:{server.server_port}')
                    page.wait_for_function('state?.can_accept && !loading')
                    page.locator('#codex-working').click()
                    page.locator('#codex-preset').select_option(label='Join paragraphs across PDF pages')
                    self.assertEqual(page.locator('#codex-instruction').input_value(),
                                     'Find places where a paragraph continues across a PDF page boundary, '
                                     'and merge the split paragraphs.')
                    page.locator('#codex-instruction').fill('Edit the first phrase in Working')
                    self.assertEqual(page.locator('#codex-preset').input_value(), '')
                    page.locator('#run-codex').click()
                    page.wait_for_function('state?.codex_agent?.state === "completed" && '
                                           '!document.getElementById("codex-dialog").open')
                    self.assertEqual(load_input(self.root / 'working')[0]['chapters'][0]['elements'][0]['resolved_md_text'],
                                     'Edited by Codex')
                    self.assertEqual(load_input(self.root / 'accepted')[0]['chapters'][0]['elements'][0]['resolved_md_text'],
                                     'First part')
                    page.locator('#working [data-element="first"]').click(button='right')
                    page.locator('#menu-codex-working').click()
                    self.assertIn('Starting at first', page.locator('#codex-id').inner_text())
                    self.assertIn('Edited Working.', page.locator('#codex-status').inner_text())
                    self.assertTrue(page.locator('#codex-dialog').evaluate('(dialog) => dialog.open'))
                    page.locator('#cancel-codex').click()
                    page.wait_for_function('!loading && !state.pending')
                    page.locator('#working [data-element="first"]').click(button='right')
                    page.locator('#menu-codex').click()
                    self.assertTrue(page.locator('#codex-preset-section').is_hidden())
                    self.assertFalse(errors)
                    browser.close()
        finally:
            self.state.stopping.set()
            with self.state.condition:
                self.state.condition.notify_all()
            server.shutdown()
            server.server_close()
            rendering.join(20)


if __name__ == '__main__':
    unittest.main()

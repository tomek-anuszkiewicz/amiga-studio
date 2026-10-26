"""A Codex task can inspect and edit Working beyond one selected element."""

import json
from pathlib import Path
from http.server import ThreadingHTTPServer
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from browser_session import browser_page
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.model import load_input, read_json, write_json
from review.service import Handler, ReviewState
from review.codex_agent import replace_source_text, run_agent, source_search
from review.source_cleanup import preview


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

    def test_source_audit_and_search_support_literal_cleanup(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'LINE DRAW: BLTADAT is used.'}]})
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0002.json',
                   {'blocks': [{'block_id': 0, 'text': 'LINE MODE (line draw)'}]})
        nearby = source_search(self.root / 'pdf-to-markdown',
                               {'query': 'LINE DRAW', 'pages': [2]})
        self.assertFalse(nearby['matches'][0]['exact_case'])
        self.assertIn('search without pages', nearby['hint'])
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = 'LINE DRAW:.'
        write_json(self.root / 'working/book.json', book)
        self.revision = load_input(self.root / 'working')[2]
        accepted = (self.root / 'accepted/book.json').read_bytes()
        actions = [
            {'action': 'source_audit', 'cursor': 0, 'limit': 20},
            {'action': 'read', 'ids': ['first']},
            {'action': 'source_search', 'query': 'LINE DRAW', 'pages': [1]},
            {'action': 'patch', 'operations': [{'op': 'replace',
             'path': '/chapters/0/elements/0/md_text', 'value': 'LINE MODE (line draw)'}]},
            {'action': 'source_replace', 'path': '/chapters/0/elements/0/md_text',
             'template': '{{text page=2 block=0 chars=0:21}}'},
            {'action': 'source_search', 'query': 'LINE DRAW:', 'pages': [1]},
            {'action': 'source_replace', 'path': '/chapters/0/elements/0/md_text',
             'template': '{{text page=1 block=0 chars=0:10}}'},
            {'action': 'source_audit', 'cursor': 0, 'limit': 20},
            {'action': 'finish', 'summary': 'Replaced the literal and removed the extra period.'},
        ]
        observed = []
        owner = self

        class Client:
            def __init__(self, config, **kwargs):
                pass
            def __enter__(self):
                return self
            def __exit__(self, *args):
                pass
            def generate_json(self, prompt, **kwargs):
                observed.append(prompt)
                return actions.pop(0)

        result = run_agent(owner.state, {'instruction': 'Clean direct book text',
                                         'task_kind': 'source_cleanup',
                                         'expected_revision': owner.revision}, client_factory=Client)
        self.assertEqual(result['edits'], 1)
        self.assertIn('"literals": ["LINE", "DRAW:."]', observed[1])
        self.assertIn('"md_text": "LINE DRAW:."', observed[2])
        self.assertIn('"chars": [0, 9]', observed[3])
        self.assertIn('"reference": "{{text page=1 block=0 chars=0:9}}"', observed[3])
        self.assertIn('Use source_replace', observed[4])
        self.assertIn('Codex changed content', observed[5])
        initial_audit = json.loads(observed[1].split('TASK STATE JSON:\n', 1)[1])['last_result']
        final_audit = json.loads(observed[8].split('TASK STATE JSON:\n', 1)[1])['last_result']
        self.assertGreater(initial_audit['total'], final_audit['total'])
        self.assertFalse(any(field['element_id'] == 'first' for field in final_audit['fields']))
        first = load_input(self.root / 'working')[0]['chapters'][0]['elements'][0]
        self.assertEqual(first['md_text'], '{{text page=1 block=0 chars=0:10}}')
        self.assertEqual(first['resolved_md_text'], 'LINE DRAW:')
        self.assertEqual((self.root / 'accepted/book.json').read_bytes(), accepted)
        self.assertFalse(actions)

    def test_deterministic_cleanup_previews_before_selected_working_change(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'LINE DRAW: BLTADAT is used.'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['pdf_regions'][0]['page'] = 2
        book['chapters'][0]['elements'][0]['md_text'] = 'LINE DRAW:.'
        write_json(self.root / 'working/book.json', book)
        self.revision = load_input(self.root / 'working')[2]
        accepted = (self.root / 'accepted/book.json').read_bytes()
        working = (self.root / 'working/book.json').read_bytes()
        report = self.state.preview_source_cleanup({'expected_revision': self.revision})
        match = next(item for item in report['findings'] if item['element_id'] == 'first')
        self.assertEqual((match['status'], match['page'], match['source_text']),
                         ('matched', 1, 'LINE DRAW:'))
        self.assertEqual((self.root / 'working/book.json').read_bytes(), working)
        self.assertEqual((self.root / 'accepted/book.json').read_bytes(), accepted)
        key = match['id']
        self.state.apply_source_cleanup({'expected_revision': self.revision, 'selections': [key]})
        updated = load_input(self.root / 'working')[0]['chapters'][0]['elements'][0]
        self.assertEqual(updated['md_text'], '{{text page=1 block=0 chars=0:10}}')
        self.assertEqual(updated['resolved_md_text'], 'LINE DRAW:')
        self.assertEqual((self.root / 'accepted/book.json').read_bytes(), accepted)

    def test_deterministic_cleanup_reports_repeated_source_as_ambiguous(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'LINE DRAW: LINE DRAW:'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = 'LINE DRAW:.'
        findings = preview(book, self.root / 'pdf-to-markdown')
        first = next(item for item in findings if item['element_id'] == 'first')
        self.assertEqual(first['status'], 'ambiguous')

    def test_source_cleanup_accepts_one_explained_exception_without_hiding_another(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'ADD '}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = (
            '{{text page=1 block=0 chars=0:3}}Q | '
            '{{text page=1 block=0 chars=0:3}}Q')
        write_json(self.root / 'working/book.json', book)
        revision = load_input(self.root / 'working')[2]
        accepted = (self.root / 'accepted/book.json').read_bytes()
        report = self.state.preview_source_cleanup({'expected_revision': revision})
        missing = [item for item in report['findings']
                   if item['element_id'] == 'first' and item['literal'] == 'Q']
        self.assertEqual(len(missing), 2)
        with self.assertRaisesRegex(ValueError, 'Stale'):
            self.state.accept_source_cleanup_exception({
                'expected_revision': 'old', 'finding_id': missing[0]['id'],
                'note': 'OCR text reads ADDO; PDF image reads ADDQ.',
                'source_text': 'ADDO', 'corrected_text': 'ADDQ'})
        self.state.accept_source_cleanup_exception({
            'expected_revision': revision, 'finding_id': missing[0]['id'],
            'note': 'OCR text reads ADDO; PDF image reads ADDQ.',
            'source_text': 'ADDO', 'corrected_text': 'ADDQ'})
        saved = read_json(self.root / 'working/book.json')
        self.assertEqual(saved['source_cleanup_exceptions'][0]['source_text'], 'ADDO')
        self.assertEqual(saved['source_cleanup_exceptions'][0]['corrected_text'], 'ADDQ')
        self.assertEqual(saved['chapters'][0]['elements'][0]['md_text'],
                         book['chapters'][0]['elements'][0]['md_text'])
        self.assertEqual((self.root / 'accepted/book.json').read_bytes(), accepted)
        revision = load_input(self.root / 'working')[2]
        remaining = [item for item in self.state.preview_source_cleanup(
            {'expected_revision': revision})['findings']
            if item['element_id'] == 'first' and item['literal'] == 'Q']
        self.assertEqual(len(remaining), 1)

        saved['chapters'][0]['elements'][0]['md_text'] = ' ' + saved['chapters'][0]['elements'][0]['md_text']
        write_json(self.root / 'working/book.json', saved)
        revision = load_input(self.root / 'working')[2]
        remaining = [item for item in self.state.preview_source_cleanup(
            {'expected_revision': revision})['findings']
            if item['element_id'] == 'first' and item['literal'] == 'Q']
        self.assertEqual(len(remaining), 1)

        saved['chapters'][0]['elements'][0]['md_text'] = saved['chapters'][0]['elements'][0]['md_text'].replace(
            '{{text page=1 block=0 chars=0:3}}Q |', '{{text page=1 block=0 chars=0:3}}R |')
        write_json(self.root / 'working/book.json', saved)
        revision = load_input(self.root / 'working')[2]
        findings = self.state.preview_source_cleanup({'expected_revision': revision})['findings']
        self.assertTrue(any(item['element_id'] == 'first' and item['literal'] == 'R'
                            for item in findings))

    def test_deterministic_cleanup_uses_reference_block_for_repeated_table_word(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [
                       {'block_id': 0, 'text': 'data elsewhere'},
                       {'block_id': 1, 'text': '#<data>\nOther'},
                       {'block_id': 2, 'text': 'data elsewhere'},
                       {'block_id': 3, 'text': 'data elsewhere'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = (
            '{{text page=1 block=1 chars=8:13}} #&lt;data&gt;')
        findings = [item for item in preview(book, self.root / 'pdf-to-markdown')
                    if item['element_id'] == 'first']
        self.assertEqual([(item['status'], item.get('block'), item.get('chars')) for item in findings],
                         [('matched', 1, [2, 6])])

    def test_deterministic_cleanup_advances_through_repeated_words_in_one_block(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'prefix data, data, data'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = (
            '{{text page=1 block=0 chars=0:7}}data, data')
        findings = [item for item in preview(book, self.root / 'pdf-to-markdown')
                    if item['element_id'] == 'first']
        self.assertEqual([(item['status'], item.get('chars')) for item in findings],
                         [('matched', [7, 12]), ('matched', [13, 17])])

    def test_deterministic_cleanup_binds_single_math_letters_beside_references(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'address n (n even)'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = (
            '{{text page=1 block=0 chars=0:8}}$n$ ($n${{text page=1 block=0 chars=12:17}})')
        findings = [item for item in preview(book, self.root / 'pdf-to-markdown')
                    if item['element_id'] == 'first']
        self.assertEqual([(item['status'], item.get('chars')) for item in findings],
                         [('matched', [8, 9]), ('matched', [11, 12])])
        from review.source_cleanup import apply
        apply(book, findings, [item['id'] for item in findings])
        self.assertEqual(book['chapters'][0]['elements'][0]['md_text'],
                         '{{text page=1 block=0 chars=0:8}}'
                         '${{text page=1 block=0 chars=8:9}}$ '
                         '(${{text page=1 block=0 chars=11:12}}$'
                         '{{text page=1 block=0 chars=12:17}})')

    def test_deterministic_cleanup_uses_next_source_occurrence_for_repeated_letters(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'address n (n even)'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = (
            '{{text page=1 block=0 chars=0:8}}$n$ ($n$)')
        findings = [item for item in preview(book, self.root / 'pdf-to-markdown')
                    if item['element_id'] == 'first']
        self.assertEqual([(item['status'], item.get('chars')) for item in findings],
                         [('matched', [8, 9]), ('matched', [11, 12])])

    def test_deterministic_cleanup_binds_letter_attached_to_referenced_word(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'ADDQ'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = '{{text page=1 block=0 chars=0:3}}Q'
        findings = [item for item in preview(book, self.root / 'pdf-to-markdown')
                    if item['element_id'] == 'first']
        self.assertEqual([(item['status'], item.get('chars')) for item in findings],
                         [('matched', [3, 4])])
        from review.source_cleanup import apply
        apply(book, findings, [findings[0]['id']])
        write_json(self.root / 'working/book.json', book)
        self.assertEqual(read_json(self.root / 'working/book.json')['chapters'][0]['elements'][0]['md_text'],
                         '{{text page=1 block=0 chars=0:4}}')

    def test_deterministic_cleanup_joins_source_ranges_across_one_space(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'AB CDQ'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = (
            '{{text page=1 block=0 chars=0:2}}'
            ' {{text page=1 block=0 chars=3:5}}Q')
        findings = [item for item in preview(book, self.root / 'pdf-to-markdown')
                    if item['element_id'] == 'first']
        self.assertEqual([(item['status'], item.get('chars')) for item in findings],
                         [('matched', [5, 6])])
        from review.source_cleanup import apply
        apply(book, findings, [findings[0]['id']])
        write_json(self.root / 'working/book.json', book)
        self.assertEqual(read_json(self.root / 'working/book.json')['chapters'][0]['elements'][0]['md_text'],
                         '{{text page=1 block=0 chars=0:6}}')

    def test_any_editor_save_joins_references_across_working(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'AB CD'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = (
            '{{text page=1 block=0 chars=0:2}} {{text page=1 block=0 chars=3:5}}')
        book['chapters'][0]['elements'][1]['md_text'] = (
            '{{text page=1 block=0 chars=0:2}}{{text page=1 block=0 chars=3:5}}')
        write_json(self.root / 'working/book.json', book)
        self.assertEqual(read_json(self.root / 'working/book.json')['chapters'][0]['elements'][0]['md_text'],
                         '{{text page=1 block=0 chars=0:5}}')
        self.assertEqual(read_json(self.root / 'working/book.json')['chapters'][0]['elements'][1]['md_text'],
                         book['chapters'][0]['elements'][1]['md_text'])

        (self.root / 'working/book.json').write_text(json.dumps(book), encoding='utf-8')
        revision = load_input(self.root / 'working')[2]
        self.state.visibility({'id': 'second', 'visible': False, 'expected_revision': revision})
        saved = read_json(self.root / 'working/book.json')
        self.assertEqual(saved['chapters'][0]['elements'][0]['md_text'],
                         '{{text page=1 block=0 chars=0:5}}')
        self.assertEqual(saved['chapters'][0]['elements'][1]['md_text'],
                         book['chapters'][0]['elements'][1]['md_text'])
        self.assertFalse(saved['chapters'][0]['elements'][1]['visible'])
        self.assertEqual(read_json(self.root / 'accepted/book.json')['chapters'][0]['elements'][0]['md_text'],
                         'First part')

    def test_deterministic_cleanup_preserves_html_and_rejects_stale_selection(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'LINE DRAW:'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0].update(
            type='table', text_format='html', table_format='html',
            md_text='<table><tr><td>LINE DRAW:.</td></tr></table>')
        write_json(self.root / 'working/book.json', book)
        revision = load_input(self.root / 'working')[2]
        report = self.state.preview_source_cleanup({'expected_revision': revision})
        match = next(item for item in report['findings'] if item['element_id'] == 'first')
        self.assertEqual(match['literal'], 'LINE DRAW:.')
        key = match['id']
        with self.assertRaisesRegex(ValueError, 'Stale edit'):
            self.state.apply_source_cleanup({'expected_revision': 'old', 'selections': [key]})
        self.state.apply_source_cleanup({'expected_revision': revision, 'selections': [key]})
        updated = load_input(self.root / 'working')[0]['chapters'][0]['elements'][0]
        self.assertIn('<table>', updated['md_text'])
        self.assertIn('{{text page=1 block=0 chars=0:10}}', updated['md_text'])

    def test_deterministic_cleanup_can_bind_chapter_title(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'Example'}]})
        report = self.state.preview_source_cleanup({'expected_revision': self.revision})
        title = next(item for item in report['findings'] if item['path'] == '/chapters/0/title')
        self.assertEqual(title['status'], 'matched')
        key = title['id']
        self.state.apply_source_cleanup({'expected_revision': self.revision, 'selections': [key]})
        self.assertEqual(load_input(self.root / 'working')[0]['chapters'][0]['title'],
                         '{{text page=1 block=0 chars=0:7}}')

    def test_deterministic_cleanup_rejects_changed_pdf_match(self):
        path = self.root / 'pdf-to-markdown/02_preprocess/page_0001.json'
        write_json(path, {'blocks': [{'block_id': 0, 'text': 'LINE DRAW:'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = 'LINE DRAW:.'
        write_json(self.root / 'working/book.json', book)
        revision = load_input(self.root / 'working')[2]
        report = self.state.preview_source_cleanup({'expected_revision': revision})
        match = next(item for item in report['findings'] if item['element_id'] == 'first')
        write_json(path, {'blocks': [{'block_id': 0, 'text': 'LINE DRAW:.'}]})
        with self.assertRaisesRegex(ValueError, 'current, unambiguous'):
            self.state.apply_source_cleanup({'expected_revision': revision,
                                             'selections': [match['id']]})

    def test_deterministic_cleanup_ignores_literal_subscript_notation(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'd15'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = (
            '{{text page=1 block=0 chars=0:1}}₁₆,{{text page=1 block=0 chars=1:3}}₈')
        findings = preview(book, self.root / 'pdf-to-markdown')
        self.assertFalse(any(item['element_id'] == 'first' for item in findings))

    def test_deterministic_cleanup_does_not_offer_neighbor_page_for_one_character(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0002.json',
                   {'blocks': [{'block_id': 0, 'text': 'I 8'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = 'I'
        findings = preview(book, self.root / 'pdf-to-markdown')
        first = next(item for item in findings if item['element_id'] == 'first')
        self.assertEqual(first['status'], 'ambiguous')
        self.assertNotIn('reference', first)

    def test_deterministic_cleanup_prefers_source_adjacent_to_existing_reference(self):
        source = 'OUTPUT: AH = Line Status'
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'OUTPUT: other content'},
                               {'block_id': 1, 'text': source}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = (
            'OUTPUT: {{text page=1 block=1 chars=8:' + str(len(source)) + '}}')
        findings = preview(book, self.root / 'pdf-to-markdown')
        first = next(item for item in findings if item['element_id'] == 'first')
        self.assertEqual((first['status'], first['block'], first['chars']),
                         ('matched', 1, [0, 7]))

    def test_source_replacement_survives_html_table_normalization(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'LINE DRAW:'}]})
        book = read_json(self.root / 'working/book.json')
        element = book['chapters'][0]['elements'][0]
        element.update(type='table', text_format='html', table_format='html',
                       md_text='<table><tr><td>LINE DRAW:.</td></tr></table>')
        write_json(self.root / 'working/book.json', book)
        revision = load_input(self.root / 'working')[2]
        self.state.mutate(revision, lambda current: replace_source_text(
            current, self.root, {'path': '/chapters/0/elements/0/md_text',
                                 'template': '<table><tr><td>{{text page=1 block=0 chars=0:10}}</td></tr></table>'}))
        updated = load_input(self.root / 'working')[0]['chapters'][0]['elements'][0]
        self.assertIn('{{text page=1 block=0 chars=0:10}}', updated['md_text'])
        self.assertNotIn('LINE DRAW:.', updated['md_text'])

    def test_markdown_prompt_presets_are_loaded_into_agent_context(self):
        presets = {'describe_images': 'describe-images.md',
                   'describe_tables': 'describe-tables.md',
                   'proofreading': 'proofreading.md'}
        for identifier, filename in presets.items():
            with self.subTest(identifier=identifier):
                seen = []

                class Client:
                    def __init__(self, config, **kwargs):
                        pass
                    def __enter__(self):
                        return self
                    def __exit__(self, *args):
                        pass
                    def generate_json(self, prompt, **kwargs):
                        seen.append(prompt)
                        return {'action': 'finish', 'summary': 'Checked the task prompt.'}

                result = run_agent(self.state, {'instruction': 'Work on prepared groups or chapters',
                                                'prompt_preset': identifier,
                                                'expected_revision': self.revision},
                                   client_factory=Client)
                source = Path(__file__).resolve().parents[1] / 'prompts' / filename
                self.assertIn(source.read_text(encoding='utf-8'), seen[0])
                self.assertIn('one JSON action per turn', seen[0])
                wrapper = source.parent / 'codex-agent-selected-task.md'
                self.assertIn(wrapper.read_text(encoding='utf-8').split('\n\n')[1].strip(), seen[0])
                request = read_json(self.root / result['diagnostics'] / 'request.json')
                self.assertEqual(request['prompt_preset'], identifier)
        with self.assertRaisesRegex(ValueError, 'Unknown Codex Working prompt preset'):
            run_agent(self.state, {'instruction': 'Invalid preset', 'prompt_preset': '../other',
                                   'expected_revision': self.revision})

    def test_suggested_tasks_are_loaded_from_markdown(self):
        prompts = Path(__file__).resolve().parents[1] / 'prompts'
        filenames = {'join_paragraphs': 'join-paragraphs-across-pdf-pages.md',
                     'source_cleanup': 'clean-direct-book-text.md',
                     'describe_images': 'describe-images-working.md',
                     'describe_tables': 'describe-tables-working.md',
                     'proofreading': 'proofreading-working.md'}
        instructions = self.state.status()['codex_suggested_tasks']
        for key, filename in filenames.items():
            with self.subTest(key=key):
                self.assertEqual(instructions[key], (prompts / filename).read_text(encoding='utf-8').strip())
        seen = []

        class Client:
            def __init__(self, config, **kwargs):
                pass

            def __enter__(self):
                return self

            def __exit__(self, *args):
                pass

            def generate_json(self, prompt, **kwargs):
                seen.append(prompt)
                return {'action': 'finish', 'summary': 'Checked join prompt.'}

        run_agent(self.state, {'instruction': instructions['join_paragraphs'],
                               'expected_revision': self.revision}, client_factory=Client)
        self.assertIn(instructions['join_paragraphs'], seen[0])

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

    def test_browser_shows_source_correction_and_unanchored_text_separately(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'ADDO'}]})
        book = read_json(self.root / 'working/book.json')
        book['chapters'][0]['elements'][0]['md_text'] = (
            '{{text page=1 block=0 chars=0:4 replace=3:4 with="Q"}} '
            '{{accepted text="X"}} Y '
            r'$\\overline\{\\mathrm\{{{accepted text="AS"}}\}\}$')
        write_json(self.root / 'working/book.json', book)
        server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        server.daemon_threads = True
        server.state = self.state
        threading.Thread(target=server.serve_forever, daemon=True).start()
        rendering = threading.Thread(target=self.state.render_loop, daemon=True)
        rendering.start()
        try:
            with browser_page(viewport={'width': 1400, 'height': 850}) as page:
                errors = []
                page.on('pageerror', lambda error: errors.append(str(error)))
                page.goto(f'http://127.0.0.1:{server.server_port}')
                page.wait_for_function('state?.can_accept && !loading')
                self.assertEqual(page.locator('#working .source-diff-correction').inner_text(), 'Q')
                self.assertEqual(page.locator('#working .source-diff-unanchored').first.inner_text(), 'X')
                self.assertEqual(page.locator('#working .source-diff-correction').get_attribute('title'),
                                 'PDF text: O → Working: Q')
                self.assertEqual(page.locator('#accepted .source-diff-correction').count(), 0)
                self.assertFalse(page.locator('#show-source-differences').is_checked())
                page.locator('#show-source-differences').check()
                self.assertTrue(page.locator('#working .chapter-content').first.evaluate(
                    "node => node.classList.contains('show-source-differences')"))
                math_addition = page.locator('#working .katex-html .source-diff-unanchored').filter(
                    has_text='AS')
                self.assertEqual(math_addition.count(), 1)
                self.assertNotEqual(math_addition.evaluate(
                    'node => getComputedStyle(node).backgroundColor'),
                    page.locator('#working .source-diff-correction').evaluate(
                        'node => getComputedStyle(node).backgroundColor'))
                page.locator('#book-actions').click()
                page.locator('[data-book-task="source_cleanup"]').click()
                page.wait_for_function('sourceCleanupReport?.findings?.length > 0')
                raw_index = page.evaluate('sourceCleanupReport.findings.findIndex(item => item.literal === "Y")')
                self.assertGreaterEqual(raw_index, 0)
                page.locator('#source-cleanup-findings .source-cleanup-row').nth(raw_index).locator('a').click()
                self.assertEqual(page.locator('#working .source-diff-focus').inner_text(), 'Y')
                self.assertFalse(errors)
        finally:
            self.state.stopping.set()
            with self.state.condition:
                self.state.condition.notify_all()
            server.shutdown()
            server.server_close()
            rendering.join(20)

    def test_browser_starts_general_command_without_element_selection(self):
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'Example'}]})
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
                with browser_page(viewport={'width': 1500, 'height': 900}) as page:
                    errors = []
                    page.on('pageerror', lambda error: errors.append(str(error)))
                    page.goto(f'http://127.0.0.1:{server.server_port}')
                    page.wait_for_function('state?.can_accept && !loading')
                    self.assertEqual(page.locator('#element-actions, #copy, #menu-codex, #menu-codex-working').count(), 0)
                    page.locator('#book-actions').click()
                    self.assertEqual(page.locator('#book-menu button').all_text_contents(),
                                     ['Codex...', 'Proofread chapters', 'Describe images',
                                      'Describe tables', 'Clean direct book text'])
                    for task in ['proofreading', 'describe_images', 'describe_tables']:
                        page.locator(f'[data-book-task="{task}"]').click()
                        self.assertEqual(page.locator('#codex-preset').input_value(), task)
                        self.assertEqual(page.locator('#codex-instruction').input_value(),
                                         self.state.status()['codex_suggested_tasks'][task])
                        page.locator('#cancel-codex').click()
                        page.locator('#book-actions').click()
                    page.locator('#book-codex').click()
                    page.locator('#codex-preset').select_option(label='Join paragraphs across PDF pages')
                    self.assertEqual(page.locator('#codex-instruction').input_value(),
                                     'Find places where a paragraph continues across a PDF page boundary, '
                                     'and merge the split paragraphs.')
                    for label, filename in [('Describe images', 'describe-images.md'),
                                            ('Describe tables', 'describe-tables.md'),
                                            ('Proofread chapters', 'proofreading.md')]:
                        page.locator('#codex-preset').select_option(label=label)
                        self.assertIn(filename, page.locator('#codex-instruction').input_value())
                    page.locator('#codex-instruction').fill('Proofread the first selected chapter')
                    self.assertEqual(page.locator('#codex-preset').locator('option:checked').inner_text(),
                                     'Proofread chapters')
                    page.locator('#codex-preset').select_option(label='Join paragraphs across PDF pages')
                    page.locator('#codex-instruction').fill('Edit the first phrase in Working')
                    self.assertEqual(page.locator('#codex-preset').input_value(), '')
                    page.locator('#run-codex').click()
                    page.wait_for_function('state?.codex_agent?.state === "completed" && '
                                           '!document.getElementById("codex-dialog").open')
                    self.assertEqual(load_input(self.root / 'working')[0]['chapters'][0]['elements'][0]['resolved_md_text'],
                                     'Edited by Codex')
                    self.assertEqual(load_input(self.root / 'accepted')[0]['chapters'][0]['elements'][0]['resolved_md_text'],
                                     'First part')
                    page.locator('#book-actions').click()
                    page.locator('[data-book-task="source_cleanup"]').click()
                    page.wait_for_function('sourceCleanupReport?.findings?.length > 0')
                    self.assertTrue(page.locator('#source-cleanup-dialog').evaluate('(dialog) => dialog.open'))
                    matches = page.locator('#source-cleanup-findings input[type=checkbox]')
                    self.assertGreater(matches.count(), 0)
                    self.assertFalse(any(matches.nth(index).is_checked() for index in range(matches.count())))
                    page.locator('#source-cleanup-select-all').click()
                    self.assertTrue(all(matches.nth(index).is_checked() for index in range(matches.count())))
                    literals = page.locator('#source-cleanup-findings button')
                    before = literals.count()
                    self.assertGreater(before, 0)
                    literals.first.click()
                    page.wait_for_function('!document.getElementById("source-cleanup-dialog").open')
                    saved_revision = load_input(self.root / 'working')[2]
                    page.wait_for_function('(revision) => state?.authored_revision === revision && !state.pending',
                                           arg=saved_revision)
                    saved_book = read_json(self.root / 'working/book.json')
                    self.assertTrue(any('{{accepted text=' in element.get('md_text', '')
                                        for chapter in saved_book['chapters'] for element in chapter['elements']))
                    page.locator('#book-actions').click()
                    page.locator('[data-book-task="source_cleanup"]').click()
                    page.wait_for_function('sourceCleanupReport?.findings?.length > 0')
                    self.assertEqual(page.locator('#source-cleanup-findings button').count(), before - 1)
                    page.locator('#source-cleanup-close').click()
                    actions.append({'action': 'finish', 'summary': 'Checked table prompt.'})
                    page.locator('#book-actions').click()
                    page.locator('[data-book-task="describe_tables"]').click()
                    self.assertEqual(page.locator('#codex-preset').input_value(), 'describe_tables')
                    page.locator('#codex-instruction').fill('Describe the prepared table group')
                    page.locator('#run-codex').click()
                    page.wait_for_function('state?.codex_agent?.summary === "Checked table prompt." && '
                                           '!document.getElementById("codex-dialog").open')
                    requests = [read_json(path) for path in
                                (self.root / '.agent/tmp/book-codex-agent').glob('*/request.json')]
                    self.assertIn('describe_tables', [request['prompt_preset'] for request in requests])
                    page.locator('#working [data-element="first"]').click(button='right')
                    page.locator('#book-actions').click()
                    page.locator('#book-codex').click()
                    self.assertIn('Starting at first', page.locator('#codex-id').inner_text())
                    self.assertIn('Checked table prompt.', page.locator('#codex-status').inner_text())
                    self.assertTrue(page.locator('#codex-dialog').evaluate('(dialog) => dialog.open'))
                    page.locator('#cancel-codex').click()
                    page.wait_for_function('!loading && !state.pending')
                    self.assertEqual(page.locator('#menu-codex, #menu-codex-working').count(), 0)
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

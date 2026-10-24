"""Codex changes must preserve content and replace only the current selected element."""

from http.server import ThreadingHTTPServer
from pathlib import Path
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

from PIL import Image
from playwright.sync_api import sync_playwright

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.codex_edit import run_command
from review.content_validation import validate_content
from review.model import element_markdown, load_input, write_json
from review.service import Handler, ReviewState


MARKDOWN = '| Pin | Name |\n| --- | --- |\n| 1 | DATA |'
HTML = '<table><tr><th>Pin</th><th>Name</th></tr><tr><td>1</td><td>DATA</td></tr></table>'


class CodexContentTests(unittest.TestCase):
    def test_nested_html_table_allows_layout_edit_but_preserves_cells(self):
        before = ('<table><tr><th colspan="2">Data format'
                  '<table><tr><td>Word</td><td>16</td></tr>'
                  '<tr><td colspan="2">Bits</td></tr></table></th></tr>'
                  '<tr><td>Exponent</td><td>15</td></tr></table>')
        after = before.replace('<table><tr><td>Word',
                               '<table style="margin-inline: auto"><tr><td>Word')
        self.assertTrue(validate_content(before, after, 'html', 'html')['valid'])
        spaced = after.replace('format<table', 'format\n<table')
        self.assertTrue(validate_content(before, spaced, 'html', 'html')['valid'])
        with self.assertRaisesRegex(ValueError, 'table cells'):
            validate_content(before, after.replace('colspan="2">Bits',
                                                   'colspan="1">Bits'), 'html', 'html')
        with self.assertRaisesRegex(ValueError, 'table cells'):
            validate_content(before, after.replace('<tr><td colspan="2">Bits</td></tr>',
                                                   'Bits'), 'html', 'html')
        with self.assertRaisesRegex(ValueError, 'changed content'):
            validate_content(before, after.replace('>16</td>', '>18</td>'), 'html', 'html')

    def test_headerless_html_preserves_delimited_math_from_markdown(self):
        before = ('|  |  |\n| --- | --- |\n'
                  '| STATE 0 | Drives $R/\\overline{W}$ high. |\n'
                  '| STATE 2 | Asserts $\\overline{AS}$ low. |')
        after = ('<table><tr><td>STATE 0</td><td>Drives R/'
                 '<span style="text-decoration: overline">W</span> high.</td></tr>'
                 '<tr><td>STATE 2</td><td>Asserts '
                 '<span style="text-decoration: overline">AS</span> low.</td></tr></table>')
        self.assertTrue(validate_content(before, after, 'markdown', 'html')['valid'])
        with self.assertRaisesRegex(ValueError, 'changed content'):
            validate_content(before, after.replace('>AS</span>', '>DS</span>'),
                             'markdown', 'html')

    def test_headerless_html_preserves_math_subscripts_from_markdown_legend(self):
        before = ('|  |  |  |\n| --- | --- | --- |\n'
                  '| EA | = | Effective Address |\n'
                  '| $d_8$ | = | 8-Bit Offset |\n'
                  '| $d_{16}$ | = | 16-Bit Offset |')
        after = ('<table><tr><td>EA</td><td>=</td><td>Effective Address</td></tr>'
                 '<tr><td>d<sub>8</sub></td><td>=</td><td>8-Bit Offset</td></tr>'
                 '<tr><td>d<sub>16</sub></td><td>=</td><td>16-Bit Offset</td></tr></table>')
        self.assertTrue(validate_content(before, after, 'markdown', 'html')['valid'])
        with self.assertRaisesRegex(ValueError, 'changed content'):
            validate_content(before, after.replace('d<sub>8</sub>', 'd8'), 'markdown', 'html')
        with self.assertRaisesRegex(ValueError, 'changed content'):
            validate_content(before.replace('$d_8$', 'd_8').replace('$d_{16}$', 'd_{16}'),
                             after, 'markdown', 'html')

    def test_markdown_html_and_csv_preserve_cells(self):
        self.assertTrue(validate_content(MARKDOWN, HTML)['valid'])
        self.assertTrue(validate_content(HTML, 'Pin,Name\n1,DATA\n', 'html', 'csv')['valid'])
        self.assertTrue(validate_content('Xn & <ea>', 'X<sub>n</sub> &amp; &lt;ea&gt;')['valid'])
        self.assertTrue(validate_content('Alpha, Beta', '**Alpha**,\nBeta')['valid'])

    def test_changes_drops_duplicates_and_word_splitting_are_rejected(self):
        for text in ('Pin,Name\n1,DATO', 'Pin,Name\n1,', 'Pin,Name\n1,DATA DATA', 'Pin,Name\n1,DA TA'):
            with self.subTest(text=text), self.assertRaisesRegex(ValueError, 'changed content'):
                validate_content(MARKDOWN, text, 'markdown', 'csv')
        with self.assertRaisesRegex(ValueError, 'changed content'):
            validate_content('Alpha Beta', 'AlphaBeta')

    def test_same_text_cannot_move_between_cells_or_change_spans(self):
        before = '<table><tr><td>A B</td><td>C</td></tr></table>'
        for after in ('<table><tr><td>A</td><td>B C</td></tr></table>',
                      '<table><tr><td colspan="2">A B</td><td>C</td></tr></table>'):
            with self.assertRaisesRegex(ValueError, 'table cells'):
                validate_content(before, after, 'html', 'html')

    def test_removes_decorative_dash_column_and_edits_punctuation(self):
        before = ('<table><tr><td>RR</td><td>—</td><td>Rerun flag, value 0.</td></tr>'
                  '<tr><td>IF</td><td>—</td><td>Instruction-fetch.</td></tr></table>')
        after = ('<table><tr><td>RR</td><td>Rerun flag value 0</td></tr>'
                 '<tr><td>IF</td><td>Instruction fetch</td></tr></table>')
        self.assertTrue(validate_content(before, after, 'html', 'html')['valid'])
        with self.assertRaisesRegex(ValueError, 'changed content'):
            validate_content(before, after.replace('value 0', 'value 1'), 'html', 'html')
        with self.assertRaises(ValueError):
            validate_content(before, after.replace('<td>IF</td>', '<td>IF</td><td>new</td>'),
                             'html', 'html')

    def test_keeps_non_decorative_table_columns_and_spans(self):
        before = ('<table><tr><td>A</td><td>—</td><td>First</td></tr>'
                  '<tr><td>B</td><td>value</td><td>Second</td></tr></table>')
        after = ('<table><tr><td>A</td><td>First</td></tr>'
                 '<tr><td>B</td><td>value</td><td>Second</td></tr></table>')
        with self.assertRaisesRegex(ValueError, 'table cells'):
            validate_content(before, after, 'html', 'html')


class CodexCommandTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.element = {'id': 'selected', 'type': 'table', 'role': None, 'visible': True,
                        'pdf_regions': [{'page': 1, 'bbox': [10, 10, 190, 70]}],
                        'table_format': 'markdown', 'md_text': MARKDOWN}
        book = {'toc': {'depth': 2}, 'chapters': [{'id': 'c', 'title': 'Example',
                'markdown_file': 'example.md', 'elements': [self.element,
                {'id': 'other', 'type': 'prose', 'role': None, 'pdf_regions': [], 'md_text': 'Keep this.'}]}]}
        write_json(self.root / 'working/book.json', book)
        write_json(self.root / 'accepted/book.json', book)
        pages = self.root / 'pdf/pages'
        pages.mkdir(parents=True)
        Image.new('RGB', (200, 100), 'white').save(pages / 'page_0001.png')
        self.state = ReviewState(self.root)
        self.command = {'id': 'selected', 'instruction': 'Convert to HTML', 'format_mode': 'allow_change',
                        'expected_revision': load_input(self.root / 'working')[2]}
        self.response = {'id': 'selected', 'type': 'table', 'format': 'html', 'text': HTML}
        # Keep diagnostic output inside the test's disposable directory.
        patcher = patch('review.codex_edit.REPOSITORY', self.root)
        patcher.start()
        self.addCleanup(patcher.stop)

    def client(self, response, callback=None):
        owner = self
        class Client:
            def __init__(self, config, **kwargs):
                owner.assertEqual(kwargs['stage'], 'book_codex_edit')
            def __enter__(self):
                return self
            def __exit__(self, *args):
                pass
            def generate_json(self, prompt, image_path=None, validator=None):
                owner.assertIn('Copy ID', prompt)
                owner.assertNotIn('Keep this.', prompt)
                owner.assertIsNotNone(image_path)
                if callback:
                    callback()
                validator(response)
                return response
        return Client

    def test_replaces_selected_element_and_preserves_accepted(self):
        accepted = (self.root / 'accepted/book.json').read_bytes()
        result = run_command(self.state, self.command, client_factory=self.client(self.response))
        book, _, _ = load_input(self.root / 'working')
        selected, other = book['chapters'][0]['elements']
        self.assertEqual(selected['resolved_md_text'], HTML)
        self.assertEqual(selected['id'], 'selected')
        self.assertEqual(other['resolved_md_text'], 'Keep this.')
        self.assertEqual((self.root / 'accepted/book.json').read_bytes(), accepted)
        self.assertTrue(result['validation']['valid'])

    def test_preserve_format_rejects_html_table_even_when_declared_markdown(self):
        before = (self.root / 'working/book.json').read_bytes()
        command = {key: value for key, value in self.command.items() if key != 'format_mode'}
        with self.assertRaisesRegex(ValueError, 'format'):
            run_command(self.state, command, client_factory=self.client(self.response))
        disguised = {**self.response, 'format': 'markdown'}
        with self.assertRaisesRegex(ValueError, 'HTML table'):
            run_command(self.state, command, client_factory=self.client(disguised))
        reverse_disguise = {**self.response, 'text': MARKDOWN}
        with self.assertRaisesRegex(ValueError, 'without HTML table markup'):
            run_command(self.state, self.command, client_factory=self.client(reverse_disguise))
        self.assertEqual((self.root / 'working/book.json').read_bytes(), before)

    def test_html_conversion_rejects_unrendered_math(self):
        before = '|  |  |\n| --- | --- |\n| STATE 0 | Drives $R/\\overline{W}$ high. |'
        html = '<table><tr><td>STATE 0</td><td>Drives $R/\\overline{W}$ high.</td></tr></table>'
        converted = ('<table><tr><td>STATE 0</td><td>Drives '
                     'R/<span style="text-decoration: overline">W</span> high.</td></tr></table>')
        with self.assertRaisesRegex(ValueError, 'math'):
            validate_content(before, html, 'markdown', 'html')
        self.assertTrue(validate_content(before, converted, 'markdown', 'html')['valid'])

    def test_existing_html_table_can_repair_unrendered_math(self):
        before = '<table><tr><td>R/$\\overline{\\mathrm{W}}$</td></tr></table>'
        after = ('<table><tr><td>R/'
                 '<span style="text-decoration: overline">W</span></td></tr></table>')
        self.assertTrue(validate_content(before, after, 'html', 'html')['valid'])
        with self.assertRaisesRegex(ValueError, 'math'):
            validate_content(before, before, 'html', 'html')
        hex_values = '<table><tr><td>$FF or $00</td></tr></table>'
        self.assertTrue(validate_content(hex_values, hex_values, 'html', 'html')['valid'])

    def test_bad_content_id_or_stale_result_never_saves(self):
        before = (self.root / 'working/book.json').read_bytes()
        for changes in ({'text': HTML.replace('DATA', 'DATO')}, {'id': 'other'}, {'asset': 'bad.png'}):
            with self.assertRaises(ValueError):
                run_command(self.state, self.command, client_factory=self.client({**self.response, **changes}))
            self.assertEqual((self.root / 'working/book.json').read_bytes(), before)
        def concurrent_edit():
            self.state.markdown({'id': 'other', 'text': 'Manual edit.', 'expected_revision': self.command['expected_revision']})
        with self.assertRaisesRegex(ValueError, 'Working changed'):
            run_command(self.state, self.command, client_factory=self.client(self.response, concurrent_edit))
        book, _, _ = load_input(self.root / 'working')
        self.assertEqual(book['chapters'][0]['elements'][0]['resolved_md_text'], MARKDOWN)
        self.assertEqual(book['chapters'][0]['elements'][1]['resolved_md_text'], 'Manual edit.')

    def test_table_to_graphic_retains_text_then_returns_to_csv(self):
        response = {'id': 'selected', 'type': 'graphic', 'format': 'markdown', 'text': MARKDOWN}
        run_command(self.state, self.command, client_factory=self.client(response))
        book, assets, revision = load_input(self.root / 'working')
        element = book['chapters'][0]['elements'][0]
        self.assertEqual(element['resolved_md_text'], MARKDOWN)
        self.assertIn(element['asset'], assets)
        self.assertEqual(element['pdf_regions'], self.element['pdf_regions'])
        response.update(type='table', format='csv', text='Pin,Name\n1,DATA\n')
        run_command(self.state, {**self.command, 'expected_revision': revision}, client_factory=self.client(response))
        element = load_input(self.root / 'working')[0]['chapters'][0]['elements'][0]
        self.assertNotIn('asset', element)
        self.assertTrue(element_markdown(element).startswith('```csv\n'))

    def test_preserves_existing_references_and_uses_ocr_for_textless_graphic(self):
        reference = '{{text page=1 block=0 chars=0:3}}'
        write_json(self.root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'Pin Name 1 DATA\n', 'bbox': [10, 10, 190, 70],
                                'char_bboxes': [[20, 20, 30, 30]] * 16}],
                    'raster': {'display_to_pixels': [1, 0, 0, 1, 0, 0]}})
        book, _, _ = load_input(self.root / 'working')
        book['chapters'][0]['elements'][0]['md_text'] = MARKDOWN.replace('Pin', reference)
        write_json(self.root / 'working/book.json', book)
        self.command['expected_revision'] = load_input(self.root / 'working')[2]
        run_command(self.state, self.command, client_factory=self.client(self.response))
        book, _, _ = load_input(self.root / 'working')
        self.assertIn(reference, book['chapters'][0]['elements'][0]['md_text'])
        graphic = book['chapters'][0]['elements'][0]
        graphic['type'] = 'graphic'
        graphic.pop('md_text')
        graphic.pop('resolved_md_text')
        write_json(self.root / 'working/book.json', book)
        self.command['expected_revision'] = load_input(self.root / 'working')[2]
        run_command(self.state, self.command, client_factory=self.client(self.response))
        result = load_input(self.root / 'working')[0]['chapters'][0]['elements'][0]
        self.assertEqual(result['type'], 'table')
        self.assertEqual(result['resolved_md_text'], HTML)

    def test_ui_command_uses_real_endpoint_and_reports_rejection(self):
        # Rendering and server exercise the real command; only inference is offline.
        server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        server.daemon_threads = True
        server.state = self.state
        threading.Thread(target=server.serve_forever, daemon=True).start()
        rendering = threading.Thread(target=self.state.render_loop, daemon=True)
        rendering.start()
        try:
            with patch('review.codex_edit.CodexClient', self.client(self.response)):
                with sync_playwright() as p:
                    browser = p.chromium.launch(channel='msedge')
                    page = browser.new_page(viewport={'width': 1600, 'height': 950})
                    errors = []
                    page.on('pageerror', lambda e: errors.append(str(e)))
                    page.goto(f'http://127.0.0.1:{server.server_port}')
                    page.wait_for_function('state?.can_accept && !loading')
                    page.locator('#working [data-element="selected"]').click(button='right')
                    page.locator('#menu-codex').click()
                    page.locator('#codex-instruction').fill('Convert this table to HTML')
                    self.assertEqual(page.locator('#codex-format-mode').input_value(), 'preserve')
                    self.assertEqual(page.locator('#codex-format-guidance').input_value(),
                                     self.state.status()['codex_format_guidance']['preserve'])
                    page.locator('#codex-format-mode').select_option('allow_change')
                    self.assertEqual(page.locator('#codex-format-guidance').input_value(),
                                     self.state.status()['codex_format_guidance']['allow_change'])
                    page.locator('#run-codex').click()
                    page.wait_for_function('!document.getElementById("codex-dialog").open && state?.can_accept && !loading')
                    self.assertEqual(load_input(self.root / 'working')[0]['chapters'][0]['elements'][0]['table_format'], 'html')
                    self.assertEqual(page.evaluate('state.changes.map(change => change.id)'), ['selected'])
                    self.assertTrue(page.locator('#previous').is_enabled())
                    self.assertTrue(page.locator('#next').is_enabled())
                    page.locator('#next').click()
                    self.assertEqual(page.evaluate('selected'), 'selected')
                    page.locator('#previous').click()
                    self.assertEqual(page.evaluate('selected'), 'selected')
                    self.response['text'] = HTML.replace('DATA', 'DATO')
                    before = (self.root / 'working/book.json').read_bytes()
                    page.locator('#working [data-element="selected"]').click(button='right')
                    page.locator('#menu-codex').click()
                    page.locator('#codex-instruction').fill('Convert again')
                    page.locator('#run-codex').click()
                    page.locator('#codex-error').wait_for(state='visible')
                    self.assertIn('changed content', page.locator('#codex-error').inner_text())
                    self.assertEqual((self.root / 'working/book.json').read_bytes(), before)
                    self.assertFalse(errors)
                    browser.close()
        finally:
            self.state.stopping.set()
            with self.state.condition:
                self.state.condition.notify_all()
            server.shutdown()
            server.server_close()
            rendering.join(20)

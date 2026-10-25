"""HTML table merging preserves rows and rejects incompatible columns."""

import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.model import load_input, write_json
from review.markdown import format_html_table
from review.service import ReviewState


class HTMLTableMergeTests(unittest.TestCase):
    def test_editor_formats_existing_table_and_saves_multiline_recipe(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            compact = '<table><tr><th colspan="2">Name</th></tr><tr><td>A<br>B</td><td>C</td></tr></table>'
            element = {'id': 'table', 'type': 'table', 'role': None, 'visible': True,
                       'table_format': 'html', 'md_text': compact, 'pdf_regions': []}
            book = {'toc': {'depth': 2}, 'chapters': [{'id': 'chapter', 'title': 'Example',
                    'markdown_file': 'example.md', 'elements': [element]}]}
            write_json(root / 'working/book.json', book)
            state = ReviewState(root)
            state.working = {**book, 'input_fingerprint': load_input(root / 'working')[2]}
            formatted = state.status()['editor_html']['table']
            self.assertEqual(formatted, format_html_table(compact))
            self.assertIn('\n', formatted)
            state.markdown({'id': 'table', 'text': compact,
                            'expected_revision': load_input(root / 'working')[2]})
            saved = json.loads((root / 'working/book.json').read_text(encoding='utf-8'))
            self.assertEqual(saved['chapters'][0]['elements'][0]['md_text'], formatted)
            self.assertEqual(load_input(root / 'working')[0]['chapters'][0]['elements'][0]['resolved_md_text'],
                             formatted)

    def test_merge_accepts_subdivided_spanned_column(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = '<table><tr><th>Name</th><th>Bits</th><th>Details</th></tr>' \
                    '<tr><td>A</td><td>00</td><td>First</td></tr></table>'
            second = '<table><tr><th>Name</th><th>Bits</th>' \
                     '<th colspan="2">Details</th></tr>' \
                     '<tr><td colspan="2"></td><td>Left</td><td>Right</td></tr></table>'
            elements = [
                {'id': identifier, 'type': 'table', 'role': None, 'visible': True,
                 'table_format': 'html', 'md_text': text, 'pdf_regions': []}
                for identifier, text in [('first', first), ('second', second)]
            ]
            book = {'toc': {'depth': 2}, 'chapters': [{'id': 'chapter', 'title': 'Example',
                    'markdown_file': 'example.md', 'elements': elements}]}
            write_json(root / 'working/book.json', book)
            state = ReviewState(root)
            state.merge_tables({'id': 'first', 'expected_revision': load_input(root / 'working')[2]})
            merged = load_input(root / 'working')[0]['chapters'][0]['elements']
            self.assertEqual(len(merged), 1)
            self.assertNotIn('<th colspan="2">Details</th>', merged[0]['md_text'])
            self.assertIn('<td>Left</td>', merged[0]['md_text'])
            self.assertIn('<td>Right</td>', merged[0]['md_text'])

    def test_merge_drops_second_html_header_section(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = ('<table><thead><tr><th>Name</th><th>Value</th></tr></thead>'
                     '<tbody><tr><td>A</td><td>1</td></tr></tbody></table>')
            second = ('<table><thead><tr><th>Name</th><th>Value</th></tr></thead>'
                      '<tbody><tr><td>B</td><td>2</td></tr></tbody></table>')
            elements = [
                {'id': identifier, 'type': 'table', 'role': None, 'visible': True,
                 'table_format': 'html', 'md_text': text, 'pdf_regions': []}
                for identifier, text in [('first', first), ('second', second)]
            ]
            book = {'toc': {'depth': 2}, 'chapters': [{'id': 'chapter', 'title': 'Example',
                    'markdown_file': 'example.md', 'elements': elements}]}
            write_json(root / 'working/book.json', book)
            ReviewState(root).merge_tables({'id': 'first', 'expected_revision': load_input(root / 'working')[2]})
            merged = load_input(root / 'working')[0]['chapters'][0]['elements'][0]['resolved_md_text']
            self.assertEqual(merged.count('<th>Name</th>'), 1)
            self.assertIn('<td>A</td>', merged)
            self.assertIn('<td>B</td>', merged)

    def test_merge_preserves_single_wrapper_rows_and_regions(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = '<table><tbody><tr><td>A</td><td>B</td></tr></tbody></table>'
            second = '<table><tbody>\n<tr><td>C</td><td>D</td></tr>\n</tbody></table>'
            elements = [
                {'id': identifier, 'type': 'table', 'role': None, 'visible': True,
                 'table_format': 'html', 'md_text': text,
                 'pdf_regions': [{'page': page, 'bbox': [0, 0, 10, 10]}]}
                for identifier, text, page in [('first', first, 1), ('second', second, 2)]
            ]
            book = {'toc': {'depth': 2}, 'chapters': [{'id': 'chapter', 'title': 'Example',
                    'markdown_file': 'example.md', 'elements': elements}]}
            write_json(root / 'working/book.json', book)
            state = ReviewState(root)
            command = {'id': 'first', 'expected_revision': load_input(root / 'working')[2]}
            state.merge_tables(command)
            merged = load_input(root / 'working')[0]['chapters'][0]['elements']
            self.assertEqual(len(merged), 1)
            self.assertEqual(merged[0]['resolved_md_text'],
                             '<table>\n  <tbody>\n    <tr>\n      <td>A</td>\n'
                             '      <td>B</td>\n    </tr>\n    <tr>\n'
                             '      <td>C</td>\n      <td>D</td>\n    </tr>\n'
                             '  </tbody>\n</table>')
            self.assertEqual(merged[0]['pdf_regions'], [e['pdf_regions'][0] for e in elements])

    def test_formatter_keeps_cell_content_and_references(self):
        text = '<table><tr><td> A  <br>B </td><td>{{text page=1 block=0 chars=0:3}}</td></tr></table>'
        formatted = format_html_table(text)
        self.assertIn('<td> A  <br>B </td>', formatted)
        self.assertIn('<td>{{text page=1 block=0 chars=0:3}}</td>', formatted)
        self.assertEqual(formatted, format_html_table(formatted))
        nested = '<table><tr><td>bits <table><tr><td>0</td></tr></table> end</td></tr></table>'
        self.assertIn('<td>bits <table><tr><td>0</td></tr></table> end</td>',
                      format_html_table(nested))

    def test_different_column_counts_leave_working_unchanged(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            elements = [
                {'id': identifier, 'type': 'table', 'role': None, 'visible': True,
                 'table_format': 'html', 'md_text': text, 'pdf_regions': []}
                for identifier, text in [
                    ('first', '<table><tr><td>A</td><td>B</td></tr></table>'),
                    ('second', '<table><tr><td colspan="3">C</td></tr></table>')]
            ]
            book = {'toc': {'depth': 2}, 'chapters': [{'id': 'chapter', 'title': 'Example',
                    'markdown_file': 'example.md', 'elements': elements}]}
            write_json(root / 'working/book.json', book)
            state = ReviewState(root)
            before = (root / 'working/book.json').read_bytes()
            with self.assertRaisesRegex(ValueError, 'different numbers of columns'):
                state.merge_tables({'id': 'first', 'expected_revision': load_input(root / 'working')[2]})
            self.assertEqual((root / 'working/book.json').read_bytes(), before)


if __name__ == '__main__':
    unittest.main()

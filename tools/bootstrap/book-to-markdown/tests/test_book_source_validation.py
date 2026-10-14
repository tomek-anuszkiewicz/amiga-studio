"""Source provenance checks must distinguish document syntax from literal text."""
import sys
import tempfile
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.model import write_json
from review.source_validation import validate_book
from review.service import ReviewState
from review.model import load_input


class SourceValidationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        write_json(self.root / '02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'Alpha Beta 123'}]})
        self.reference = '{{text page=1 block=0 chars=0:5}}'

    def check(self, text, kind='prose'):
        return validate_book({'chapters': [{'id': 'c', 'title': self.reference,
            'elements': [{'id': 'e', 'type': kind, 'md_text': text}]}]}, self.root)

    def test_markdown_structure_and_reference_splitting_are_allowed(self):
        for text in ('## **' + self.reference + '**',
                     '| ' + self.reference + ' |\n| --- |\n| ' + self.reference + ' |',
                     '```asm\n' + self.reference + '\n```',
                     '> [!NOTE] ' + self.reference + '\n> ' + self.reference):
            with self.subTest(text=text):
                self.assertEqual(self.check(text, 'callout')['issues'], [])

    def test_literal_words_digits_punctuation_and_code_are_reported(self):
        for text in (self.reference + ' invented', self.reference + '123',
                     self.reference + '.', '`invented`', '```asm\ninvented\n```'):
            with self.subTest(text=text):
                self.assertTrue(self.check(text)['issues'])

    def test_html_structure_does_not_hide_literal_cell_or_attribute_text(self):
        text = '<table><tr><th colspan="2">' + self.reference + '</th></tr></table>'
        self.assertEqual(self.check(text, 'table')['issues'], [])
        for text in ('<table><tr><td>invented</td></tr></table>',
                     '<span title="invented">' + self.reference + '</span>',
                     '<CSI>', '<!-- invented -->'):
            with self.subTest(text=text):
                self.assertTrue(self.check(text, 'table')['issues'])

    def test_math_commands_are_structure_but_arguments_are_content(self):
        self.assertEqual(self.check('$\\\\mathrm\\{' + self.reference + '\\}$')['issues'], [])
        for text in ('$\\\\text\\{invented\\}$', '$\\\\madeup\\{' + self.reference + '\\}$'):
            self.assertTrue(self.check(text)['issues'])

    def test_math_command_fragments_are_not_reported_as_literal_words(self):
        write_json(self.root / '02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'mathrm Alpha'}]})
        text = '$\\\\m{{text page=1 block=0 chars=1:3}}hrm\\{{{text page=1 block=0 chars=7:12}}\\}$'
        self.assertEqual(self.check(text)['issues'], [])

    def test_literal_tabs_and_invalid_source_ranges_are_reported(self):
        for text in (self.reference + '\t', '{{text page=1 block=0 chars=0:99}}',
                     '{{text page=2 block=0 chars=0:5}}', '{{text broken}}'):
            with self.subTest(text=text):
                self.assertTrue(self.check(text)['issues'])

    def test_referenced_newlines_and_escaped_punctuation_keep_markdown_structure(self):
        write_json(self.root / '02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'Alpha $FF\n'}]})
        self.assertEqual(self.check('```asm\n{{text page=1 block=0 chars=0:10}}```')['issues'], [])
        self.assertEqual(self.check('\\\\{{text page=1 block=0 chars=6:9}}')['issues'], [])
        self.assertTrue(self.check('`\\\\{{text page=1 block=0 chars=6:9}}`')['issues'])

    def test_chapter_titles_are_checked_and_file_identifiers_are_not_prose(self):
        report = validate_book({'chapters': [{'id': 'c', 'title': 'Invented title',
                                             'markdown_file': 'chapter-001.md', 'elements': []}]}, self.root)
        self.assertEqual({i['field'] for i in report['issues']}, {'title'})

    def test_editor_trusts_literal_baseline_and_manual_edits_without_source_audit(self):
        root = self.root / 'book'
        write_json(root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                   {'blocks': [{'block_id': 0, 'text': 'Alpha'}]})
        book = {'toc': {'depth': 2}, 'chapters': [{'id': 'c', 'title': self.reference,
                'markdown_file': 'chapter.md', 'elements': [{'id': 'e', 'type': 'prose',
                'role': None, 'pdf_regions': [], 'md_text': self.reference}]}]}
        write_json(root / 'working/book.json', book)
        write_json(root / 'accepted/book.json', book)
        accepted = (root / 'accepted/book.json').read_bytes()
        state = ReviewState(root)
        self.assertNotIn('source_validation', state.status())
        state.markdown({'id': 'e', 'text': '**Alpha**',
                        'expected_revision': load_input(root / 'working')[2]})
        self.assertNotIn('source_validation', state.status())
        state.markdown({'id': 'e', 'text': '**Alpha** invented',
                        'expected_revision': load_input(root / 'working')[2]})
        self.assertNotIn('source_validation', state.status())
        self.assertEqual(load_input(root / 'working')[0]['chapters'][0]['elements'][0]['resolved_md_text'],
                         '**Alpha** invented')
        self.assertEqual((root / 'accepted/book.json').read_bytes(), accepted)

"""Role edits must preserve representation, source content and review semantics."""

from copy import deepcopy
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.model import (changes, load_input, set_element_type,
                          write_json)
from review.service import ReviewState


class ElementRoleTest(unittest.TestCase):
    def book(self, *items):
        return {'toc': {'depth': 2}, 'chapters': [{'id': 'chapter', 'title': 'Example',
                'markdown_file': 'example.md', 'elements': list(items)}]}

    def element(self, identifier, kind='prose', role=None):
        return {'id': identifier, 'type': kind, 'role': role, 'md_text': 'Original text',
                'pdf_regions': [{'page': 1, 'bbox': [0, 0, 10, 10]}]}

    def test_current_format_is_required_without_repair(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            old = self.element('text')
            old.pop('role')
            write_json(root / 'working/book.json', self.book(old))
            with self.assertRaises(KeyError):
                load_input(root / 'working')
            write_json(root / 'working/book.json', self.book(self.element('old', 'note', 'note')))
            with self.assertRaisesRegex(ValueError, 'Unsupported element type'):
                load_input(root / 'working')

    def test_role_edit_is_independent_validated_and_revision_checked(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            graphic = self.element('image', 'graphic')
            graphic.pop('md_text')
            graphic['asset'] = 'assets/image.png'
            (root / 'working/assets').mkdir(parents=True)
            (root / 'working/assets/image.png').write_bytes(b'image fixture')
            write_json(root / 'working/book.json', self.book(self.element('text'), self.element('table', 'table'), graphic))
            state = ReviewState(root)
            for identifier in ('text', 'table', 'image'):
                before, _, revision = load_input(root / 'working')
                original = next(e for e in before['chapters'][0]['elements'] if e['id'] == identifier)
                command = {'id': identifier, 'role': 'legend', 'expected_revision': revision}
                state.change_role(command)
                after = load_input(root / 'working')[0]
                updated = next(e for e in after['chapters'][0]['elements'] if e['id'] == identifier)
                self.assertEqual(updated, {**original, 'role': 'legend'})
                with self.assertRaisesRegex(ValueError, 'Stale'):
                    state.change_role(command)
            revision = load_input(root / 'working')[2]
            raw = (root / 'working/book.json').read_bytes()
            with self.assertRaisesRegex(ValueError, 'Unsupported'):
                state.change_role({'id': 'text', 'role': 'anything', 'expected_revision': revision})
            self.assertEqual((root / 'working/book.json').read_bytes(), raw)
            state.change_role({'id': 'text', 'role': None, 'expected_revision': revision})
            text = load_input(root / 'working')[0]['chapters'][0]['elements'][0]
            text['role'] = 'note'
            set_element_type(text, 'heading')
            self.assertEqual(text['role'], 'note')

    def test_split_retains_role_and_different_roles_cannot_merge(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = self.element('first', role='note')
            second = self.element('second', role='legend')
            write_json(root / 'working/book.json', self.book(first, second))
            state = ReviewState(root)
            revision = load_input(root / 'working')[2]
            with self.assertRaisesRegex(ValueError, 'different roles'):
                state.merge({'id': 'first', 'next_id': 'second', 'text': 'Combined', 'expected_revision': revision})
            state.split({'id': 'first', 'first': 'Original', 'second': 'text', 'expected_revision': revision})
            split = load_input(root / 'working')[0]['chapters'][0]['elements']
            self.assertEqual([e['role'] for e in split], ['note', 'note', 'legend'])
            for element in split[:2]:
                element.update(type='table', table_format='markdown', md_text='| A |\n|---|\n| B |')
            split[1]['role'] = 'legend'
            write_json(root / 'working/book.json', self.book(*split))
            revision = load_input(root / 'working')[2]
            with self.assertRaisesRegex(ValueError, 'different roles'):
                state.merge_tables({'id': 'first', 'expected_revision': revision})


if __name__ == '__main__':
    unittest.main()

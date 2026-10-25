"""Moving an element across a chapter boundary transfers its membership."""

from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from review.model import load_input, write_json
from review.service import ReviewState


class MoveElementTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.book = {'toc': {'depth': 2}, 'chapters': [
            self.chapter('first', ['a', 'b']),
            self.chapter('middle', ['c', 'd']),
            self.chapter('last', ['e', 'f']),
        ]}
        write_json(self.root / 'working/book.json', self.book)
        self.state = ReviewState(self.root)

    @staticmethod
    def chapter(identifier, ids):
        return {'id': identifier, 'title': identifier.title(),
                'markdown_file': identifier + '.md', 'elements': [
                    {'id': element_id, 'type': 'prose', 'role': None, 'visible': True,
                     'md_text': element_id, 'pdf_regions': []} for element_id in ids]}

    def move(self, identifier, direction):
        self.state.move({'id': identifier, 'direction': direction,
                         'expected_revision': load_input(self.root / 'working')[2]})
        return load_input(self.root / 'working')[0]['chapters']

    @staticmethod
    def ids(chapters):
        return [[element['id'] for element in chapter['elements']] for chapter in chapters]

    def test_first_element_moves_to_end_of_previous_chapter(self):
        chapters = self.move('c', 'up')
        self.assertEqual(self.ids(chapters), [['a', 'b', 'c'], ['d'], ['e', 'f']])
        self.assertEqual([chapter['id'] for chapter in chapters], ['first', 'middle', 'last'])

    def test_last_element_moves_to_start_of_next_chapter(self):
        chapters = self.move('b', 'down')
        self.assertEqual(self.ids(chapters), [['a'], ['b', 'c', 'd'], ['e', 'f']])

    def test_opposite_boundary_moves_restore_chapter_membership(self):
        self.move('c', 'up')
        chapters = self.move('c', 'down')
        self.assertEqual(self.ids(chapters), [['a', 'b'], ['c', 'd'], ['e', 'f']])
        self.assertEqual([chapter['id'] for chapter in chapters], ['first', 'middle', 'last'])

    def test_moving_only_element_removes_its_chapter(self):
        for direction in ('up', 'down'):
            with self.subTest(direction=direction):
                self.book['chapters'][1]['elements'] = self.chapter('middle', ['c'])['elements']
                write_json(self.root / 'working/book.json', self.book)
                chapters = self.move('c', direction)
                self.assertEqual([chapter['id'] for chapter in chapters], ['first', 'last'])
                expected = [['a', 'b', 'c'], ['e', 'f']] if direction == 'up' else [
                    ['a', 'b'], ['c', 'e', 'f']]
                self.assertEqual(self.ids(chapters), expected)

    def test_move_within_chapter_still_swaps_neighbors(self):
        chapters = self.move('a', 'down')
        self.assertEqual(self.ids(chapters), [['b', 'a'], ['c', 'd'], ['e', 'f']])


if __name__ == '__main__':
    unittest.main()

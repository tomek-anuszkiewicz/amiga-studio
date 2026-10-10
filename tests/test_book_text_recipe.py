"""Source templates must preserve braces, corrections and exact source ranges."""

from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools/bootstrap/book-to-markdown'))
from review.text_recipe import compile_template, compile_region_template, compile_markdown_template, edit_template, slice_template, preserve_bold, resolve_template
from review.model import InputCache, element_markdown, export_markdown, import_book, load_input, set_heading_level, write_json
from review.service import replace_accepted
from review.service import ReviewState
from review.render import Renderer
from markdown_it import MarkdownIt
import pymupdf
from common.pdf_geometry import text_blocks


class TextRecipeTest(unittest.TestCase):
    def test_input_cache_reuses_model_and_detects_external_dependency_changes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            page = root / 'pdf-to-markdown/02_preprocess/page_0001.json'
            write_json(page, {'blocks': [{'block_id': 0, 'text': 'First'}]})
            book = {'toc': {'depth': 2}, 'chapters': [{'id': 'c', 'title': 'Example', 'markdown_file': 'example.md',
                'elements': [{'id': 'p', 'type': 'prose', 'md_text': '{{text page=1 block=0 chars=0:5}}', 'pdf_regions': []},
                             {'id': 'g', 'type': 'graphic', 'asset': 'assets/a.png', 'pdf_regions': []}]}]}
            working = root / 'working'
            (working / 'assets').mkdir(parents=True)
            asset = working / 'assets/a.png'
            asset.write_bytes(b'old asset')
            write_json(working / 'book.json', book)
            cache = InputCache()
            original = cache.load(working)
            self.assertIs(cache.load(working), original)
            changed = {**book, 'toc': {'depth': 3}}
            cache.load(working, changed)
            self.assertEqual(cache.load(working)[0]['toc']['depth'], 2)
            write_json(working / 'book.json', changed)
            self.assertEqual(cache.load(working)[0]['toc']['depth'], 3)
            write_json(page, {'blocks': [{'block_id': 0, 'text': 'Other'}]})
            source_changed = cache.load(working)
            self.assertEqual(source_changed[0]['chapters'][0]['elements'][0]['resolved_md_text'], 'Other')
            asset.write_bytes(b'changed asset')
            self.assertNotEqual(cache.load(working)[2], source_changed[2])
            asset.unlink()
            with self.assertRaises(OSError):
                cache.load(working)

    def test_merge_preserves_both_pages_ranges_and_boxes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            regions = [{'page': 1, 'bbox': [10, 20, 100, 40]}, {'page': 2, 'bbox': [15, 25, 110, 55]}]
            for page in (1, 2):
                write_json(root / f'pdf-to-markdown/02_preprocess/page_{page:04d}.json',
                           {'blocks': [{'block_id': 0, 'text': 'Same'}]})
            first = {'id': 'a', 'type': 'prose', 'md_text': '{{text page=1 block=0 chars=0:4}}', 'pdf_regions': [regions[0]]}
            second = {'id': 'b', 'type': 'prose', 'md_text': '{{text page=2 block=0 chars=0:4}}', 'pdf_regions': regions}
            book = {'toc': {'depth': 2}, 'chapters': [{'id': 'c', 'title': 'Example', 'markdown_file': 'example.md', 'elements': [first, second]}]}
            write_json(root / 'working/book.json', book)
            write_json(root / 'accepted/book.json', book)
            accepted_before = (root / 'accepted/book.json').read_bytes()
            state = ReviewState(root)
            command = {'id': 'a', 'next_id': 'b', 'expected_revision': load_input(root / 'working')[2], 'text': '**Same**\nSame {edited}'}
            before = (root / 'working/book.json').read_bytes()
            for invalid in ({**command, 'next_id': 'wrong'}, {**command, 'text': ''}, {**command, 'expected_revision': 'stale'}):
                with self.assertRaises(ValueError):
                    state.merge(invalid)
                self.assertEqual((root / 'working/book.json').read_bytes(), before)
            state.merge(command)
            result = load_input(root / 'working')[0]['chapters'][0]['elements']
            self.assertEqual(len(result), 1)
            self.assertEqual(result[0]['id'], 'a')
            self.assertEqual(result[0]['resolved_md_text'], command['text'])
            self.assertEqual(result[0]['pdf_regions'], regions)
            self.assertIn('{{text page=1 block=0 chars=0:4}}', result[0]['md_text'])
            self.assertIn('{{text page=2 block=0 chars=0:4}}', result[0]['md_text'])
            self.assertEqual((root / 'accepted/book.json').read_bytes(), accepted_before)
            with self.assertRaisesRegex(ValueError, 'no next'):
                state.merge({**command, 'expected_revision': load_input(root / 'working')[2]})

    def test_edit_and_split_preserve_unicode_ranges_and_repeated_source(self):
        pages = {1: [{'block_id': 0, 'text': 'Same 😀 Same 😀'}]}
        template = '{{text page=1 block=0 chars=0:13}}'
        edited = edit_template(template, pages, '**Same 😀** Same 😀 {manual}')
        self.assertEqual(resolve_template(edited, pages), '**Same 😀** Same 😀 {manual}')
        first = slice_template(edited, 0, 11)
        second = slice_template(edited, 11, len('**Same 😀** Same 😀 {manual}'))
        self.assertEqual(resolve_template(first, pages) + resolve_template(second, pages), '**Same 😀** Same 😀 {manual}')
        self.assertIn('chars=7:13', second)
        self.assertNotIn('chars=0:', second)

    def test_markdown_and_split_are_atomic_and_reject_stale_edits(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_json(root / 'pdf-to-markdown/02_preprocess/page_0001.json',
                       {'blocks': [{'block_id': 0, 'text': 'Same Same'}]})
            element = {'id': 'p', 'type': 'prose', 'md_text': '{{text page=1 block=0 chars=0:9}}', 'pdf_regions': []}
            book = {'toc': {'depth': 2}, 'chapters': [{'id': 'c', 'title': 'Example', 'markdown_file': 'example.md', 'elements': [element]}]}
            write_json(root / 'working/book.json', book)
            state = ReviewState(root)
            command = {'id': 'p', 'expected_revision': load_input(root / 'working')[2]}
            before = (root / 'working/book.json').read_bytes()
            with self.assertRaises(ValueError):
                state.split({**command, 'first': 'Same ', 'second': ''})
            with self.assertRaises(OSError):
                state.markdown({**command, 'text': '![missing](assets/missing.png)'})
            self.assertEqual((root / 'working/book.json').read_bytes(), before)
            state.split({**command, 'first': 'Same ', 'second': 'Same'})
            split = load_input(root / 'working')[0]['chapters'][0]['elements']
            self.assertEqual(split[0]['md_text'], '{{text page=1 block=0 chars=0:5}}')
            self.assertEqual(split[1]['md_text'], '{{text page=1 block=0 chars=5:9}}')
            with self.assertRaisesRegex(ValueError, 'Stale'):
                state.markdown({**command, 'text': 'Old edit'})
            command['expected_revision'] = load_input(root / 'working')[2]
            state.markdown({**command, 'text': '### Same'})
            heading = load_input(root / 'working')[0]['chapters'][0]['elements'][0]
            self.assertEqual((heading['type'], heading['heading_level']), ('heading', 3))

    def test_recipe_reorders_pdf_label_and_preserves_listing_structure(self):
        source = 'Assembler\nADD < ea > ,Dn \nSyntax:\nADD Dn, < ea > \n'
        blocks = [{'block_id': 10, 'text': source}]
        target = '**Assembler Syntax:**\n\n```asm\nADD < ea > ,Dn\nADD Dn, < ea >\n```'
        template = compile_markdown_template(53, blocks, target)
        self.assertEqual(resolve_template(template, {53: blocks}), target)
        self.assertNotIn('Assembler', template)
        self.assertNotIn('Syntax', template)
        self.assertNotIn('ADD', template)
        self.assertIn('```asm', template)
        html = MarkdownIt('commonmark', {'html': True}).render(element_markdown(
            {'type': 'code_block', 'resolved_md_text': target}))
        self.assertIn('<strong>Assembler Syntax:</strong>', html)
        self.assertIn('class="language-asm"', html)
        self.assertNotIn('language-text', html)

    def test_import_preserves_prepared_markdown_without_literal_transcription(self):
        prepared = ['**Operation:** Source + Destination',
                    '```asm\nADD <ea>,Dn\nADD Dn,<ea>\n```',
                    '| X | N |\n|---|---|\n| * | * |', '\\*Word and long only', '**NOTE**']
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / 'pdf-to-markdown'
            status = {s: {'status': 'success'} for s in ('02', '04', '05', '06')}
            # The last table refresh does not restrict the retained page collection.
            status['05']['page_ranges'] = '2'
            write_json(workspace / 'stage_status.json', status)
            source = 'Operation: Source + Destination\nADD <ea>,Dn\nADD Dn,<ea>\nX N\n* *\n*Word and long only\nNOTE\n'
            write_json(workspace / '02_preprocess/page_0001.json', {'page': 1, 'page_id': 'page_0001',
                'raster': {'display_to_pixels': [1, 0, 0, 1, 0, 0]},
                'blocks': [{'block_id': 0, 'text': source, 'bbox': [0, 0, 100, 10],
                            'char_bboxes': [[0, 0, 100, 10]] * len(source)}]})
            segments = [{'segment_id': f'page_0001_seg_{i:03d}', 'type': kind, 'bbox': [0, 0, 100, 10],
                         'md_text': text} for i, (kind, text) in enumerate(zip(
                         ['prose', 'code_block', 'table', 'footnote', 'callout'], prepared), 1)]
            write_json(workspace / '04_filter_page_content/page_0001_segments.json', {'page': 1, 'segments': segments})
            (workspace / '05_transform_page_tables').mkdir()
            previous = {'toc': {'depth': 3}, 'chapters': [{'elements': [
                {'id': 'page_0001_seg_001', 'visible': False}]}]}
            imported = import_book(root, previous)
            self.assertEqual(imported['toc'], previous['toc'])
            self.assertFalse(imported['chapters'][0]['elements'][0]['visible'])
            self.assertEqual([e['resolved_md_text'] for e in imported['chapters'][0]['elements']], prepared)
            raw = (root / 'working/book.json').read_text(encoding='utf-8')
            self.assertNotIn('Operation:', raw)
            self.assertNotIn('Word and long only', raw)
            self.assertTrue(all('{{text ' in e['md_text'] for e in imported['chapters'][0]['elements']))
            self.assertNotIn('{{markdown ', raw)
            loaded, _, _ = load_input(root / 'working')
            self.assertEqual([e['resolved_md_text'] for e in loaded['chapters'][0]['elements']], prepared)
            page_data = {'page': 1, 'blocks': [{'block_id': 0, 'text': source.replace('Source', 'Target')}]}
            write_json(workspace / '02_preprocess/page_0001.json', page_data)
            updated, _, _ = load_input(root / 'working')
            self.assertEqual(updated['chapters'][0]['elements'][0]['resolved_md_text'], '**Operation:** Target + Destination')

    def test_prepared_reference_is_single_pass_and_missing_segments_fail(self):
        template = '{{markdown stage=05 page=1 segment=page_0001_seg_001}}'
        source = '**NOTE** {{text page=9 block=0 chars=0:4}}'
        prepared = {('05', 1, 'page_0001_seg_001'): source}
        self.assertEqual(resolve_template(template, {}, prepared), source)
        with self.assertRaisesRegex(ValueError, 'missing prepared segment'):
            resolve_template(template, {}, {})

    def test_region_recipe_preserves_bold_without_replacing_pdf_text(self):
        text = 'Description: The size may be speciﬁed as word or\nlong.\n'
        blocks = [{'block_id': 12, 'text': text, 'bbox': [0, 0, 100, 10],
                   'char_bboxes': [[0, 0, 100, 10]] * len(text)}]
        page = {'blocks': blocks, 'raster': {'display_to_pixels': [1, 0, 0, 1, 0, 0]}}
        template = compile_region_template(56, page, [0, 0, 100, 10],
            markdown='**Description:** The size may be specified as word or long.')
        resolved = resolve_template(template, {56: blocks})
        self.assertEqual(resolved, '**Description:**' + text[len('Description:'):])
        self.assertNotIn('Description', template)
        self.assertIn('chars=0:12', template)

    def test_bold_spans_cross_blocks_and_follow_matching_context(self):
        blocks = [{'block_id': 1, 'text': 'Same label. Bold\n'},
                  {'block_id': 2, 'text': 'across blocks. Same label.\n'}]
        template = preserve_bold(compile_template(1, blocks), {1: blocks},
                                'Same label. **Bold across blocks.** __Same label.__')
        self.assertEqual(resolve_template(template, {1: blocks}),
                         'Same label. **Bold\nacross blocks.** **Same label.**\n')

    def test_adjacent_elements_split_a_shared_pdf_block(self):
        heading, prose = 'ADDA\nAdd Address\n', 'Effective Address field\n'
        text = heading + prose
        boxes = [[0, 0, 100, 10]] * len(heading) + [[0, 20, 100, 30]] * len(prose)
        blocks = [{'block_id': 8, 'text': text, 'bbox': [0, 0, 100, 30],
                   'char_bboxes': boxes}]
        page = {'blocks': blocks, 'raster': {'display_to_pixels': [1, 0, 0, 1, 0, 0]}}
        results = [resolve_template(compile_region_template(57, page, box), {57: blocks})
                   for box in ([0, 0, 100, 15], [0, 15, 100, 35])]
        self.assertEqual(results, [heading, prose])
        self.assertEqual(''.join(results), text)

    def test_character_ranges_follow_raster_transform_and_column_boundary(self):
        blocks = [{'block_id': 0, 'text': 'AB\n', 'bbox': [0, 0, 20, 10],
                   'char_bboxes': [[0, 0, 10, 10], [10, 0, 20, 10], [10, 0, 20, 10]]}]
        page = {'blocks': blocks, 'raster': {'display_to_pixels': [2, 0, 0, 2, -3, -4]}}
        left = compile_region_template(1, page, [-3, -4, 17, 16])
        right = compile_region_template(1, page, [17, -4, 37, 16])
        self.assertEqual(resolve_template(left, {1: blocks}), 'A')
        self.assertEqual(resolve_template(right, {1: blocks}), 'B\n')
        del blocks[0]['char_bboxes']
        with self.assertRaisesRegex(ValueError, 'regenerate Stage 02'):
            compile_region_template(1, page, [-3, -4, 17, 16])

    def test_character_extraction_preserves_block_identity_and_text(self):
        with pymupdf.open() as document:
            page = document.new_page()
            page.insert_text((50, 50), 'ADDA\nAdd Address\nEffective Address field')
            page.set_rotation(90)
            original = text_blocks(page)
            detailed = text_blocks(page, character_geometry=True)
            self.assertTrue(detailed)
            for block in detailed:
                self.assertEqual(len(block['char_bboxes']), len(block['text']))
            self.assertEqual([{k: v for k, v in b.items() if k != 'char_bboxes'}
                              for b in detailed], original)

    def test_tight_region_preserves_trailing_space_and_newline(self):
        blocks = [{'block_id': 0, 'text': 'A \nB\n', 'bbox': [0, 0, 20, 30],
                   'char_bboxes': [[0, 0, 10, 10], [10, 0, 20, 10], [10, 0, 20, 10],
                                   [0, 20, 10, 30], [0, 20, 10, 30]]}]
        page = {'blocks': blocks, 'raster': {'display_to_pixels': [1, 0, 0, 1, 0, 0]}}
        template = compile_region_template(1, page, [0, 0, 10, 15])
        self.assertEqual(resolve_template(template, {1: blocks}), 'A \n')

    def test_line_break_formatting_preserves_existing_markdown_structure(self):
        parser = MarkdownIt('commonmark', {'html': True}).enable('table')
        cases = [
            ('heading', '# SECTION 1 \nOVERVIEW \n'),
            ('prose', '- First\n- Second\n'),
            ('table', '| X | N |\n| --- | --- |\n| carry | undefined |\n'),
            ('code_block', '```text\nX  N\n1  0\n```\n'),
            ('prose', '`first\nsecond`\n'),
            ('prose', 'First  \nSecond\\\nThird\n'),
            ('prose', '$$\na + b\nc + d\n$$\n'),
            ('prose', '<table>\n<tr><td>X</td></tr>\n</table>\n'),
        ]
        for kind, text in cases:
            with self.subTest(kind=kind, text=text):
                element = {'type': kind, 'resolved_md_text': text, 'heading_level': 1}
                self.assertEqual(parser.render(element_markdown(element)), parser.render(text))

    def test_source_lines_remain_visible_in_preview_and_export(self):
        source = "X — Set the same as the carry bit. \nN — Undefined. \nZ — Unchanged if zero. \n"
        element = {"id": "flags", "type": "table_legend", "resolved_md_text": source}
        markdown = element_markdown(element)
        parser = MarkdownIt('commonmark', {'html': True})
        self.assertEqual(parser.render(markdown).count('<br'), 2)
        self.assertEqual(element['resolved_md_text'], source)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            book = {'toc': {'depth': 2}, 'chapters': [
                {'id': 'c', 'title': 'Flags', 'markdown_file': 'flags.md', 'elements': [element]}]}
            export_markdown(book, root)
            self.assertEqual(parser.render((root / 'flags.md').read_text(encoding='utf-8')).count('<br'), 2)
            renderer = Renderer()
            try:
                snapshot = renderer.render(root, book, {}, 'line-break-regression')
                document = root / snapshot['render']['chapters'][1]['document']
                self.assertEqual(document.read_text(encoding='utf-8').count('<br'), 2)
            finally:
                renderer.close()

    def test_source_and_literal_braces_are_never_executed(self):
        blocks = [{"block_id": 4, "text": r"PDF {{text page=9 block=0 chars=0:4}} {x} \\ end"}]
        template = compile_template(1, blocks)
        self.assertEqual(resolve_template(template, {1: blocks}), blocks[0]['text'])

    def test_multiple_blocks_preserve_ocr_and_whitespace_exactly(self):
        blocks = [{"block_id": 1, "text": "M68000 \n"},
                  {"block_id": 2, "text": "MC68HCOOO \nUser's Manual \n"}]
        template = compile_template(3, blocks)
        self.assertIn('block=1', template)
        self.assertIn('block=2', template)
        self.assertEqual(resolve_template(template, {3: blocks}), ''.join(b['text'] for b in blocks))
        self.assertEqual(template, '{{text page=3 block=1 chars=0:8}}{{text page=3 block=2 chars=0:26}}')

    def test_bad_reference(self):
        blocks = [{"block_id": 1, "text": "abc"}]
        with self.assertRaises(ValueError):
            resolve_template('{{text page=1 block=1 chars=0:9}}', {1: blocks})
        with self.assertRaises(ValueError):
            resolve_template('{{unknown}}', {1: blocks})

    def test_render_accept_edit_and_changed_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            blocks = [{"block_id": 1, "text": "Heading {x}\n"}]
            template = '# ' + compile_template(1, blocks)
            page_path = root / 'pdf-to-markdown/02_preprocess/page_0001.json'
            write_json(page_path, {'blocks': blocks})
            metadata_page = root / 'pdf-to-markdown/02_preprocess/page_0002.json'
            write_json(metadata_page, {'blocks': [{'block_id': 0, 'text': 'chapter'}]})
            name_template = '{{text page=2 block=0 chars=0:7}}'
            book = {'toc': {'depth': 2},
                    'chapters': [{'id': 'c', 'title': 'Title ' + name_template,
                    'markdown_file': name_template + '.md',
                    'elements': [{'id': 'h', 'type': 'heading', 'heading_level': 1,
                    'pdf_regions': [], 'md_text': template}]}]}
            write_json(root / 'working/book.json', book)
            loaded, assets, fingerprint = load_input(root / 'working')
            renderer = Renderer()
            try:
                snapshot = renderer.render(root / 'working', loaded, assets, fingerprint, None)
                replace_accepted(root, snapshot)
            finally:
                renderer.close()
            accepted, _, _ = load_input(root / 'accepted')
            self.assertEqual(accepted['chapters'][0]['resolved_title'], 'Title chapter')
            self.assertEqual(accepted['chapters'][0]['resolved_markdown_file'], 'chapter.md')
            export_markdown(accepted, root / 'accepted')
            self.assertTrue((root / 'accepted/chapter.md').exists())
            self.assertIn('(chapter.md#element-h)', (root / 'accepted/table-of-contents.md').read_text(encoding='utf-8'))
            self.assertEqual(accepted['chapters'][0]['elements'][0]['resolved_md_text'], '# Heading {x}\n')
            raw = (root / 'accepted/book.json').read_text(encoding='utf-8')
            self.assertNotIn('resolved_md_text', raw)
            self.assertNotIn('resolved_title', raw)
            self.assertNotIn('resolved_markdown_file', raw)
            self.assertIn(name_template, raw)
            self.assertNotIn('text_source', raw)
            self.assertNotIn('md_text_format', raw)
            heading = loaded['chapters'][0]['elements'][0]
            set_heading_level(heading, 3)
            write_json(root / 'working/book.json', loaded)
            loaded, _, _ = load_input(root / 'working')
            self.assertEqual(element_markdown(loaded['chapters'][0]['elements'][0]), '### Heading {x}\n')
            write_json(metadata_page, {'blocks': [{'block_id': 0, 'text': '../oops'}]})
            with self.assertRaisesRegex(ValueError, 'bundle-local'):
                load_input(root / 'working')
            write_json(metadata_page, {'blocks': [{'block_id': 0, 'text': 'chapter'}]})
            write_json(page_path, {'blocks': [{'block_id': 1, 'text': 'Changed'}]})
            with self.assertRaisesRegex(ValueError, 'range is outside'):
                load_input(root / 'working')


if __name__ == '__main__':
    unittest.main()

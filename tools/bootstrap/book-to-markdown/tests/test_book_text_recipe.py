"""Source templates must preserve braces, corrections and exact source ranges."""

from pathlib import Path
import importlib.util
import json
import re
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[4] / 'tools/bootstrap/book-to-markdown'))
from review.text_recipe import REFERENCE, compile_region_template, edit_template, join_adjacent_references, slice_template, preserve_bold, resolve_template, source_differences, source_reference, source_span_with_spacing


from review.markdown import callout_plugin
from review.model import InputCache, element_markdown, export_markdown, import_book, load_input, read_json, set_heading_level, toc_markdown, write_json
from review.service import replace_accepted
from review.service import ReviewState
from review.source_cleanup import preview, reconcile_working
from review.render import Renderer
from markdown_it import MarkdownIt
import pymupdf


class ReorderedReferenceTests(unittest.TestCase):
    def test_reordered_text_keeps_source_references(self):
        pages = {1: [{'block_id': 0, 'text': 'One'}, {'block_id': 1, 'text': 'Two'}]}
        template = '{{text page=1 block=0 chars=0:3}} {{text page=1 block=1 chars=0:3}}'
        result = edit_template(template, pages, 'Two One', allow_reorder=True)
        self.assertEqual(resolve_template(result, pages), 'Two One')
        self.assertIn('{{text page=1 block=1 chars=0:3}}', result)
        self.assertIn('{{text page=1 block=0 chars=0:3}}', result)
from common.pdf_geometry import text_blocks


class TextRecipeTest(unittest.TestCase):
    def test_source_span_uses_case_for_case_only_changes(self):
        block = {'block_id': 2, 'text': 'bit EXAMPLE res LOAD nd'}
        pages = {93: [block]}
        for start, end, visible, mode in ((0, 3, 'BIT', 'upper'),
                                           (4, 11, 'example', 'lower'),
                                           (12, 15, 'RES', 'upper'),
                                           (16, 20, 'load', 'lower')):
            with self.subTest(visible=visible):
                template = source_span_with_spacing(93, block, start, end, visible)
                self.assertEqual(template, source_reference(93, 2, start, end, case=mode))
                self.assertEqual(resolve_template(template, pages), visible)
                self.assertEqual(source_differences(template, pages), [])
        substantive = source_span_with_spacing(93, block, 21, 23, 'AND')
        self.assertNotIn(' case=', substantive)
        self.assertTrue(source_differences(substantive, pages))
        for source, visible, mode in (('r e s', 'RES', 'upper'),
                                      ('L O A D', 'load', 'lower')):
            with self.subTest(source=source):
                spaced = {'block_id': 3, 'text': source}
                template = source_span_with_spacing(93, spaced, 0, len(source), visible)
                self.assertEqual(resolve_template(template, {93: [spaced]}), visible)
                self.assertEqual(source_differences(template, {93: [spaced]}), [])
                self.assertEqual(template.count(f' case="{mode}"'), len(visible))
                self.assertNotIn(' replace=', template)

    def test_case_reference_preserves_source_and_has_no_correction_marker(self):
        blocks = [{'block_id': 2, 'text': 'BIT example AND'}]
        pages = {93: blocks}
        lower = source_reference(93, 2, 0, 3, case='lower')
        upper = source_reference(93, 2, 4, 11, case='upper')
        narrowed = source_reference(93, 2, 13, 15, case='lower')
        template = f'{lower} {upper} {narrowed}'
        self.assertEqual(resolve_template(template, pages), 'bit EXAMPLE nd')
        self.assertEqual(source_differences(template, pages), [])
        self.assertEqual(slice_template(upper, 2, 5, pages),
                         source_reference(93, 2, 6, 9, case='upper'))
        self.assertEqual(resolve_template(slice_template(upper, 2, 5, pages), pages), 'AMP')
        self.assertEqual(resolve_template(edit_template(lower, pages, '**bit**'), pages),
                         '**bit**')
        adjacent = lower + source_reference(93, 2, 3, 4)
        self.assertEqual(join_adjacent_references(adjacent, Path('.')), adjacent)
        self.assertEqual(source_differences(
            source_reference(93, 2, 0, 3, (0, 3, 'bit')), pages)[0]['kind'],
            'correction')

    def test_case_reference_rejects_ambiguous_or_length_changing_transformations(self):
        pages = {1: [{'block_id': 0, 'text': 'Straße ΟΣ'}]}
        with self.assertRaises(ValueError):
            resolve_template(source_reference(1, 0, 0, 6, case='upper'), pages)
        with self.assertRaises(ValueError):
            resolve_template(source_reference(1, 0, 7, 9, case='lower'), pages)
        with self.assertRaises(ValueError):
            resolve_template('{{text page=1 block=0 chars=0:3 case="UPPER|LOWER"}}', pages)


    def test_source_overlay_ignores_paragraph_reflow_and_marks_only_ocr_character(self):
        source = 'A system implementation based on an 8-bit data bus reduces cost in comparison to 16-\nbit systems.'
        visible = 'A system implementation based on an 8-bit data bus reduces cost in comparison to 16-bit systems.'
        blocks = {1: [{'block_id': 0, 'text': source}]}
        template = source_reference(1, 0, 0, len(source), (0, len(source), visible))
        self.assertEqual(resolve_template(template, blocks), visible)
        self.assertEqual(source_differences(template, blocks), [])

        source = 'The MC68ECOOO and the New model.\n'
        visible = 'The MC68EC000 and the New model.'
        blocks = {1: [{'block_id': 0, 'text': source}]}
        template = source_reference(1, 0, 0, len(source), (0, len(source), visible))
        marks = source_differences(template, blocks)
        self.assertEqual([(mark['text'], mark['source_text']) for mark in marks], [('000', 'OOO')])


    def test_accepted_literal_is_not_a_pending_cleanup_finding(self):
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            write_json(workspace / '02_preprocess/page_0001.json',
                       {'blocks': [{'block_id': 0, 'text': 'Source'}]})
            book = {'chapters': [{'id': 'c', 'title': 'Source', 'elements': [
                {'id': 'e', 'type': 'prose', 'role': None,
                 'md_text': '{{text page=1 block=0 chars=0:6}} '
                            '{{accepted text="Vision"}} Raw',
                 'pdf_regions': [{'page': 1, 'bbox': [0, 0, 10, 10]}]}]}]}
            findings = [item for item in preview(book, workspace) if item['element_id'] == 'e']
            self.assertEqual([item['literal'] for item in findings], ['Raw'])


    def test_reconciliation_does_not_take_an_earlier_exact_word_for_a_local_typo(self):
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            source = 'information and detailed infonnation on'
            write_json(workspace / '02_preprocess/page_0001.json',
                       {'blocks': [{'block_id': 0, 'text': source}]})
            template = ('{{text page=1 block=0 chars=16:24}} '
                        '{{accepted text="information"}} '
                        '{{text page=1 block=0 chars=37:39}}')
            book = {'toc': {'depth': 2}, 'chapters': [{'id': 'c', 'title': 'Example',
                'markdown_file': 'example.md', 'elements': [{'id': 'p', 'type': 'prose',
                'md_text': template, 'pdf_regions': [{'page': 1, 'bbox': [0, 0, 10, 10]}],
                'role': None}]}]}
            reconcile_working(book, {'chapters': []}, workspace)
            result = book['chapters'][0]['elements'][0]['md_text']
            self.assertIn('{{accepted text="information"}}', result)
            self.assertNotIn('chars=0:11', result)

    def test_saved_working_edit_keeps_unmatched_letter_explicit_without_accepting_book(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_json(root / 'pdf-to-markdown/02_preprocess/page_0023.json',
                       {'blocks': [{'block_id': 6, 'text': 'ADDO'}]})
            book = {'toc': {'depth': 2}, 'chapters': [{'id': 'c', 'title': 'Example',
                'markdown_file': 'example.md', 'elements': [{'id': 'p', 'type': 'prose',
                'md_text': '{{text page=23 block=6 chars=0:4}}',
                'pdf_regions': [{'page': 23, 'bbox': [0, 0, 10, 10]}], 'role': None}]}]}
            write_json(root / 'working/book.json', book)
            write_json(root / 'accepted/book.json', book)
            accepted_before = (root / 'accepted/book.json').read_bytes()
            state = ReviewState(root)
            state.markdown({'id': 'p', 'expected_revision': load_input(root / 'working')[2],
                            'text': 'ADDQ'})
            saved = load_input(root / 'working')[0]['chapters'][0]['elements'][0]
            self.assertEqual(saved['resolved_md_text'], 'ADDQ')
            self.assertIn('chars=0:3', saved['md_text'])
            self.assertIn('{{accepted text="Q"}}', saved['md_text'])
            self.assertEqual(saved['source_differences'][0]['kind'], 'unanchored')
            self.assertEqual((root / 'accepted/book.json').read_bytes(), accepted_before)


    def test_chapter_display_title_tracks_first_visible_contents_entry(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            book = {'toc': {'depth': 2}, 'chapters': [{'id': 'c', 'title': 'Original cover',
                'markdown_file': 'chapter.md', 'elements': [
                    {'id': 'cover', 'type': 'heading', 'heading_level': 1,
                     'md_text': '# Original cover', 'visible': False, 'role': None, 'pdf_regions': []},
                    {'id': 'section', 'type': 'heading', 'heading_level': 1,
                     'md_text': '# **SECTION 1 OVERVIEW**', 'role': None, 'pdf_regions': []},
                    {'id': 'detail', 'type': 'heading', 'heading_level': 2,
                     'md_text': '## 1.1 Details', 'role': None, 'pdf_regions': []}]}]}
            write_json(root / 'working/book.json', book)
            loaded, _, _ = load_input(root / 'working')
            self.assertEqual(loaded['chapters'][0]['toc_title'], 'SECTION 1 OVERVIEW')
            self.assertEqual(loaded['chapters'][0]['resolved_title'], 'Original cover')
            book['chapters'][0]['elements'][1]['visible'] = False
            write_json(root / 'working/book.json', book)
            loaded, _, _ = load_input(root / 'working')
            self.assertEqual(loaded['chapters'][0]['toc_title'], '1.1 Details')
            book['toc']['depth'] = 1
            write_json(root / 'working/book.json', book)
            loaded, _, _ = load_input(root / 'working')
            self.assertIsNone(loaded['chapters'][0]['toc_title'])
            self.assertNotIn('toc_title', (root / 'working/book.json').read_text(encoding='utf-8'))

    def test_generated_contents_use_plain_heading_labels(self):
        book = {'toc': {'depth': 2}, 'chapters': [{'id': 'c', 'title': 'Example',
            'markdown_file': 'example.md', 'elements': [{'id': 'h', 'type': 'heading',
            'heading_level': 2, 'md_text': '## **INITIALIZE COMM PORT** (AH = 00H)',
            'pdf_regions': [], 'role': None}]}]}
        self.assertIn('[INITIALIZE COMM PORT (AH = 00H)]', toc_markdown(book))
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            renderer = Renderer()
            try:
                snapshot = renderer.render(root, book, {}, 'plain-contents-label')
                contents = root / snapshot['render']['chapters'][0]['document']
                self.assertIn('>INITIALIZE COMM PORT (AH = 00H)</a>',
                              contents.read_text(encoding='utf-8'))
            finally:
                renderer.close()

    def test_input_cache_reuses_model_and_detects_external_dependency_changes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            page = root / 'pdf-to-markdown/02_preprocess/page_0001.json'
            write_json(page, {'blocks': [{'block_id': 0, 'text': 'First'}]})
            book = {'toc': {'depth': 2}, 'chapters': [{'id': 'c', 'title': 'Example', 'markdown_file': 'example.md',
                'elements': [{'id': 'p', 'type': 'prose', 'md_text': '{{text page=1 block=0 chars=0:5}}', 'pdf_regions': [], 'role': None},
                             {'id': 'g', 'type': 'graphic', 'asset': 'assets/a.png', 'pdf_regions': [], 'role': None}]}]}
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
            first = {'id': 'a', 'type': 'prose', 'md_text': '{{text page=1 block=0 chars=0:4}}', 'pdf_regions': [regions[0]], 'role': None}
            second = {'id': 'b', 'type': 'prose', 'md_text': '{{text page=2 block=0 chars=0:4}}', 'pdf_regions': regions, 'role': None}
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
            element = {'id': 'p', 'type': 'prose', 'md_text': '{{text page=1 block=0 chars=0:9}}', 'pdf_regions': [], 'role': None}
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


    def test_import_preserves_prepared_markdown_without_literal_transcription(self):
        prepared = ['**Operation:** Source + Dest ination',
                    '```asm\nADD <ea>,Dn\nADD Dn,<ea>\n```',
                    '| X | N |\n|---|---|\n| * | * |', '\\*Word and long only', 'NOTE', 'Read carefully.']
        expected = ['**Operation:** Source + Destination'] + prepared[1:-2] + ['> [!NOTE] NOTE\n>\n> Read carefully.']
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / 'pdf-to-markdown'
            status = {s: {'status': 'success'} for s in ('02', '04', '05', '06', '07', '08')}
            # The last table refresh does not restrict the retained page collection.
            status['05']['page_ranges'] = '2'
            write_json(workspace / 'stage_status.json', status)
            source = 'Operation: Source + Dest ination\nADD <ea>,Dn\nADD Dn,<ea>\nX N\n* *\n*Word and long only\nNOTE\nRead carefully.\n'
            write_json(workspace / '02_preprocess/page_0001.json', {'page': 1, 'page_id': 'page_0001',
                'raster': {'display_to_pixels': [1, 0, 0, 1, 0, 0]},
                'blocks': [{'block_id': 0, 'text': source, 'bbox': [0, 0, 100, 10],
                            'char_bboxes': [[0, 0, 100, 10]] * len(source)}]})
            segments = [{'segment_id': f'page_0001_seg_{i:03d}', 'type': kind, 'bbox': [0, 0, 100, 10],
                         'md_text': text, 'role': None} for i, (kind, text) in enumerate(zip(
                         ['prose', 'code_block', 'table', 'prose', 'callout', 'callout_text'], prepared), 1)]
            segments[2]['role'] = 'legend'
            segments[3]['role'] = 'footnote'
            write_json(workspace / '04_filter_page_content/page_0001_segments.json', {'page': 1, 'segments': segments})
            (workspace / '05_transform_page_tables').mkdir()
            stage_path = Path(__file__).resolve().parents[4] / 'tools/bootstrap/pdf-to-markdown/stages/07_repair_callouts/repair_callouts.py'
            spec = importlib.util.spec_from_file_location('repair_callouts', stage_path)
            stage = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(stage)
            stage.repair_callouts(workspace)
            final = read_json(workspace / '07_repair_callouts/page_0001_segments.json')
            final['segments'][0]['md_text'] = '**Operation:** Source + Destination'
            write_json(workspace / '08_proofreading/page_0001_segments.json', final)
            write_json(root / 'working/book.json', {'toc': {'depth': 3}, 'chapters': []})
            obsolete = ['working/old-chapter.md', 'working/assets/old.png',
                        'working/renders/old/document.html', 'accepted/book.json',
                        'accepted/assets/old.png', 'pdf/pages/page_9999.png']
            for relative in obsolete:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('obsolete output', encoding='utf-8')
            status_without_proofreading = dict(status)
            status_without_proofreading.pop('08')
            write_json(workspace / 'stage_status.json', status_without_proofreading)
            with self.assertRaisesRegex(ValueError, 'Preparation stage 08 is not successful'):
                import_book(root)
            self.assertTrue((root / 'working/book.json').exists())
            write_json(workspace / 'stage_status.json', status)
            imported = import_book(root)
            for relative in obsolete:
                self.assertFalse((root / relative).exists(), relative)
            self.assertFalse((root / 'accepted').exists())
            self.assertEqual(imported['toc'], {'depth': 2})
            self.assertTrue(imported['chapters'][0]['elements'][0]['visible'])
            self.assertEqual([e['resolved_md_text'] for e in imported['chapters'][0]['elements']], expected)
            self.assertEqual(imported['chapters'][0]['elements'][-1]['id'], 'page_0001_seg_005')
            self.assertEqual([e['role'] for e in imported['chapters'][0]['elements']],
                             [None, None, 'legend', 'footnote', None])
            raw = (root / 'working/book.json').read_text(encoding='utf-8')
            self.assertNotIn('Operation:', raw)
            self.assertNotIn('Word and long only', raw)
            self.assertTrue(all('{{text ' in e['md_text'] for e in imported['chapters'][0]['elements']))
            self.assertNotIn('{{markdown ', raw)
            loaded, _, _ = load_input(root / 'working')
            self.assertEqual([e['resolved_md_text'] for e in loaded['chapters'][0]['elements']], expected)
            page_data = {'page': 1, 'blocks': [{'block_id': 0, 'text': source.replace('Source', 'Target')}]}
            write_json(workspace / '02_preprocess/page_0001.json', page_data)
            updated, _, _ = load_input(root / 'working')
            self.assertEqual(updated['chapters'][0]['elements'][0]['resolved_md_text'], '**Operation:** Target + Destination')

    def test_import_anchors_letter_spaced_notes_sharing_one_source_block(self):
        first_source = ('n o t e Is A 5 0 0 a n d A 2 0 0 0 k e y b o a r d s '
                        'o n l y ( n u m e r i c p a d ) \n')
        second_source = ('n o t e 2: I n t e r n a t i o n a l k e y b o a r d s '
                         'o n l y (the s e k e y s a r e c u t o u t s o f t h e \n')
        continuation = ('l a r g e r k e y o n t h e U S A S C I I v e r s i o n . ) \n'
                        'T h e k e y t h a t g e n e r a t e s \n')
        ending = ('$ 3 0 is c u t o u t o f t h e l e f t s h i f t key. \n'
                  'K e y $ 2 B is c u t o u t o f \n'
                  'r e t u r n . \nT h e s e k e y s a r e l a b e l e d w i t h '
                  'c o u n t r y - s p e c i f i c m a r k i n g s , \n')
        first = 'note 1: A500 and A2000 keyboards only (numeric pad)'
        second = ('note 2: International keyboards only (these keys are cutouts of the larger '
                  'key on the US ASCII version.) The key that generates `$30` is cut out of '
                  'the left shift key. Key `$2B` is cut out of return. These keys are labeled '
                  'with country-specific markings.')
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / 'pdf-to-markdown'
            write_json(workspace / 'stage_status.json',
                       {stage: {'status': 'success'} for stage in ('02', '06', '07', '08')})
            blocks = [
                {'block_id': 15, 'text': first_source + second_source,
                 'bbox': [0, 0, 100, 30],
                 'char_bboxes': ([[0, 0, 100, 10]] * len(first_source) +
                                 [[0, 20, 100, 30]] * len(second_source))},
                {'block_id': 16, 'text': continuation, 'bbox': [0, 40, 100, 50],
                 'char_bboxes': [[0, 40, 100, 50]] * len(continuation)},
                {'block_id': 17, 'text': ending, 'bbox': [0, 60, 100, 70],
                 'char_bboxes': [[0, 60, 100, 70]] * len(ending)},
            ]
            write_json(workspace / '02_preprocess/page_0001.json',
                       {'page': 1, 'page_id': 'page_0001', 'blocks': blocks,
                        'raster': {'display_to_pixels': [1, 0, 0, 1, 0, 0]}})
            segments = [
                {'segment_id': 'first', 'type': 'prose', 'role': 'footnote',
                 'bbox': [0, 0, 100, 10], 'md_text': first},
                {'segment_id': 'second', 'type': 'prose', 'role': 'footnote',
                 'bbox': [0, 20, 100, 70], 'md_text': second},
            ]
            write_json(workspace / '08_proofreading/page_0001_segments.json',
                       {'page': 1, 'page_id': 'page_0001', 'segments': segments})
            imported = import_book(root)
            elements = imported['chapters'][0]['elements']
            self.assertEqual([element['resolved_md_text'] for element in elements], [first, second])
            for element in elements:
                self.assertTrue(all(int(ref[1]) == 1 for ref in REFERENCE.finditer(element['md_text'])))
            self.assertIn('{{accepted text="1"}}', elements[0]['md_text'])
            self.assertIn('block=15 chars=', elements[0]['md_text'])
            self.assertIn('block=16 chars=', elements[1]['md_text'])
            self.assertIn('block=17 chars=', elements[1]['md_text'])
            self.assertEqual(resolve_template(elements[0]['md_text'], {1: blocks}), first)
            self.assertEqual(resolve_template(elements[1]['md_text'], {1: blocks}), second)

    def test_import_anchors_repeated_spaced_table_labels_in_source_order(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / 'pdf-to-markdown'
            write_json(workspace / 'stage_status.json',
                       {stage: {'status': 'success'} for stage in ('02', '06', '07', '08')})
            source = '| s p a r e | s p a r e | C U R S | C U R S | F 1 0 |\n'
            block = {'block_id': 0, 'text': source, 'bbox': [0, 0, 100, 10],
                     'char_bboxes': [[0, 0, 100, 10]] * len(source)}
            write_json(workspace / '02_preprocess/page_0001.json',
                       {'page': 1, 'page_id': 'page_0001', 'blocks': [block],
                        'raster': {'display_to_pixels': [1, 0, 0, 1, 0, 0]}})
            target = '| spare | spare | CURS | CURS | F10 |'
            write_json(workspace / '08_proofreading/page_0001_segments.json',
                       {'page': 1, 'page_id': 'page_0001', 'segments': [
                           {'segment_id': 'table', 'type': 'table', 'role': None,
                            'bbox': [0, 0, 100, 10], 'md_text': target}]})
            element = import_book(root)['chapters'][0]['elements'][0]
            self.assertEqual(element['resolved_md_text'], target)
            self.assertNotIn('{{accepted text=', element['md_text'])
            self.assertEqual(resolve_template(element['md_text'], {1: [block]}), target)

    def test_obsolete_prepared_reference_is_not_interpreted(self):
        with self.assertRaisesRegex(ValueError, 'Invalid text reference'):
            resolve_template('{{markdown stage=05 page=1 segment=page_0001_seg_001}}', {})

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
        source = source_reference(1, 1, 0, len(blocks[0]['text'])) + source_reference(1, 2, 0, len(blocks[1]['text']))
        template = preserve_bold(source, {1: blocks},
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
                element = {'type': kind, 'resolved_md_text': text, 'heading_level': 1, 'role': None}
                self.assertEqual(parser.render(element_markdown(element)), parser.render(text))

    def test_source_lines_remain_visible_in_preview_and_export(self):
        source = "X — Set the same as the carry bit. \nN — Undefined. \nZ — Unchanged if zero. \n"
        element = {"id": "flags", "type": "prose", "role": "legend", "resolved_md_text": source}
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

    def test_note_role_stays_plain_while_callout_is_highlighted(self):
        source = ('Notes:\n1. First qualification.\n'
                  '2. Check $\\overline{\\mathrm{AS}}$.')
        labeled = {'id': 'labeled', 'type': 'prose', 'role': 'note', 'md_text': source}
        callout_source = '> [!NOTE] NOTE\n>\n> A source advisory.'
        callout = {'id': 'callout', 'type': 'callout', 'role': None, 'md_text': callout_source}
        self.assertEqual(element_markdown(labeled), source)
        self.assertEqual(element_markdown(callout), callout_source)
        parser = MarkdownIt('commonmark', {'html': True}).use(callout_plugin)
        self.assertNotIn('<blockquote', parser.render(element_markdown(labeled)))
        self.assertIn('class="markdown-callout"', parser.render(element_markdown(callout)))
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            book = {'toc': {'depth': 2}, 'chapters': [
                {'id': 'c', 'title': 'Notes', 'markdown_file': 'notes.md',
                 'elements': [labeled, callout]}]}
            export_markdown(book, root)
            exported = (root / 'notes.md').read_text(encoding='utf-8')
            self.assertIn(source, exported)
            self.assertIn(callout_source, exported)
        self.assertEqual(labeled['md_text'], source)
        self.assertEqual(callout['md_text'], callout_source)

    def test_source_and_literal_braces_are_never_executed(self):
        blocks = [{"block_id": 4, "text": r"PDF {{text page=9 block=0 chars=0:4}} {x} \\ end"}]
        template = source_reference(1, 4, 0, len(blocks[0]['text']))
        self.assertEqual(resolve_template(template, {1: blocks}), blocks[0]['text'])

    def test_multiple_blocks_preserve_ocr_and_whitespace_exactly(self):
        blocks = [{"block_id": 1, "text": "M68000 \n"},
                  {"block_id": 2, "text": "MC68HCOOO \nUser's Manual \n"}]
        template = ''.join(source_reference(3, block['block_id'], 0, len(block['text'])) for block in blocks)
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
            template = '# ' + source_reference(1, 1, 0, len(blocks[0]['text']))
            page_path = root / 'pdf-to-markdown/02_preprocess/page_0001.json'
            write_json(page_path, {'blocks': blocks})
            metadata_page = root / 'pdf-to-markdown/02_preprocess/page_0002.json'
            write_json(metadata_page, {'blocks': [{'block_id': 0, 'text': 'chapter'}]})
            name_template = '{{text page=2 block=0 chars=0:7}}'
            book = {'toc': {'depth': 2},
                    'source_cleanup_exceptions': [{
                        'anchor': 'a' * 64, 'element_id': 'h', 'field': 'md_text',
                        'literal': 'Q', 'note': 'OCR text differs from the page image.'}],
                    'chapters': [{'id': 'c', 'title': 'Title ' + name_template,
                    'markdown_file': name_template + '.md',
                    'elements': [{'id': 'h', 'type': 'heading', 'heading_level': 1,
                    'pdf_regions': [], 'md_text': template, 'role': None}]}]}
            write_json(root / 'working/book.json', book)
            loaded, assets, fingerprint = load_input(root / 'working')
            renderer = Renderer()
            try:
                snapshot = renderer.render(root / 'working', loaded, assets, fingerprint, None)
                replace_accepted(root, snapshot)
            finally:
                renderer.close()
            accepted, _, _ = load_input(root / 'accepted')
            self.assertEqual(accepted['source_cleanup_exceptions'], book['source_cleanup_exceptions'])
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

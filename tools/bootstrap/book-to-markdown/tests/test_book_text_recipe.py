"""Source templates must preserve braces, corrections and exact source ranges."""

from pathlib import Path
import importlib.util
import json
import re
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[4] / 'tools/bootstrap/book-to-markdown'))
from review.text_recipe import REFERENCE, attach_adjacent_corrections, compile_template, compile_region_template, compile_markdown_template, edit_template, slice_template, preserve_bold, resolve_template, source_differences, source_reference


from review.markdown import callout_plugin
from review.model import InputCache, element_markdown, export_markdown, import_book, load_input, set_heading_level, toc_markdown, write_json
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

    def test_subscript_characters_keep_their_own_ocr_addresses(self):
        source = ('Relative with Offset\nEA = (PC)+d15\n(d15,PC)\n'
                  'Relative with Index and Offset\nEA= (PC)+da\n(d5,PC,Xn)\n')
        blocks = [{'block_id': 7, 'text': source}]
        target = '| <br>(d<sub>16</sub>,PC)<br>(d<sub>8</sub>,PC,Xn) |'
        template = compile_markdown_template(19, blocks, target)
        self.assertEqual(resolve_template(template, {19: blocks}), target)
        self.assertIn('<sub>{{text page=19 block=7', template)
        self.assertIn('with="8"', template)
        self.assertNotIn('{{accepted text="8"}}', template)

    def test_math_command_syntax_is_not_mistaken_for_pdf_letters(self):
        source = 'EA = (PC)+d15\nEA= (PC)+da\n'
        blocks = [{'block_id': 7, 'text': source}]
        target = ('$\\mathrm{EA} = (\\mathrm{PC})+\\mathrm{d}_{16}$<br>'
                  '$\\mathrm{EA} = (\\mathrm{PC})+\\mathrm{d}_{8}$')
        template = compile_markdown_template(19, blocks, target)
        self.assertEqual(resolve_template(template, {19: blocks}), target)
        self.assertNotIn('{{accepted text="m"}}', template)
        self.assertIn('with="8"', template)
        self.assertNotIn('{{accepted text="8"}}', template)

    def test_subscript_line_can_bridge_missing_ocr_separator(self):
        source = 'da \n8-Bit Offset (Displacement) \nd15 \n16-Bit Offset (Displacement) \n'
        blocks = [{'block_id': 14, 'text': source}]
        target = ('d<sub>8</sub> = 8-Bit Offset (Displacement)\n'
                  'd<sub>16</sub> = 16-Bit Offset (Displacement)')
        template = compile_markdown_template(19, blocks, target)
        self.assertEqual(resolve_template(template, {19: blocks}), target)
        self.assertIn('<sub>{{text page=19 block=14', template)
        self.assertNotIn('{{accepted text="8"}}', template)
        self.assertNotIn('{{accepted text="16"}}', template)
        self.assertNotIn('{{accepted text="6"}}', template)
        self.assertIn('block=14 chars=1:2 replace=0:1 with="8"', template)
        self.assertIn('block=14 chars=34:36 replace=0:2 with="16"', template)

    def test_repeated_near_subscript_rows_do_not_claim_one_source(self):
        source = 'da \n8-Bit Offset (Displacement) \n'
        blocks = [{'block_id': 1, 'text': source}, {'block_id': 2, 'text': source}]
        target = 'd<sub>8</sub> = 8-Bit Offset (Displacement)'
        template = compile_markdown_template(19, blocks, target)
        self.assertEqual(resolve_template(template, {19: blocks}), target)
        self.assertNotIn('chars=1:2 replace=0:1 with="8"', template)

    def test_ocr_letter_correction_keeps_the_original_source_range(self):
        blocks = [{'block_id': 6, 'text': 'ADDO'}]
        template = compile_markdown_template(23, blocks, 'ADDQ')
        self.assertEqual(resolve_template(template, {23: blocks}), 'ADDQ')
        self.assertIn('chars=0:4', template)
        self.assertIn('replace=3:4', template)
        self.assertNotIn('}}Q', template)
        self.assertEqual(resolve_template('{{text page=23 block=6 chars=0:4}}', {23: blocks}), 'ADDO')
        self.assertEqual(resolve_template(slice_template(template, 3, 4), {23: blocks}), 'Q')

    def test_short_ocr_digit_letter_confusions_keep_source_addresses(self):
        for source, visible, offset in (('07', 'D7', 0), ('O7', 'D7', 0),
                                        ('D7', '07', 0),
                                        ('0K', 'OK', 0), ('DO', 'D0', 1)):
            with self.subTest(source=source, visible=visible):
                blocks = [{'block_id': 6, 'text': source}]
                template = compile_markdown_template(27, blocks, visible)
                self.assertEqual(resolve_template(template, {27: blocks}), visible)
                self.assertIn('block=6 chars=0:2', template)
                self.assertIn(f'replace={offset}:{offset + 1}', template)
                self.assertNotIn('{{accepted text=', template)

    def test_multiple_ocr_digit_letter_confusions_in_one_word_share_a_source_range(self):
        blocks = [{'block_id': 6, 'text': 'MC68ECOOO'}]
        template = compile_markdown_template(1, blocks, 'MC68EC000')
        self.assertEqual(resolve_template(template, {1: blocks}), 'MC68EC000')
        self.assertIn('block=6 chars=0:9', template)
        self.assertIn('replace=6:9 with="000"', template)
        self.assertNotIn('{{accepted text=', template)

        blocks = [{'block_id': 6, 'text': 'MC68OOO'}]
        template = compile_markdown_template(1, blocks, 'MC68000')
        self.assertIn('block=6 chars=0:7 replace=4:7 with="000"', template)

    def test_repeated_ocr_words_use_their_table_row_context(self):
        source = ('10.13 \nMC68ECOOO DC Electrical Specifications 10-23\n'
                  '10.14 \nMC68ECOOO AC Electrical Specifications-Read and Write 10-24\n'
                  '10.15 \nMC68ECOOO AC Electrical Specifications-Bus Arbitration 10-28\n')
        blocks = [{'block_id': 6, 'text': source}]
        target = ('<tr><td>10.13</td><td>MC68EC000 DC Electrical Specifications</td></tr>\n'
                  '<tr><td>10.14</td><td>MC68EC000 AC Electrical Specifications—Read and Write</td></tr>\n'
                  '<tr><td>10.15</td><td>MC68EC000 AC Electrical Specifications—Bus Arbitration</td></tr>')
        template = compile_markdown_template(9, blocks, target)
        self.assertEqual(resolve_template(template, {9: blocks}), target)
        starts = [match.start() for match in re.finditer('MC68ECOOO', source)]
        for start in starts:
            self.assertIn(f'block=6 chars={start}:{start + 9} replace=6:9 with="000"',
                          template)
        self.assertNotIn('{{accepted text="MC68EC000"}}', template)

    def test_ocr_word_continues_the_selected_source_block(self):
        blocks = [
            {'block_id': 14, 'text': 'MC68HC001/MC68ECOOO'},
            {'block_id': 16, 'text': 'MC68HCOOO/MC68ECOOO/MC68HC001'},
        ]
        target = '- MC68HC001/MC68EC000\n- MC68HC000/MC68EC000/MC68HC001'
        template = compile_markdown_template(15, blocks, target)
        self.assertEqual(resolve_template(template, {15: blocks}), target)
        self.assertIn('block=14 chars=10:19 replace=6:9 with="000"', template)
        self.assertIn('block=16 chars=10:19 replace=6:9 with="000"', template)
        self.assertNotIn('{{accepted text="MC68EC000"}}', template)

    def test_unique_phrase_in_the_selected_region_keeps_its_source_address(self):
        sentence = "The supervisor programmer's model supplement of the MC68010 is shown in Figure 2-3. "
        blocks = [
            {'block_id': 26, 'text': sentence + '\n'},
            {'block_id': 27, 'text':
             'In addition to the supervisor stack pointer and status register, it includes the vector base \n'
             'register (VRB) and the alternate function code registers (AFC).The VBR is used to \n'
             'determine the location of the exception vector table in memory to support multiple vector \n'},
        ]
        target = (sentence + 'In addition to the supervisor stack pointer and status register, it includes '
                  'the vector base register (VRB) and the alternate function code registers (AFC).'
                  'The VBR is used to determine the location of the exception vector table in memory '
                  'to support multiple vector')
        template = compile_markdown_template(17, blocks, target)
        self.assertEqual(resolve_template(template, {17: blocks}), target)
        self.assertIn('block=26 chars=0:84', template)
        self.assertNotIn('{{accepted text=' + json.dumps(sentence, ensure_ascii=False) + '}}', template)

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

    def test_reconciliation_rechecks_math_after_accepting_other_fragments(self):
        blocks = [{'block_id': 10, 'text': 'D = A(B + B)C + (A + A)B(C + C)\n'},
                  {'block_id': 11, 'text': 'D = ABC + ABC + AB(C + C) + AB(C + C)\n'},
                  {'block_id': 12, 'text': 'D = ABC + ABC + ABC + ABC + ABC + ABC\n'}]
        target = (r'$$' '\n'
                  r'D = A(B + \overline{B})\overline{C} + (A + \overline{A})B(C + \overline{C})' '\n'
                  r'$$' '\n\n' r'$$' '\n'
                  r'D = AB\overline{C} + A\overline{BC} + AB(C + \overline{C})'
                  r' + \overline{A}B(C + \overline{C})' '\n'
                  r'$$' '\n\n' r'$$' '\n'
                  r'D = AB\overline{C} + A\overline{BC} + ABC + AB\overline{C}'
                  r' + \overline{A}BC + \overline{A}B\overline{C}' '\n' r'$$')
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            write_json(workspace / '02_preprocess/page_0128.json', {'blocks': blocks})
            book = {'chapters': [{'id': 'c', 'title': 'Math', 'elements': [
                {'id': 'e', 'type': 'prose', 'role': None,
                 'md_text': compile_markdown_template(128, blocks, target),
                 'pdf_regions': [{'page': 128, 'bbox': [0, 0, 10, 10]}]}]}]}
            reconcile_working(book, {'chapters': []}, workspace)
            self.assertEqual(preview(book, workspace), [])

    def test_unique_long_ocr_word_in_large_block_keeps_one_contiguous_address(self):
        source = 'Table of contents ' * 8 + 'MC68ECOOO Read Cycle Timing Diagram'
        blocks = [{'block_id': 6, 'text': source}]
        template = compile_markdown_template(7, blocks, 'MC68EC000')
        start = source.index('MC68ECOOO')
        self.assertEqual(resolve_template(template, {7: blocks}), 'MC68EC000')
        self.assertIn(f'block=6 chars={start}:{start + 9} replace=6:9 with="000"', template)

    def test_repeated_long_ocr_word_in_large_block_stays_unanchored(self):
        source = 'Table of contents ' * 8 + 'MC68ECOOO and MC68ECOOO'
        blocks = [{'block_id': 6, 'text': source}]
        template = compile_markdown_template(7, blocks, 'MC68EC000')
        self.assertNotIn('{{text page=', template)

    def test_a_word_cannot_borrow_its_suffix_from_another_source_block(self):
        blocks = [{'block_id': 1, 'text': 'MC68ECXXX'},
                  {'block_id': 2, 'text': '000'}]
        template = compile_markdown_template(1, blocks, 'MC68EC000')
        self.assertEqual(resolve_template(template, {1: blocks}), 'MC68EC000')
        self.assertNotIn('block=2', template)
        self.assertNotIn('chars=0:6', template)

    def test_a_word_cannot_be_assembled_from_two_exact_fragments(self):
        blocks = [{'block_id': 1, 'text': 'Marl'},
                  {'block_id': 2, 'text': 'borough'}]
        template = compile_markdown_template(1, blocks, 'Marlborough')
        self.assertEqual(resolve_template(template, {1: blocks}), 'Marlborough')
        self.assertNotIn('{{text page=', template)

    def test_inserted_hyphen_keeps_adjacent_digits_in_one_ocr_word(self):
        blocks = [{'block_id': 0, 'text': 'TABLE  32 \n'}]
        template = compile_markdown_template(76, blocks, 'TABLE 3-2')
        self.assertEqual(resolve_template(template, {76: blocks}), 'TABLE 3-2')
        self.assertIn('block=0 chars=7:8', template)
        self.assertIn('block=0 chars=8:9', template)
        self.assertNotIn('{{accepted text="3"}}', template)
        self.assertNotIn('{{accepted text="2"}}', template)

    def test_inserted_hyphen_does_not_claim_repeated_ocr_words(self):
        blocks = [{'block_id': 0, 'text': 'TABLE  32 32 \n'}]
        template = compile_markdown_template(76, blocks, 'TABLE 3-2')
        self.assertEqual(resolve_template(template, {76: blocks}), 'TABLE 3-2')
        self.assertNotIn('chars=7:8', template)
        self.assertNotIn('chars=8:9', template)

    def test_markup_in_prose_keeps_the_second_ocr_word_correction(self):
        source = ('This manual includes hardware details and programming information for the MC68000, \n'
                  'the MC68HCOOO, the MC68HC001, the MC68008, the MC68010, and the MC68ECOOO. \n'
                  'For ease of reading, the name M68000 MPUs will be used when referring to all \n'
                  'processors. Refer to M68000PM/AD, M68000 Programmer\'s Reference Manual, for \n'
                  'detailed infonnation on the MC68000 instruction set. \n')
        target = ('This manual includes hardware details and programming information for the MC68000, '
                  'the MC68HC000, the MC68HC001, the MC68008, the MC68010, and the MC68EC000. '
                  'For ease of reading, the name M68000 MPUs will be used when referring to all '
                  'processors. Refer to M68000PM/AD, *M68000 Programmer\'s Reference Manual*, '
                  'for detailed information on the MC68000 instruction set.')
        blocks = [{'block_id': 1, 'text': source}]
        template = compile_markdown_template(15, blocks, target)
        start = source.index('MC68ECOOO')
        self.assertEqual(resolve_template(template, {15: blocks}), target)
        self.assertNotIn('chars=0:87 replace=0:87', template)
        self.assertIn('chars=83:84 replace=0:1 with=""', template)
        self.assertIn(f'block=1 chars={start}:{start + len("MC68ECOOO")}', template)
        self.assertIn('replace=6:9 with="000"', template)
        self.assertNotIn('{{accepted text="000."}}', template)
        information_start = source.index('infonnation')
        self.assertIn(f'block=1 chars={information_start}:{information_start + 11} '
                      'replace=4:6 with="rm"', template)
        self.assertNotIn('block=1 chars=54:65', template)

    def test_short_ocr_confusions_bind_words_inside_punctuation(self):
        blocks = [{'block_id': 6, 'text': '07-DO'}]
        template = compile_markdown_template(27, blocks, 'D7-D0')
        self.assertEqual(resolve_template(template, {27: blocks}), 'D7-D0')
        self.assertIn('block=6 chars=0:2', template)
        self.assertIn('block=6 chars=3:5', template)
        self.assertNotIn('{{accepted text=', template)

    def test_short_ocr_confusion_does_not_choose_repeated_source_word(self):
        blocks = [{'block_id': 6, 'text': '07 07'}]
        template = compile_markdown_template(27, blocks, 'D7')
        self.assertNotIn('{{text page=', template)

    def test_short_ocr_confusion_needs_local_context_in_a_long_block(self):
        blocks = [{'block_id': 6, 'text': 'unrelated text ' * 10 + '07'}]
        template = compile_markdown_template(27, blocks, 'D7')
        self.assertNotIn('{{text page=', template)

    def test_context_selects_the_ocr_block_in_a_table_row(self):
        blocks = [{'block_id': 3, 'text': 'ADD \nSource + Destination'},
                  {'block_id': 5, 'text': 'ADDI \nImmediate Data + Destination'},
                  {'block_id': 6, 'text': 'ADDO \nImmediate Data + Destination'}]
        template = compile_markdown_template(23, blocks, '| ADDQ | Immediate Data + Destination |')
        self.assertEqual(resolve_template(template, {23: blocks}),
                         '| ADDQ | Immediate Data + Destination |')
        self.assertIn('block=6 chars=0:4 replace=3:4 with="Q"', template)
        self.assertNotIn('block=3 chars=0:3', template)

    def test_table_cells_split_from_one_ocr_block_keep_character_addresses(self):
        blocks = [{'block_id': 39, 'text': '0\r\nx\r\n'},
                  {'block_id': 40, 'text': '1\r\n0\r\n'},
                  {'block_id': 41, 'text': '1\r\n1\r\n'},
                  {'block_id': 42, 'text': 'USP\r\n'},
                  {'block_id': 43, 'text': 'ISP\r\n'},
                  {'block_id': 44, 'text': 'MSP\r\n'}]
        target = ('| S | M | ACTIVE STACK |\n| --- | --- | --- |\n'
                  '| 0 | x | USP |\n| 1 | 0 | ISP |\n| 1 | 1 | MSP |\n')
        template = compile_markdown_template(40, blocks, target)
        self.assertEqual(resolve_template(template, {40: blocks}), target)
        self.assertIn('block=39 chars=0:1', template)
        self.assertIn('block=39 chars=3:4', template)
        self.assertNotIn('{{accepted text="x"}}', template)

    def test_repeated_target_word_does_not_reuse_the_same_source_range(self):
        blocks = [{'block_id': 1, 'text': 'START ABC END'}]
        template = compile_markdown_template(1, blocks, 'START ABC ABC END')
        self.assertEqual(resolve_template(template, {1: blocks}), 'START ABC ABC END')
        ranges = [(int(match[3]), int(match[4])) for match in REFERENCE.finditer(template)]
        self.assertTrue(all(first[1] <= second[0] for first, second in zip(ranges, ranges[1:])))

    def test_letter_spaced_register_name_with_ocr_zero_error_keeps_source(self):
        blocks = [{'block_id': 0, 'text': 'B L T C O N O'}]
        template = compile_markdown_template(145, blocks, 'BLTCON0')
        self.assertEqual(resolve_template(template, {145: blocks}), 'BLTCON0')
        self.assertIn('chars=0:13 replace=0:13 with="BLTCON0"', template)
        self.assertEqual(resolve_template('{{text page=145 block=0 chars=0:13}}', {145: blocks}),
                         'B L T C O N O')
        last_character = slice_template(template, 6, 7, {145: blocks})
        self.assertEqual(resolve_template(last_character, {145: blocks}), '0')
        self.assertIn('chars=12:13', last_character)

    def test_bold_table_cell_does_not_hide_letter_spaced_source_text(self):
        blocks = [{'block_id': 0, 'text': 'B L T C O N O'}]
        target = '| **BLTCON0** |'
        template = compile_markdown_template(145, blocks, target)
        self.assertEqual(resolve_template(template, {145: blocks}), target)
        self.assertIn('chars=0:13 replace=0:13 with="BLTCON0"', template)

    def test_short_table_address_keeps_ocr_symbol_in_source(self):
        blocks = [{'block_id': 32, 'text': '" 0 4 0\n'}]
        template = compile_markdown_template(147, blocks, '| ~040 |')
        self.assertEqual(resolve_template(template, {147: blocks}), '| ~040 |')
        self.assertIn('block=32 chars=0:7', template)
        self.assertNotIn('{{accepted text=', template)

    def test_short_heading_attaches_ocr_digit_to_complete_source_line(self):
        blocks = [{'block_id': 1, 'text': 'Address Bus (A23-AO) \n'}]
        target = 'Address Bus (A23-A0)'
        template = compile_markdown_template(27, blocks, target)
        self.assertEqual(resolve_template(template, {27: blocks}), target)
        self.assertNotIn('{{accepted text=', template)
        self.assertIn('block=1', template)

    def test_table_row_prefers_zero_read_as_letter_o(self):
        blocks = [{'block_id': 0, 'text': 'J O Y O D A T\n'},
                  {'block_id': 1, 'text': 'J O Y 1 D A T\n'}]
        template = compile_markdown_template(146, blocks, '| JOY0DAT |')
        self.assertEqual(resolve_template(template, {146: blocks}), '| JOY0DAT |')
        self.assertIn('block=0 chars=0:13', template)
        self.assertNotIn('{{accepted text=', template)

    def test_short_ocr_gap_between_source_ranges_is_recorded_as_a_correction(self):
        blocks = [{'block_id': 0, 'text': 'ABCltlDEF'}]
        template = ('{{text page=1 block=0 chars=0:3}}'
                    '{{accepted text="ili"}}'
                    '{{text page=1 block=0 chars=6:9}}')
        corrected = attach_adjacent_corrections(template, {1: blocks})
        self.assertIn('chars=3:6 replace=0:3 with="ili"', corrected)
        self.assertEqual(resolve_template(corrected, {1: blocks}), 'ABCiliDEF')

    def test_plain_paragraph_aligns_all_ocr_errors_without_loose_letters(self):
        source = ('Motorola specifically disclaims any and all llabltlty, '
                  'including without limitation consequential damages.\n')
        visible = ('Motorola specifically disclaims any and all liability, '
                   'including without limitation consequential damages.')
        blocks = [{'block_id': 4, 'text': source}]
        template = compile_markdown_template(3, blocks, visible)
        self.assertEqual(resolve_template(template, {3: blocks}), visible)
        self.assertNotIn('{{accepted text=', template)
        self.assertEqual(''.join(source[int(ref[3]):int(ref[4])]
                                 for ref in REFERENCE.finditer(template)), source)

    def test_plain_multiline_list_keeps_ocr_suffix_as_correction(self):
        source = ('MC68000 \nMC68008 \nNiC68010 \nMC68HCOOO \nMC68HC001 \n'
                  'and the New \nMC68ECOOO \n')
        visible = ('MC68000\nMC68008\nMC68010\nMC68HC000\nMC68HC001\n'
                   'and the New\nMC68EC000')
        blocks = [{'block_id': 4, 'text': source}]
        template = compile_markdown_template(1, blocks, visible)
        self.assertEqual(resolve_template(template, {1: blocks}), visible)
        self.assertNotIn('{{accepted text=', template)
        self.assertEqual(''.join(source[int(ref[3]):int(ref[4])]
                                 for ref in REFERENCE.finditer(template)), source)

    def test_letter_spaced_paragraph_across_blocks_keeps_each_block_address(self):
        phrases = ('These two control registers ', 'are used together to ',
                   'control blitter operations.')
        blocks = [{'block_id': number, 'text': ' '.join(phrase) + '\n'}
                  for number, phrase in enumerate(phrases, 2)]
        visible = ''.join(phrases)
        template = compile_markdown_template(145, blocks, visible)
        self.assertEqual(resolve_template(template, {145: blocks}), visible)
        self.assertNotIn('{{accepted text=', template)
        self.assertEqual([int(ref[2]) for ref in REFERENCE.finditer(template)], [2, 3, 4])

    def test_unanchored_literal_remains_explicit_and_editable(self):
        blocks = [{'block_id': 1, 'text': 'Source'}]
        template = compile_markdown_template(1, blocks, 'Source Vision')
        self.assertIn('{{accepted text="Vision"}}', template)
        self.assertEqual(resolve_template(template, {1: blocks}), 'Source Vision')
        self.assertEqual(resolve_template(slice_template(template, 7, 13), {1: blocks}), 'Vision')

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

    def test_saved_working_edit_reconciles_ocr_correction_without_accepting_book(self):
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
            self.assertIn('chars=0:4 replace=3:4 with="Q"', saved['md_text'])
            self.assertEqual(saved['source_differences'][0]['kind'], 'correction')
            self.assertEqual((root / 'accepted/book.json').read_bytes(), accepted_before)

    def test_letter_spaced_ocr_retains_full_source_span(self):
        blocks = [{'block_id': 1, 'text': 'B L T D D A T\nB l i t t e r d e s t i n a t i o n\n'}]
        target = '| BLTDDAT | Blitter destination |'
        template = compile_markdown_template(147, blocks, target)
        self.assertEqual(resolve_template(template, {147: blocks}), target)
        self.assertIn('block=1 chars=0:13 replace=0:13 with="BLTDDAT"', template)
        self.assertIn('with="Blitter destination"', template)
        self.assertEqual(resolve_template(slice_template(template, 2, 9), {147: blocks}), 'BLTDDAT')
        first = slice_template(template, 12, 20, {147: blocks})
        second = slice_template(template, 20, len(target) - 2, {147: blocks})
        self.assertEqual(resolve_template(first + second, {147: blocks}), 'Blitter destination')
        self.assertIn('block=1', first)
        self.assertIn('block=1', second)

    def test_long_letter_spaced_paragraph_uses_reviewable_source_spans(self):
        visible = ('These two control registers work together to manage the display. '
                   'Each register keeps a separate setting for the next operation.')
        source = ' '.join(visible)
        blocks = [{'block_id': 1, 'text': source}]
        template = compile_markdown_template(145, blocks, visible)
        self.assertEqual(resolve_template(template, {145: blocks}), visible)
        references = list(REFERENCE.finditer(template))
        self.assertEqual(''.join(source[int(ref[3]):int(ref[4])] for ref in references), source)
        self.assertLessEqual(max(int(ref[4]) - int(ref[3]) for ref in references), 80)
        self.assertNotIn('{{accepted text=', template)

    def test_long_letter_spaced_blocks_keep_short_source_corrections(self):
        phrases = ('These two control registers work together to manage the display. ',
                   'Each register keeps a separate setting for the next operation.')
        blocks = [{'block_id': number, 'text': ' '.join(phrase) + '\n'}
                  for number, phrase in enumerate(phrases, 1)]
        target = ''.join(phrases)
        template = compile_markdown_template(145, blocks, target)
        self.assertEqual(resolve_template(template, {145: blocks}), target)
        for block in blocks:
            references = [ref for ref in REFERENCE.finditer(template)
                          if int(ref[2]) == block['block_id']]
            self.assertEqual(''.join(block['text'][int(ref[3]):int(ref[4])]
                                     for ref in references), block['text'])
            self.assertLessEqual(max(int(ref[4]) - int(ref[3]) for ref in references), 80)

    def test_styled_prose_does_not_replace_a_whole_phrase_for_one_ocr_digit(self):
        source = ('This register is the audio channel x (x=0,l,2,3) \n'
                  'DMA data buffer. It contains 2 bytes of data that are each 2s complement.\n')
        target = ('This register is the audio channel x (x=0,1,2,3) '
                  'DMA data buffer. It contains 2 bytes of data that are each 2s complement. *Note*')
        blocks = [{'block_id': 20, 'text': source}]
        template = compile_markdown_template(144, blocks, target)
        self.assertEqual(resolve_template(template, {144: blocks}), target)
        corrections = [ref for ref in REFERENCE.finditer(template) if ' replace=' in ref[0]]
        self.assertTrue(corrections)
        self.assertLess(max(int(ref[4]) - int(ref[3]) for ref in corrections), 20)

    def test_ocr_ligature_expands_within_the_source_word(self):
        blocks = [{'block_id': 15, 'text': 'bit ﬁeld'}]
        template = compile_markdown_template(38, blocks, 'bit field')
        self.assertEqual(resolve_template(template, {38: blocks}), 'bit field')
        self.assertIn('chars=4:8 replace=0:1 with="fi"', template)
        self.assertEqual(resolve_template('{{text page=38 block=15 chars=4:5}}',
                                          {38: blocks}), 'ﬁ')
        sliced = slice_template(template, 4, 6, {38: blocks})
        self.assertEqual(resolve_template(sliced, {38: blocks}), 'fi')
        self.assertIn('block=15 chars=4:5 replace=0:1 with="fi"', sliced)

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
            {'type': 'code_block', 'resolved_md_text': target, 'role': None}))
        self.assertIn('<strong>Assembler Syntax:</strong>', html)
        self.assertIn('class="language-asm"', html)
        self.assertNotIn('language-text', html)

    def test_import_preserves_prepared_markdown_without_literal_transcription(self):
        prepared = ['**Operation:** Source + Destination',
                    '```asm\nADD <ea>,Dn\nADD Dn,<ea>\n```',
                    '| X | N |\n|---|---|\n| * | * |', '\\*Word and long only', 'NOTE', 'Read carefully.']
        expected = prepared[:-2] + ['> [!NOTE] NOTE\n>\n> Read carefully.']
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / 'pdf-to-markdown'
            status = {s: {'status': 'success'} for s in ('02', '04', '05', '06', '07')}
            # The last table refresh does not restrict the retained page collection.
            status['05']['page_ranges'] = '2'
            write_json(workspace / 'stage_status.json', status)
            source = 'Operation: Source + Destination\nADD <ea>,Dn\nADD Dn,<ea>\nX N\n* *\n*Word and long only\nNOTE\nRead carefully.\n'
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
            write_json(root / 'working/book.json', {'toc': {'depth': 3}, 'chapters': []})
            obsolete = ['working/old-chapter.md', 'working/assets/old.png',
                        'working/renders/old/document.html', 'accepted/book.json',
                        'accepted/assets/old.png', 'pdf/pages/page_9999.png']
            for relative in obsolete:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('obsolete output', encoding='utf-8')
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

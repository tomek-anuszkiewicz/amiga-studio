"""Focused contracts for the page-local first-pass source alignment."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from review.source_alignment import align_template_literals, compile_page_template
from review.text_recipe import REFERENCE, resolve_template, source_differences


def page(*texts):
    return {
        "blocks": [{"block_id": index, "text": text,
                    "bbox": [0, index * 20, 100, index * 20 + 10]}
                   for index, text in enumerate(texts)],
        "raster": {"display_to_pixels": [1, 0, 0, 1, 0, 0]},
    }


class PageAlignmentTest(unittest.TestCase):
    def test_unpaired_code_delimiters_do_not_cross_inline_containers(self):
        for target in ('A literal ` mark.\n\n$x$\n\nAnother ` mark.',
                       '# A `\n$x$\nAnother ` mark.',
                       '- A `\n- $x$\n- Another `',
                       '| First | Second |\n| --- | --- |\n| A ` | $x$ and ` |'):
            with self.subTest(target=target):
                source = page('A literal mark x Another First Second and')
                recipe = compile_page_template(1, source, [0, 0, 100, 10], target)
                self.assertEqual(resolve_template(recipe, {1: source['blocks']}), target)

    def test_escaped_backticks_preserve_math_and_html_source_mapping(self):
        from review.source_cleanup import _literal_spans
        for target, visible in ((r'\`$x$\`', 'x'),
                                (r'\`<p>Word</p>\`', 'Word')):
            with self.subTest(target=target):
                source = page(visible)
                recipe = compile_page_template(1, source, [0, 0, 100, 10], target)
                self.assertEqual(resolve_template(recipe, {1: source['blocks']}), target)
                self.assertNotIn('{{accepted', recipe)
                self.assertEqual(list(_literal_spans(recipe)), [])

    def test_math_with_escaped_table_pipes_retains_source_mapping(self):
        from review.source_cleanup import _literal_spans
        target = '| Meaning | Value |\n| --- | --- |\n| norm | ' + r'$\|x\|$ |'
        source = page('Meaning Value norm x')
        recipe = compile_page_template(1, source, [0, 0, 100, 10], target)
        self.assertEqual(resolve_template(recipe, {1: source['blocks']}), target)
        self.assertNotIn('{{accepted', recipe)
        self.assertEqual(list(_literal_spans(recipe)), [])

    def test_angle_expressions_inside_math_are_aligned_as_text(self):
        from review.source_cleanup import _literal_spans
        target = '$x<y>z$'
        source = page('x<y>z')
        recipe = compile_page_template(1, source, [0, 0, 100, 10], target)
        self.assertEqual(resolve_template(recipe, {1: source['blocks']}), target)
        self.assertNotIn('{{accepted', recipe)
        self.assertTrue(any('y' in literal for _, _, literal, _ in _literal_spans(target)))
        self.assertEqual(list(_literal_spans(recipe)), [])

    def test_supported_html_structure_is_preserved_in_alignment_and_cleanup(self):
        from review.source_cleanup import _literal_spans
        target = ('<details><summary>Title</summary>Body</details><h2>Heading</h2><hr>'
                  '<mark>highlight</mark><small>note</small>')
        source = page('Title Body Heading highlight note')
        recipe = compile_page_template(1, source, [0, 0, 100, 10], target)
        self.assertEqual(resolve_template(recipe, {1: source['blocks']}), target)
        self.assertNotIn('{{accepted', recipe)
        self.assertEqual([literal for _, _, literal, _ in _literal_spans(target)],
                         ['Title', 'Body', 'Heading', 'highlight', 'note'])

    def test_code_angles_do_not_hide_words_from_alignment_or_cleanup(self):
        from review.source_cleanup import _literal_spans
        from review.source_validation import validate_book
        from review.model import write_json
        import tempfile
        body = 'flags = (1<<0);\n/* retained words, pc -> amiga */\n<td>literal</td>'
        source = page(body)
        target = '```c\n' + body + '\n```\n\n`<CSI>`'
        source['blocks'].append({'block_id': 1, 'text': 'CSI', 'bbox': [0, 20, 100, 30]})
        recipe = compile_page_template(1, source, [0, 0, 100, 30], target)
        self.assertEqual(resolve_template(recipe, {1: source['blocks']}), target)
        book = {'chapters': [{'id': 'c', 'title': '', 'elements': [
            {'id': 'e', 'type': 'code_block', 'md_text': recipe}]}]}
        with tempfile.TemporaryDirectory() as temporary:
            workspace = Path(temporary)
            write_json(workspace / '02_preprocess/page_0001.json', source)
            self.assertFalse(validate_book(book, workspace)['issues'])
        literals = [literal for _, _, literal, _ in _literal_spans(target)]
        self.assertIn('retained words,', literals)
        self.assertIn('CSI', literals)

    def test_line_break_and_inline_math_keep_exact_visible_markdown(self):
        source = page("The cal\u00ad\nculation gives 31,469.\n")
        target = "The calculation gives $31{,}469$."
        recipe = compile_page_template(122, source, [0, 0, 100, 10], target)
        self.assertEqual(resolve_template(recipe, {122: source["blocks"]}), target)
        self.assertTrue(REFERENCE.search(recipe))
        self.assertFalse(any(mark["kind"] == "unanchored" for mark in
                             source_differences(recipe, {122: source["blocks"]})))

    def test_html_cells_can_use_distinct_pdf_blocks(self):
        source = page("Alpha\n", "Beta\n")
        target = "<table><tr><td>Alpha</td><td>Beta</td></tr></table>"
        recipe = compile_page_template(4, source, [0, 0, 100, 30], target)
        self.assertEqual(resolve_template(recipe, {4: source["blocks"]}), target)
        self.assertIn("block=0", recipe)
        self.assertIn("block=1", recipe)

    def test_repeated_table_label_prefers_exact_local_block(self):
        source = page("clock constant", "Clock Constant\n3579545")
        target = "<table><tr><th>Clock Constant</th></tr></table>"
        recipe = compile_page_template(122, source, [0, 20, 100, 30], target)
        self.assertEqual(resolve_template(recipe, {122: source["blocks"]}), target)
        self.assertIn("block=1 chars=0:5", recipe)
        self.assertNotIn("block=0", recipe)
        self.assertFalse(source_differences(recipe, {122: source["blocks"]}))

    def test_missing_word_remains_accepted(self):
        source = page("Alpha\n")
        target = "Alpha Gamma"
        recipe = compile_page_template(4, source, [0, 0, 100, 10], target)
        self.assertEqual(resolve_template(recipe, {4: source["blocks"]}), target)
        self.assertIn('{{accepted text="Gamma"}}', recipe)

    def test_numbered_content_is_referenced_or_accepted_without_changing_markdown(self):
        source = page("1. Alpha\n2. Beta\n306. cylinders\n")
        target = ("```text\n1. Alpha\n2. Beta\n```\n\n"
                  "Note: 1. Alpha\n2. Beta\n\n"
                  "| Value | Label |\n| --- | --- |\n"
                  "| 306. | cylinders |\n| 999. | absent |")
        recipe = compile_page_template(4, source, [0, 0, 100, 10], target)
        pages = {4: source["blocks"]}
        self.assertEqual(resolve_template(recipe, pages), target)
        referenced = [resolve_template(match[0], pages)
                      for match in REFERENCE.finditer(recipe)]
        self.assertEqual([text for text in referenced if text.isdecimal()],
                         ["1", "2", "1", "2", "306"])
        self.assertIn('{{accepted text="999"}}', recipe)

    def test_formula_can_reference_words_out_of_pdf_extraction_order(self):
        source = page("sample interval\nclock constant\nPeriod value\n"
                      "clock interval\nsamples per second\n")
        target = (r"$$\text{Period value} = "
                  r"\frac{\text{sample interval}}{\text{clock interval}} = "
                  r"\frac{\text{clock constant}}{\text{samples per second}}$$")
        recipe = compile_page_template(122, source, [0, 0, 100, 10], target)
        self.assertEqual(resolve_template(recipe, {122: source["blocks"]}), target)
        self.assertNotIn('{{accepted text="="}}', recipe)
        self.assertIn('block=0', recipe)

    def test_edit_alignment_preserves_authored_references_and_acceptance(self):
        source = page("Alpha Beta Gamma")
        authored = ('{{text page=4 block=0 chars=0:5}} '
                    '{{accepted text="Deliberate"}} Beta')
        recipe = align_template_literals(4, source, [0, 0, 100, 10], authored)
        self.assertTrue(recipe.startswith('{{text page=4 block=0 chars=0:5}} '
                                          '{{accepted text="Deliberate"}} '))
        self.assertIn('block=0 chars=6:10', recipe)
        self.assertEqual(resolve_template(recipe, {4: source["blocks"]}),
                         'Alpha Deliberate Beta')

    def test_bold_markers_stay_syntax_while_display_math_operator_is_addressed(self):
        source = page("Alpha A * B")
        target = "**Alpha**\n$$\nA * B\n$$"
        recipe = compile_page_template(4, source, [0, 0, 100, 10], target)
        self.assertEqual(resolve_template(recipe, {4: source["blocks"]}), target)
        self.assertIn("**", recipe)
        self.assertFalse(any(mark["kind"] == "unanchored" for mark in
                             source_differences(recipe, {4: source["blocks"]})))

    def test_ocr_symbol_inside_number_uses_neighboring_digits(self):
        source = page("3,579£45 ticks/second")
        target = r"3{,}579{,}545\ \text{ticks/second}"
        recipe = compile_page_template(122, source, [0, 0, 100, 10], target)
        self.assertEqual(resolve_template(recipe, {122: source["blocks"]}), target)
        self.assertNotIn('{{accepted text="545"}}', recipe)
        self.assertIn('chars=5:6 replace=0:1 with="5"', recipe)
        self.assertIn(('5', '£'), [(mark['text'], mark['source_text']) for mark in
                                  source_differences(recipe, {122: source["blocks"]})])


    def test_table_word_with_ocr_letter_instead_of_digit(self):
        source = page("AUD3EN\nAudio channel 3 enable.\nAUD2EN\nAudio channel 2 enable.\n"
                      "AUD1EN\nAudio channel 1 enable.\nAUDOEN\nAudio channel 0 enable.\n")
        target = "| 0 | AUD0EN | Audio channel 0 enable. |"
        recipe = compile_page_template(123, source, [0, 0, 100, 10], target)
        self.assertEqual(resolve_template(recipe, {123: source["blocks"]}), target)
        self.assertNotIn('{{accepted text="AUD0EN"}}', recipe)
        self.assertIn('replace=0:1 with="0"', recipe)
        self.assertIn(('0', 'O'), [(mark['text'], mark['source_text']) for mark in
                                 source_differences(recipe, {123: source["blocks"]})])

    def test_repeated_two_character_table_codes_use_real_ocr_words(self):
        source = page("AO BO CO Cl DO\nAO BO CO Cl DO\n")
        target = ("<table><tr><td>A0</td><td>B0</td><td>C0</td>"
                  "<td>C1</td><td>D0</td></tr></table>")
        recipe = compile_page_template(130, source, [0, 0, 100, 10], target)
        self.assertEqual(resolve_template(recipe, {130: source["blocks"]}), target)
        self.assertNotIn("{{accepted", recipe)
        corrections = source_differences(recipe, {130: source["blocks"]})
        self.assertEqual([(mark["text"], mark["source_text"]) for mark in corrections],
                         [("0", "O"), ("0", "O"), ("0", "O"), ("1", "l"), ("0", "O")])

    def test_table_label_prefers_whole_word_over_prose_prefix(self):
        source = page("7\nblitter finished\nBLIT\n",
                      "external INT6 & CIAB\nEXTER\n")
        target = "<table><tr><td>BLIT</td><td>EXTER</td></tr></table>"
        recipe = compile_page_template(137, source, [0, 0, 100, 30], target)
        self.assertEqual(resolve_template(recipe, {137: source["blocks"]}), target)
        self.assertIn('block=0 chars=19:23', recipe)
        self.assertIn('block=1 chars=21:26', recipe)
        self.assertFalse(source_differences(recipe, {137: source["blocks"]}))

    def test_two_character_table_word_prefers_exact_word_over_prose_fragment(self):
        source = page("OR\nDB8\n9th data bit\n", "polarity")
        target = "<table><tr><td>OR</td></tr></table>"
        recipe = compile_page_template(143, source, [0, 0, 100, 30], target)
        self.assertEqual(resolve_template(recipe, {143: source["blocks"]}), target)
        self.assertIn('block=0 chars=0:2', recipe)
        self.assertFalse(source_differences(recipe, {143: source["blocks"]}))

    def test_repeated_word_prefers_exact_case_over_heading_case(self):
        source = page("PRECOMP\n", "( 1=MFM precomp 0-GCR precontp)")
        target = "<td>( 1=MFM precomp 0=GCR precomp)</td>"
        recipe = compile_page_template(144, source, [0, 0, 100, 30], target)
        self.assertEqual(resolve_template(recipe, {144: source["blocks"]}), target)
        self.assertEqual(recipe.count('block=1 chars=8:15'), 2)
        self.assertNotIn('block=0', recipe)
        self.assertFalse(source_differences(recipe, {144: source["blocks"]}))

    def test_word_split_across_adjacent_pdf_blocks(self):
        source = page("The code infor-\n", "mation is here.\n")
        target = "The code information is here."
        recipe = compile_page_template(72, source, [0, 0, 100, 30], target)
        self.assertEqual(resolve_template(recipe, {72: source["blocks"]}), target)
        self.assertIn('block=0 chars=9:14', recipe)
        self.assertIn('block=1 chars=0:6', recipe)
        self.assertNotIn('{{accepted text="information"}}', recipe)

    def test_hyphenated_word_can_end_before_a_fused_ocr_glyph(self):
        source = page('expansion.library and ask for the current buildings (GetCurrent- \n',
                      'BindingO). In this structure will be the head of a singly linked list\n')
        recipe = compile_page_template(55, source, [0, 0, 100, 30], 'GetCurrentBinding')
        self.assertEqual(resolve_template(recipe, {55: source['blocks']}), 'GetCurrentBinding')
        self.assertNotIn('{{accepted', recipe)
        self.assertEqual([int(ref[2]) for ref in REFERENCE.finditer(recipe)], [0, 1])
        self.assertIn('block=1 chars=0:7', recipe)

    def test_fused_continuation_requires_a_local_hyphenated_boundary(self):
        from review.source_alignment import _split_page_word
        for first, near in [('GetCurrent\n', {0, 1}), ('GetCurrent-\n', {0})]:
            source = page(first, 'BindingO).')
            nearby = {id(block) for index, block in enumerate(source['blocks']) if index in near}
            self.assertIsNone(_split_page_word('GetCurrentBinding', source['blocks'], nearby))

    def test_repeated_long_ocr_word_can_anchor_to_one_occurrence(self):
        source = page("AUDOLC\nAUDOLC\n")
        recipe = compile_page_template(120, source, [0, 0, 100, 10], "AUD0LC")
        self.assertEqual(resolve_template(recipe, {120: source["blocks"]}), "AUD0LC")
        self.assertNotIn("{{accepted", recipe)
        self.assertIn(('0', 'O'), [(mark['text'], mark['source_text']) for mark in
                                 source_differences(recipe, {120: source["blocks"]})])

    def test_three_ocr_glyph_errors_in_long_word_stay_anchored(self):
        source = page("MC68ECOOO\n")
        target = "MC68EC000"
        recipe = compile_page_template(12, source, [0, 0, 100, 10], target)
        self.assertEqual(resolve_template(recipe, {12: source["blocks"]}), target)
        self.assertNotIn("{{accepted", recipe)
        self.assertEqual([(mark["text"], mark["source_text"]) for mark in
                          source_differences(recipe, {12: source["blocks"]})],
                         [("0", "O")] * 3)

    def test_repeated_short_ocr_word_marks_one_substitution_yellow(self):
        source = page("Sec\nSec <ea>\n")
        target = "| Scc | Scc <ea> |"
        recipe = compile_page_template(25, source, [0, 0, 100, 10], target)
        self.assertEqual(resolve_template(recipe, {25: source["blocks"]}), target)
        self.assertNotIn("{{accepted", recipe)
        self.assertEqual([(mark["kind"], mark["text"], mark["source_text"])
                          for mark in source_differences(recipe, {25: source["blocks"]})],
                         [("correction", "c", "e")] * 2)

    def test_exact_case_elsewhere_beats_mapped_case_change(self):
        source = page("example\n", "This EXAMPLE\n")
        recipe = compile_page_template(100, source, [0, 20, 100, 30], "This example")
        self.assertEqual(resolve_template(recipe, {100: source["blocks"]}), "This example")
        self.assertIn('block=0 chars=0:7', recipe)
        self.assertFalse(source_differences(recipe, {100: source["blocks"]}))

    def test_missing_ocr_letters_leave_only_the_inserted_letters_blue(self):
        for source_word, target_word, inserted in (
                ("Marborough", "Marlborough", "l"),
                ("MC68000", "MC68EC000", "EC")):
            with self.subTest(target_word=target_word):
                source = page(source_word)
                recipe = compile_page_template(6, source, [0, 0, 100, 10], target_word)
                self.assertEqual(resolve_template(recipe, {6: source["blocks"]}), target_word)
                self.assertEqual([(mark["kind"], mark["text"]) for mark in
                                  source_differences(recipe, {6: source["blocks"]})],
                                 [("unanchored", inserted)])

    def test_two_ocr_glyphs_change_without_marking_the_whole_word(self):
        source = page("SeUanusRequest")
        target = "SetJanusRequest"
        recipe = compile_page_template(83, source, [0, 0, 100, 10], target)
        self.assertEqual(resolve_template(recipe, {83: source["blocks"]}), target)
        self.assertEqual([(mark["text"], mark["source_text"]) for mark in
                          source_differences(recipe, {83: source["blocks"]})],
                         [("tJ", "U")])

    def test_spaced_ocr_register_names_keep_only_glyph_corrections(self):
        source = page("B L T C O N O\n", "J O Y O D A T\n", "C O P I L C H\n")
        target = ("<table><tr><td>BLTCON0</td><td>JOY0DAT</td>"
                  "<td>COP1LCH</td></tr></table>")
        recipe = compile_page_template(148, source, [0, 0, 100, 50], target)
        self.assertEqual(resolve_template(recipe, {148: source["blocks"]}), target)
        self.assertNotIn("{{accepted", recipe)
        self.assertEqual([(mark["text"], mark["source_text"]) for mark in
                          source_differences(recipe, {148: source["blocks"]})],
                         [("0", "O"), ("0", "O"), ("1", "I")])

    def test_code_suffix_uses_uppercase_ocr_identifier(self):
        source = page("ADFLFNCTNJNFO\n", "EXECJNTERRUPTS\n", "into interrupts\n")
        target = "<code>INFO</code> <code>INTERRUPTS</code>"
        recipe = compile_page_template(86, source, [0, 0, 100, 50], target)
        self.assertEqual(resolve_template(recipe, {86: source["blocks"]}), target)
        self.assertEqual([(mark["text"], mark["source_text"]) for mark in
                          source_differences(recipe, {86: source["blocks"]})],
                         [("I", "J"), ("I", "J")])

    def test_bracket_inside_ocr_identifier_is_one_glyph_correction(self):
        source = page("EXEC_L[BRAR1ES_!\n", "exec/libraries.i\n")
        target = "<code>LIBRARIES</code>"
        recipe = compile_page_template(86, source, [0, 0, 100, 30], target)
        self.assertEqual(resolve_template(recipe, {86: source["blocks"]}), target)
        self.assertEqual([(mark["text"], mark["source_text"]) for mark in
                          source_differences(recipe, {86: source["blocks"]})],
                         [("I", "["), ("I", "1")])

    def test_two_ocr_digit_letter_pairs_in_short_code(self):
        for source_word, target_word, changes in (
                ("8O88G0", "8088Go", [("0", "O"), ("o", "0")]),
                ("lOf", "10f", [("1", "l"), ("0", "O")])):
            with self.subTest(target_word=target_word):
                source = page(source_word)
                recipe = compile_page_template(88, source, [0, 0, 100, 10], target_word)
                self.assertEqual(resolve_template(recipe, {88: source["blocks"]}), target_word)
                self.assertEqual([(mark["text"], mark["source_text"]) for mark in
                                  source_differences(recipe, {88: source["blocks"]})],
                                 changes)


if __name__ == "__main__":
    unittest.main()

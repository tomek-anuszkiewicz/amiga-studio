"""List authored text without PDF references, including accepted literals."""

import json
import re

from .model import read_json
from .markdown import PARSER, code_source_spans, html_tag_spans, math_source_positions
from .text_recipe import ACCEPTED, REFERENCE, difference_locator, source_text


def _formatting(text):
    """Mask syntax while preserving code, formula arguments and text offsets."""
    masked = bytearray(len(text))
    lines = text.splitlines(keepends=True)
    offsets = [0]
    for line in lines:
        offsets.append(offsets[-1] + len(line))
    tokens = PARSER.parse(text)
    code = code_source_spans(text, tokens)

    def hide(start, end):
        masked[start:end] = b'\1' * (end - start)

    for token in tokens:
        if token.type in {'fence', 'code_block'} and token.map:
            start, end = token.map
            if token.type == 'fence':
                hide(offsets[start], offsets[start + 1])
                if end > start + 1 and re.fullmatch(r'\s*(`{3,}|~{3,})\s*', lines[end - 1]):
                    hide(offsets[end - 1], offsets[end])

    def in_code(start, end):
        return any(left <= start and end <= right for left, right in code)

    for start, end in html_tag_spans(text):
        hide(start, end)
    for entity in re.finditer(r'&(?:#[0-9]+|#x[0-9a-fA-F]+|[A-Za-z][A-Za-z0-9]+);', text):
        if not in_code(*entity.span()):
            hide(*entity.span())

    for expression, positions in math_source_positions(text, tokens, code):
        def hide_expression(start, end):
            for position in positions[start:end]:
                masked[position] = 1

        # Command names describe TeX notation. Their arguments, including plain
        # words and numbers, remain content requiring their own PDF references.
        for command in re.finditer(r'\\([A-Za-z]+)', expression):
            hide_expression(command.start(), command.end())
            if command[1] in {'begin', 'end'}:
                environment = re.match(r'\{[^}]*\}', expression[command.end():])
                if environment:
                    hide_expression(command.end(), command.end() + environment.end())
        for syntax in re.finditer(r'[$\{\}_^]', expression):
            hide_expression(syntax.start(), syntax.end())
    return masked


def inventory(book, workspace):
    """Return a read-only inventory in chapter and element order."""
    fields = []
    if 'title' in book.get('toc', {}):
        fields.append(('toc', None, 'title', book['toc']['title'], [], True))
    for chapter in book['chapters']:
        first_pages = sorted({r['page'] for e in chapter['elements'][:1]
                              for r in e.get('pdf_regions', [])})
        fields.append((chapter['id'], None, 'title', chapter['title'], first_pages, True))
        for element in chapter['elements']:
            if 'md_text' in element:
                fields.append((chapter['id'], element['id'], 'md_text', element['md_text'],
                               sorted({r['page'] for r in element.get('pdf_regions', [])}),
                               element.get('visible', True)))
    blocks, findings = {}, []
    for chapter_id, element_id, field, template, pages, visible in fields:
        parts, origins = [], []
        position, accepted_id = 0, 0
        while position < len(template):
            reference = REFERENCE.match(template, position)
            accepted = ACCEPTED.match(template, position) if not reference else None
            if reference:
                page, block = int(reference[1]), int(reference[2])
                if page not in blocks:
                    entries = read_json(workspace / '02_preprocess' / f'page_{page:04d}.json')['blocks']
                    blocks[page] = {entry['block_id']: entry['text'] for entry in entries}
                value = source_text(reference, blocks[page][block])
                origin, position = 0, reference.end()
            elif accepted:
                value = json.loads(accepted[1])
                accepted_id += 1
                origin, position = accepted_id, accepted.end()
            elif template[position] == '\\':
                value = template[position + 1]
                origin, position = -1, position + 2
            else:
                value = template[position]
                origin, position = -1, position + 1
            parts.append(value)
            origins.extend([origin] * len(value))
        text = ''.join(parts)
        masked = _formatting(text)
        context = ''.join(' ' if masked[i] else char for i, char in enumerate(text))
        start = 0
        while start < len(text):
            origin = origins[start]
            if not origin or masked[start]:
                start += 1
                continue
            end = start + 1
            while end < len(text) and origins[end] == origin and not masked[end]:
                end += 1
            literal = text[start:end].strip()
            if any(char.isalnum() for char in literal):
                left = start + len(text[start:end]) - len(text[start:end].lstrip())
                right = end - (len(text[start:end]) - len(text[start:end].rstrip()))
                findings.append({'chapter_id': chapter_id, 'element_id': element_id,
                                 'field': field, 'pages': pages, 'visible': visible,
                                 'status': 'accepted' if origin > 0 else 'pending',
                                 'literal': literal,
                                 **difference_locator(text, left, literal),
                                 'context': (context[max(0, left - 60):left] + '[' + literal + ']' +
                                             context[right:right + 60])})
            start = end
    return findings

"""Compare authored content across Markdown, HTML and CSV representations."""

import csv
from difflib import SequenceMatcher
from html.parser import HTMLParser
import io
import re

from .markdown import PARSER


EDITABLE_PUNCTUATION = frozenset(',.\u002d\u2010\u2011\u2012\u2013\u2014')


def significant_tokens(tokens):
    return [token for token in tokens if token not in EDITABLE_PUNCTUATION]


def significant_table(table):
    rows = [[{**cell, 'text': significant_tokens(cell['text'])} for cell in row]
            for row in table]
    if not rows or len({len(row) for row in rows}) != 1 or len(rows[0]) < 2:
        return rows
    decorative_columns = {index for index in range(len(rows[0]))
                          if all(not row[index]['text'] and row[index]['rowspan'] == 1
                                 and row[index]['colspan'] == 1 for row in rows)}
    if len(decorative_columns) == len(rows[0]):
        return rows
    return [[cell for index, cell in enumerate(row) if index not in decorative_columns]
            for row in rows]


def text_tokens(text):
    # Ignore layout whitespace without joining or splitting words. Preserve case,
    # numbers, punctuation, symbols and repetition, including their order.
    return re.findall(r'\w+|[^\w\s]', text, re.UNICODE)


def normalized_math_expression(expression):
    previous = None
    while expression != previous:
        previous = expression
        expression = re.sub(r'\\(?:mathrm|text)\{([^{}]*)\}', r'\1', expression)
    return re.sub(r'_\{([^{}]+)\}', r'_\1', expression)


class ContentHTML(HTMLParser):
    def __init__(self, *, math_subscripts=False, math_literals=(), math_overlines=False,
                 reject_raw_math=False, math_source=False):
        super().__init__(convert_charrefs=True)
        self.math_subscripts = math_subscripts
        self.math_literals = math_literals
        self.math_overlines = math_overlines
        self.reject_raw_math = reject_raw_math
        self.math_source = math_source
        self.overline_spans = []
        self.text = []
        self.tables = []
        self.table_stack = []

    def append(self, text):
        self.text.append(text)
        for table in self.table_stack:
            if table['cell'] is not None:
                table['cell']['parts'].append(text)

    def handle_starttag(self, tag, attrs):
        if tag == 'span':
            overline = self.math_overlines and bool(re.search(
                r'(?:^|;)\s*text-decoration\s*:\s*overline\s*(?:;|$)',
                dict(attrs).get('style', ''), re.I))
            self.overline_spans.append(overline)
            if overline:
                self.append('\\overline{')
        if tag == 'sub' and self.math_subscripts:
            self.append('_')
        if tag == 'table':
            if self.table_stack and self.table_stack[-1]['cell'] is None:
                raise ValueError('Nested table must be inside a table cell')
            if self.table_stack:
                self.append(' ')
            self.table_stack.append({'rows': [], 'row': None, 'cell': None})
        elif tag == 'tr' and self.table_stack:
            self.table_stack[-1]['row'] = []
        elif tag in {'td', 'th'} and self.table_stack and self.table_stack[-1]['row'] is not None:
            spans = dict(attrs)
            cell = {'parts': [], 'rowspan': int(spans.get('rowspan', '1')),
                    'colspan': int(spans.get('colspan', '1'))}
            if cell['rowspan'] < 1 or cell['colspan'] < 1:
                raise ValueError('Table spans must be positive')
            self.table_stack[-1]['cell'] = cell
        elif tag == 'br':
            self.append(' ')
        elif tag not in {'thead', 'tbody', 'tfoot', 'caption', 'p', 'div', 'span',
                         'strong', 'b', 'em', 'i', 'u', 's', 'sub', 'sup', 'code',
                         'pre', 'ul', 'ol', 'li', 'blockquote', 'a', 'hr'}:
            # PDF placeholders such as <ea> are visible source content.
            self.append(self.get_starttag_text())
        for name, value in attrs:
            if name in {'alt', 'title', 'aria-label'}:
                self.append(value or '')

    def handle_endtag(self, tag):
        if tag == 'span' and self.overline_spans:
            if self.overline_spans.pop():
                self.append('}')
        if tag in {'td', 'th'} and self.table_stack and self.table_stack[-1]['cell'] is not None:
            table = self.table_stack[-1]
            cell = table['cell']
            table['row'].append({'text': text_tokens(''.join(cell.pop('parts'))), **cell})
            table['cell'] = None
            self.append(' ')
        elif tag == 'tr' and self.table_stack and self.table_stack[-1]['row'] is not None:
            table = self.table_stack[-1]
            table['rows'].append(table['row'])
            table['row'] = None
        elif tag == 'table' and self.table_stack:
            table = self.table_stack.pop()
            if table['cell'] is not None or table['row'] is not None:
                raise ValueError('Unclosed HTML table')
            if self.table_stack:
                self.table_stack[-1]['cell'].setdefault('tables', []).append(table['rows'])
                self.append(' ')
            else:
                self.tables.append(table['rows'])
        elif tag in {'p', 'div', 'li', 'pre', 'blockquote'}:
            self.append(' ')
        elif tag not in {'thead', 'tbody', 'tfoot', 'caption', 'strong', 'b', 'em',
                         'i', 'u', 's', 'sub', 'sup', 'code', 'ul', 'ol', 'a',
                         'span', 'td', 'th', 'tr', 'table'}:
            self.append(f'</{tag}>')

    def handle_data(self, data):
        if self.reject_raw_math and re.search(r'\\[A-Za-z]+|(?<!\\)\$[^$\s]{1,100}\$', data):
            raise ValueError('HTML table contains unrendered Markdown math')
        if self.math_source:
            data = re.sub(r'(?<!\\)\$([^$\n]+)\$',
                          lambda match: normalized_math_expression(match[1]), data)
        for literal, expression in self.math_literals:
            data = data.replace(literal, expression)
        self.append(data)

    def handle_startendtag(self, tag, attrs):
        self.handle_starttag(tag, attrs)

    def handle_comment(self, data):
        raise ValueError('HTML comments cannot carry verified content')


def inline_text(tokens, *, math_subscripts=False, math_literals=(), math_formatting=False):
    parts = []
    for token in tokens:
        if token.type == 'image':
            raise ValueError('Generated image links are not a text-preserving format conversion')
        if token.type == 'html_inline':
            parser = ContentHTML(math_subscripts=math_subscripts, math_literals=math_literals)
            parser.feed(token.content)
            parser.close()
            parts.append(''.join(parser.text))
        elif token.type in {'softbreak', 'hardbreak'}:
            parts.append(' ')
        elif token.children:
            parts.append(inline_text(token.children, math_subscripts=math_subscripts,
                                     math_literals=math_literals, math_formatting=math_formatting))
        elif token.type in {'math_inline', 'math_inline_double'} and math_formatting:
            parts.append(normalized_math_expression(token.content))
        elif token.type in {'math_inline', 'math_inline_double'} and math_subscripts:
            parts.append(re.sub(r'_\{([^{}]+)\}', r'_\1', token.content))
        elif token.type in {'text', 'code_inline', 'math_inline', 'math_inline_double'}:
            parts.append(token.content)
    return ''.join(parts)


def content_snapshot(text, representation='markdown', *, math_subscripts=False, math_literals=(),
                     math_formatting=False, math_overlines=False, reject_raw_math=False,
                     math_source=False):
    if representation == 'csv':
        parsed = PARSER.parse(text)
        if len(parsed) == 1 and parsed[0].type == 'fence' and parsed[0].info.strip() == 'csv':
            text = parsed[0].content
        rows = list(csv.reader(io.StringIO(text), strict=True))
        if not rows or any(len(row) != len(rows[0]) for row in rows):
            raise ValueError('CSV must contain a rectangular table')
        table = [[{'text': text_tokens(cell), 'rowspan': 1, 'colspan': 1} for cell in row] for row in rows]
        return {'text': [word for row in table for cell in row for word in cell['text']], 'tables': [table]}
    if representation == 'text':
        return {'text': text_tokens(text), 'tables': []}
    if representation not in {'markdown', 'html'}:
        raise ValueError('Expected markdown, html, csv or text representation')
    parts, tables, rows, row = [], [], None, None
    for token in PARSER.parse(text):
        if token.type == 'table_open':
            rows = []
        elif token.type == 'tr_open':
            row = []
        elif token.type == 'inline':
            value = inline_text(token.children or [], math_subscripts=math_subscripts,
                                math_literals=math_literals, math_formatting=math_formatting)
            parts.append(value)
            if row is not None:
                row.append({'text': text_tokens(value), 'rowspan': 1, 'colspan': 1})
        elif token.type == 'tr_close':
            rows.append(row)
            row = None
        elif token.type == 'table_close':
            tables.append(rows)
            rows = None
        elif token.type == 'html_block':
            parser = ContentHTML(math_subscripts=math_subscripts, math_literals=math_literals,
                                 math_overlines=math_overlines, reject_raw_math=reject_raw_math,
                                 math_source=math_source)
            parser.feed(token.content)
            parser.close()
            if parser.table_stack:
                raise ValueError('Unclosed HTML table')
            parts.append(''.join(parser.text))
            tables.extend(parser.tables)
        elif token.type in {'fence', 'code_block', 'math_block', 'math_block_label'}:
            parts.append(token.content)
    return {'text': text_tokens(' '.join(parts)), 'tables': tables}


def markdown_math_literals(text):
    def collect(tokens):
        for token in tokens:
            if token.type in {'math_inline', 'math_inline_double'}:
                delimiter = '$$' if token.type == 'math_inline_double' else '$'
                yield delimiter + token.content + delimiter, token.content
            yield from collect(token.children or [])

    return tuple(dict.fromkeys(collect(PARSER.parse(text))))


def validate_content(before, after, before_format='markdown', after_format='markdown'):
    source = content_snapshot(before, before_format)
    target = content_snapshot(after, after_format, reject_raw_math=after_format == 'html')
    source['text'] = significant_tokens(source['text'])
    target['text'] = significant_tokens(target['text'])
    if source['text'] != target['text'] and before_format == 'markdown' and after_format == 'html':
        math_literals = markdown_math_literals(before)
        if math_literals:
            # Markdown removes math delimiters, while the HTML parser sees
            # them as text. Normalize only expressions present in the source.
            math_target = content_snapshot(after, after_format, math_literals=math_literals)
            math_target['text'] = significant_tokens(math_target['text'])
            if source['text'] == math_target['text']:
                target = math_target
            else:
                # Math subscripts may also move from LaTeX to HTML.
                math_source = content_snapshot(before, before_format, math_subscripts=True)
                math_target = content_snapshot(after, after_format, math_subscripts=True,
                                               math_literals=math_literals)
                math_source['text'] = significant_tokens(math_source['text'])
                math_target['text'] = significant_tokens(math_target['text'])
                if math_source['text'] == math_target['text']:
                    source, target = math_source, math_target
        if source['text'] != target['text']:
            math_source = content_snapshot(before, before_format, math_formatting=True)
            math_target = content_snapshot(after, after_format, math_overlines=True,
                                           math_subscripts=True, reject_raw_math=True)
            math_source['text'] = significant_tokens(math_source['text'])
            math_target['text'] = significant_tokens(math_target['text'])
            if math_source['text'] == math_target['text']:
                source, target = math_source, math_target
    if source['text'] != target['text'] and before_format == after_format == 'html':
        math_source = content_snapshot(before, before_format, math_source=True)
        math_target = content_snapshot(after, after_format, math_overlines=True,
                                       math_subscripts=True, reject_raw_math=True)
        math_source['text'] = significant_tokens(math_source['text'])
        math_target['text'] = significant_tokens(math_target['text'])
        if math_source['text'] == math_target['text']:
            source, target = math_source, math_target
    if source['text'] != target['text']:
        for operation, a, b, c, d in SequenceMatcher(None, source['text'], target['text'], autojunk=False).get_opcodes():
            if operation != 'equal':
                raise ValueError('Codex changed content: expected ' + repr(' '.join(source['text'][a:b])[:180])
                                 + ', received ' + repr(' '.join(target['text'][c:d])[:180]))
    source_tables = source['tables']
    if source_tables != target['tables'] and before_format == 'markdown' and after_format == 'html':
        # A blank Markdown header is required syntax, not a source legend row.
        source_tables = [table[1:] if table and all(not cell['text'] for cell in table[0]) else table
                         for table in source_tables]
    if [significant_table(table) for table in source_tables] != [significant_table(table) for table in target['tables']]:
        # OCR text has no authored cell layout. Its text must still match exactly;
        # newly inferred layout can be reviewed by the user.
        if source['tables'] or not target['tables']:
            raise ValueError('Codex changed table cells, their order or row/column spans')
    return {'valid': True, 'text_tokens': len(source['text']), 'tables': len(target['tables'])}

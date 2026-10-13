"""Compare authored content across Markdown, HTML and CSV representations."""

import csv
from difflib import SequenceMatcher
from html.parser import HTMLParser
import io
import re

from .markdown import PARSER


def text_tokens(text):
    # Ignore layout whitespace without joining or splitting words. Preserve case,
    # numbers, punctuation, symbols and repetition, including their order.
    return re.findall(r'\w+|[^\w\s]', text, re.UNICODE)


class ContentHTML(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.text = []
        self.tables = []
        self.rows = None
        self.row = None
        self.cell = None

    def append(self, text):
        self.text.append(text)
        if self.cell is not None:
            self.cell['parts'].append(text)

    def handle_starttag(self, tag, attrs):
        if tag == 'table':
            if self.rows is not None:
                raise ValueError('Nested tables are not supported by content validation')
            self.rows = []
        elif tag == 'tr' and self.rows is not None:
            self.row = []
        elif tag in {'td', 'th'} and self.row is not None:
            spans = dict(attrs)
            self.cell = {'parts': [], 'rowspan': int(spans.get('rowspan', '1')),
                         'colspan': int(spans.get('colspan', '1'))}
            if self.cell['rowspan'] < 1 or self.cell['colspan'] < 1:
                raise ValueError('Table spans must be positive')
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
        if tag in {'td', 'th'} and self.cell is not None:
            self.row.append({'text': text_tokens(''.join(self.cell.pop('parts'))), **self.cell})
            self.cell = None
            self.text.append(' ')
        elif tag == 'tr' and self.row is not None:
            self.rows.append(self.row)
            self.row = None
        elif tag == 'table' and self.rows is not None:
            self.tables.append(self.rows)
            self.rows = None
        elif tag in {'p', 'div', 'li', 'pre', 'blockquote'}:
            self.append(' ')
        elif tag not in {'thead', 'tbody', 'tfoot', 'caption', 'strong', 'b', 'em',
                         'i', 'u', 's', 'sub', 'sup', 'code', 'ul', 'ol', 'a',
                         'span', 'td', 'th', 'tr', 'table'}:
            self.append(f'</{tag}>')

    def handle_data(self, data):
        self.append(data)

    def handle_startendtag(self, tag, attrs):
        self.handle_starttag(tag, attrs)

    def handle_comment(self, data):
        raise ValueError('HTML comments cannot carry verified content')


def inline_text(tokens):
    parts = []
    for token in tokens:
        if token.type == 'image':
            raise ValueError('Generated image links are not a text-preserving format conversion')
        if token.type == 'html_inline':
            parser = ContentHTML()
            parser.feed(token.content)
            parser.close()
            parts.append(''.join(parser.text))
        elif token.type in {'softbreak', 'hardbreak'}:
            parts.append(' ')
        elif token.children:
            parts.append(inline_text(token.children))
        elif token.type in {'text', 'code_inline', 'math_inline', 'math_inline_double'}:
            parts.append(token.content)
    return ''.join(parts)


def content_snapshot(text, representation='markdown'):
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
            value = inline_text(token.children or [])
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
            parser = ContentHTML()
            parser.feed(token.content)
            parser.close()
            if parser.rows is not None or parser.cell is not None:
                raise ValueError('Unclosed HTML table')
            parts.append(''.join(parser.text))
            tables.extend(parser.tables)
        elif token.type in {'fence', 'code_block', 'math_block', 'math_block_label'}:
            parts.append(token.content)
    return {'text': text_tokens(' '.join(parts)), 'tables': tables}


def validate_content(before, after, before_format='markdown', after_format='markdown'):
    source = content_snapshot(before, before_format)
    target = content_snapshot(after, after_format)
    if source['text'] != target['text']:
        for operation, a, b, c, d in SequenceMatcher(None, source['text'], target['text'], autojunk=False).get_opcodes():
            if operation != 'equal':
                raise ValueError('Codex changed content: expected ' + repr(' '.join(source['text'][a:b])[:180])
                                 + ', received ' + repr(' '.join(target['text'][c:d])[:180]))
    if source['tables'] != target['tables']:
        # OCR text has no authored cell layout. Its text must still match exactly;
        # newly inferred layout can be reviewed by the user.
        if source['tables'] or not target['tables']:
            raise ValueError('Codex changed table cells, their order or row/column spans')
    return {'valid': True, 'text_tokens': len(source['text']), 'tables': len(target['tables'])}

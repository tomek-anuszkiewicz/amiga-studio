"""Preserve source paragraph lines as portable Markdown hard breaks."""

from html import escape, unescape
from html.parser import HTMLParser
import re

from markdown_it import MarkdownIt
from markdown_it.rules_block.table import escapedSplit
from markdown_it.rules_inline import newline
from markdown_it.rules_inline.backticks import backtick
from markdown_it.token import Token
from mdit_py_plugins.dollarmath import dollarmath_plugin


def source_newline(state, silent):
    position = state.pos
    matched = newline(state, silent)
    if matched and not silent and state.tokens[-1].type == "softbreak":
        state.tokens[-1].meta["source_line"] = state.src.count("\n", 0, position)
    return matched


def source_backtick(state, silent):
    position, count = state.pos, len(state.tokens)
    matched = backtick(state, silent)
    if matched and not silent and len(state.tokens) > count and state.tokens[-1].type == 'code_inline':
        state.tokens[-1].meta['source_span'] = (position, state.pos)
    return matched


PARSER = MarkdownIt("commonmark", {"html": True}).enable("table").enable("strikethrough")
PARSER.use(dollarmath_plugin, allow_labels=False)
PARSER.inline.ruler.at("newline", source_newline)
PARSER.inline.ruler.at('backticks', source_backtick)


def _source_positions(text, expression, start, end, code=()):
    """Map inline source through continuation prefixes and table pipe escapes."""
    text = text.replace('\0', '\ufffd')
    pattern = ''.join(r'(\r?\n)[ \t>]*' if char == '\n' else
                      r'(\\?\|)' if char == '|' else '(' + re.escape(char) + ')'
                      for char in expression)
    match = next((match for match in re.finditer(pattern, text[start:end])
                  if not any(left <= start + match.start() and start + match.end() <= right
                             for left, right in code)), None)
    if match is None:
        raise ValueError('Cannot locate a parsed expression in the source text')
    return [start + match.end(group) - 1 for group in range(1, len(expression) + 1)]


def math_source_positions(text, tokens, code):
    """Map parsed math through continuation prefixes and table pipe escapes."""
    def expressions(entries):
        for token in entries:
            if token.type.startswith('math_'):
                yield token.markup + token.content + token.markup
            if token.children:
                yield from expressions(token.children)

    cursor = 0
    for expression in expressions(tokens):
        positions = _source_positions(text, expression, cursor, len(text), code)
        cursor = positions[-1] + 1
        yield expression, positions


def code_source_spans(text, tokens):
    """Map parser-recognized code without crossing inline container boundaries."""
    offsets = [0]
    for line in text.splitlines(keepends=True):
        offsets.append(offsets[-1] + len(line))
    code = [(offsets[token.map[0]], offsets[token.map[1]])
            for token in tokens
            if token.type in {'fence', 'code_block'} and token.map]
    cursor = 0
    for token in tokens:
        if token.type != 'inline' or not token.content:
            continue
        positions = _source_positions(text, token.content,
                                      max(cursor, offsets[token.map[0]]), offsets[token.map[1]])
        spans = [child.meta['source_span'] for child in token.children or []
                 if child.type == 'code_inline']
        code.extend((positions[start], positions[end - 1] + 1) for start, end in spans)
        cursor = positions[-1] + 1
    return code


def html_tag_spans(text):
    """Locate HTML markup outside Markdown code and math, keeping offsets."""
    tokens = PARSER.parse(text)
    code = code_source_spans(text, tokens)
    protected = code + [(positions[0], positions[-1] + 1)
                        for _, positions in math_source_positions(text, tokens, code)]
    pattern = r'<!--[\s\S]*?-->|</?([A-Za-z][A-Za-z0-9:_-]*)(?:\s+(?:[^<>\"\x27]|\"[^\"]*\"|\x27[^\x27]*\x27)*)?\s*/?>'
    for match in re.finditer(pattern, text):
        if not any(start <= match.start() < end for start, end in protected):
            yield match.span()


def callout_plugin(md):
    """Render Markdown callout markers as titled advisory blockquotes."""
    def transform(state):
        tokens = state.tokens
        for index in range(len(tokens) - 3, -1, -1):
            if tokens[index].type != 'blockquote_open' or tokens[index + 1].type != 'paragraph_open' or tokens[index + 2].type != 'inline':
                continue
            inline = tokens[index + 2]
            match = re.match(r'^\[!([A-Za-z][A-Za-z0-9_-]*)\][ \t]*([^\n]*)(?:\n|$)', inline.content)
            if not match:
                continue
            kind, title = match[1].lower(), match[2].strip() or match[1].upper()
            tokens[index].attrJoin('class', 'markdown-callout')
            tokens[index].attrSet('data-callout', kind)
            heading = Token('html_block', '', 0)
            heading.content = '<p class="callout-title">' + escape(title) + '</p>\n'
            inline.content = inline.content[match.end():]
            if inline.content:
                inline.children = []
                state.md.inline.parse(inline.content, state.md, state.env, inline.children)
            else:
                del tokens[index + 1:index + 4]
            tokens.insert(index + 1, heading)
    md.core.ruler.after('inline', 'callouts', transform)


def table_structure(text):
    """Validate one rectangular Markdown table and locate its data rows."""
    tokens = PARSER.parse(text)
    if not tokens or tokens[0].type != 'table_open' or tokens[-1].type != 'table_close':
        raise ValueError('Expected a single Markdown table without surrounding text')
    tables = [token for token in tokens if token.type == 'table_open']
    if len(tables) != 1:
        raise ValueError('Expected exactly one Markdown table')
    start, end = tables[0].map
    lines = text.splitlines(keepends=True)
    if ''.join(lines[:start]).strip() or ''.join(lines[end:]).strip():
        raise ValueError('Table contains surrounding text')
    headers, alignments = [], []
    in_header = False
    for token in tokens:
        if token.type == 'thead_open':
            in_header = True
        elif token.type == 'thead_close':
            in_header = False
        elif in_header and token.type == 'th_open':
            alignments.append(token.attrGet('style'))
        elif in_header and token.type == 'inline':
            rendered = PARSER.renderInline(token.content)
            headers.append(' '.join(unescape(re.sub(r'<[^>]*>', ' ', rendered)).split()))
    for line in lines[start + 2:end]:
        cells = escapedSplit(line.strip())
        if cells and cells[0] == '':
            cells.pop(0)
        if cells and cells[-1] == '':
            cells.pop()
        if len(cells) != len(headers):
            raise ValueError('Table row has a different number of columns')
    if end <= start + 2:
        raise ValueError('Table has no data rows to merge')
    return headers, alignments, sum(map(len, lines[:start + 2]))


class _HTMLTableColumns(HTMLParser):
    def __init__(self):
        super().__init__()
        self.depth = 0
        self.widths = []
        self.cell_counts = []
        self.width = None
        self.cell_count = None
        self.first_row_has_header = False
        self.first_row_end = None
        self.header_section_end = None
        self.has_body = False

    def handle_starttag(self, tag, attrs):
        if tag == 'table':
            self.depth += 1
        elif self.depth == 1 and tag == 'tbody':
            self.has_body = True
        elif self.depth == 1 and tag == 'tr':
            self.width = 0
            self.cell_count = 0
        elif self.depth == 1 and tag in {'td', 'th'} and self.width is not None:
            span = int(dict(attrs).get('colspan', '1'))
            if span < 1:
                raise ValueError('Table column spans must be positive')
            self.width += span
            self.cell_count += 1
            if tag == 'th' and not self.widths:
                self.first_row_has_header = True

    def handle_endtag(self, tag):
        if tag == 'table':
            self.depth -= 1
        elif self.depth == 1 and tag == 'thead':
            self.header_section_end = self.getpos()
        elif self.depth == 1 and tag == 'tr' and self.width is not None:
            if not self.widths:
                self.first_row_end = self.getpos()
            self.widths.append(self.width)
            self.cell_counts.append(self.cell_count)
            self.width = None
            self.cell_count = None


def html_table_structure(text):
    """Locate data rows inside one HTML table and count first-row cell groups."""
    opening = re.match(r'\s*<table\b[^>]*>\s*', text, re.I)
    if not opening:
        raise ValueError('Expected a single HTML table')
    closing = re.search(r'(?:</tbody>\s*)?</table>\s*$', text, re.I)
    if not closing:
        raise ValueError('Expected a single HTML table')
    rows = text[opening.end():closing.start()]
    parser = _HTMLTableColumns()
    parser.feed(text)
    parser.close()
    if parser.has_body and not re.match(r'</tbody>', closing[0], re.I):
        raise ValueError('Expected a single HTML table')
    if parser.depth or not parser.widths or not any(parser.widths):
        raise ValueError('Expected a single HTML table with rows')
    body_start = opening.end()
    lines = text.splitlines(keepends=True)

    def after_tag(position, tag):
        line, column = position
        offset = sum(map(len, lines[:line - 1])) + column
        match = re.match(rf'</{tag}>\s*', text[offset:], re.I)
        if not match:
            raise ValueError(f'Expected an HTML {tag} closing tag')
        return offset + match.end()

    if parser.header_section_end is not None:
        body_start = after_tag(parser.header_section_end, 'thead')
    body = re.match(r'<tbody\b[^>]*>\s*', text[body_start:], re.I)
    if body:
        body_start += body.end()
    if parser.header_section_end is None and parser.first_row_has_header:
        body_start = after_tag(parser.first_row_end, 'tr')
    return parser.cell_counts[0], body_start, closing.start()


TABLE_TAG = re.compile(r'</?(?:table|thead|tbody|tfoot|tr|th|td)\b[^>]*>', re.I)


def format_html_table(text):
    """Indent table structure while leaving cell contents untouched."""
    parts = []
    depth = cell_depth = nested_depth = position = 0
    for match in TABLE_TAG.finditer(text):
        gap = text[position:match.start()]
        tag = match[0]
        name = re.match(r'</?([A-Za-z]+)', tag)[1].lower()
        closing = tag.startswith('</')
        if nested_depth or (cell_depth and name == 'table' and not closing):
            parts.append(gap + tag)
            if name == 'table':
                nested_depth += -1 if closing else 1
            position = match.end()
            continue
        if cell_depth or gap.strip():
            parts.append(gap)
        if closing:
            depth -= 1
            if name in {'td', 'th'}:
                parts.append(tag)
                cell_depth -= 1
            else:
                if parts and not parts[-1].endswith('\n'):
                    parts.append('\n')
                parts.append('  ' * depth + tag)
        else:
            if parts and not parts[-1].endswith('\n'):
                parts.append('\n')
            parts.append('  ' * depth + tag)
            depth += 1
            if name in {'td', 'th'}:
                cell_depth += 1
        position = match.end()
    tail = text[position:]
    if cell_depth or tail.strip():
        parts.append(tail)
    return ''.join(parts)


def preserve_line_breaks(text):
    """Mark soft breaks only; block syntax, code, math and HTML keep their layout."""
    lines = text.splitlines(keepends=True)
    breaks = set()
    for token in PARSER.parse(text):
        if token.type == "inline" and token.map:
            for child in token.children or []:
                if child.type == "softbreak":
                    breaks.add(token.map[0] + child.meta["source_line"])
    for index in breaks:
        line = lines[index]
        ending = "\r\n" if line.endswith("\r\n") else line[-1:]
        lines[index] = line[:-len(ending)] + "\\" + ending
    return "".join(lines)

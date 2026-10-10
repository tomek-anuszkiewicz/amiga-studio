"""Preserve source paragraph lines as portable Markdown hard breaks."""

from html import escape, unescape
import re

from markdown_it import MarkdownIt
from markdown_it.rules_block.table import escapedSplit
from markdown_it.rules_inline import newline
from markdown_it.token import Token
from mdit_py_plugins.dollarmath import dollarmath_plugin


def source_newline(state, silent):
    position = state.pos
    matched = newline(state, silent)
    if matched and not silent and state.tokens[-1].type == "softbreak":
        state.tokens[-1].meta["source_line"] = state.src.count("\n", 0, position)
    return matched


PARSER = MarkdownIt("commonmark", {"html": True}).enable("table").enable("strikethrough")
PARSER.use(dollarmath_plugin, allow_labels=False)
PARSER.inline.ruler.at("newline", source_newline)


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

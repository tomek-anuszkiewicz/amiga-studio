"""Preserve source paragraph lines as portable Markdown hard breaks."""

from markdown_it import MarkdownIt
from markdown_it.rules_inline import newline
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

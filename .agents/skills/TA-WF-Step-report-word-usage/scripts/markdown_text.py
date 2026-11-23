"""Extract reportable Markdown text while retaining source positions."""

import re

from markdown_it import MarkdownIt
from markdown_it.rules_block import reference
from markdown_it.rules_inline import (
    autolink, backticks, emphasis, entity, escape, html_inline, image, link, strikethrough,
)


WIKI_LINK = re.compile(r"!?\[\[([^\]\n]+)\]\]")
HTML_TAG = re.compile(r"</?([A-Za-z][A-Za-z0-9-]*)")
HTML_FORMATTING = {
    "a", "abbr", "b", "bdi", "bdo", "cite", "code", "data", "del", "dfn", "em",
    "font", "i", "ins", "kbd", "label", "mark", "s", "samp", "small", "span",
    "strike", "strong", "sub", "sup", "time", "tt", "u", "var",
}
FORMATTING_TOKENS = {"em_open", "em_close", "strong_open", "strong_close", "s_open", "s_close"}


def blank_span(text):
    return re.sub(r"[^\n]", " ", text)


def hide(state, start, end, *, join=False):
    """Hide source columns, optionally joining the visible text on either side."""
    state.env["visible"][start:end] = [""] * (end - start) if join else blank_span(state.src[start:end])


def observe(rule, record):
    """Capture the source range consumed by a successful inline rule."""
    def observed(state, silent):
        start = state.pos
        token_count = len(state.tokens)
        if not rule(state, silent):
            return False
        if not silent and state.src is state.env["source"]:
            record(state, start, token_count)
        return True

    return observed


def hide_consumed(state, start, _token_count):
    hide(state, start, state.pos)


def hide_html(state, start, _token_count):
    # Markdown-it has accepted the complete tag. Only identify its element
    # here so layout and replaced elements keep a word boundary.
    tag = HTML_TAG.match(state.tokens[-1].content)
    join = (
        state.env["block_type"] in {"inline", "html_block"}
        and tag is not None
        and tag[1].lower() in HTML_FORMATTING
    )
    hide(state, start, state.pos, join=join)


def keep_decoded_text(state, start, _token_count):
    if state.env["block_type"] != "inline":
        return
    token = state.tokens[-1]
    if token.type == "text_special" and token.content != token.markup:
        # Store the parser's text in the first source column of the entity or
        # escape. Empty slots keep the other columns available for alignment.
        state.env["visible"][start:state.pos] = [token.content] + [""] * (state.pos - start - 1)
    elif token.type == "hardbreak":
        hide(state, start, state.pos)


def keep_code_word(state, start, token_count):
    if len(state.tokens) == token_count or state.tokens[-1].type != "code_inline":
        return
    token = state.tokens[-1]
    width = len(token.markup)
    body = state.src[start + width:state.pos - width]
    if re.fullmatch(r"[A-Za-z]+", body.strip(" \t")):
        join = state.env["block_type"] == "inline"
        if join:
            # A qualifying single-line word may lose one space at each end
            # as part of CommonMark's code span normalization.
            width += (len(body) - len(token.content)) // 2
        hide(state, start, start + width, join=join)
        hide(state, state.pos - width, state.pos, join=join)
    else:
        hide(state, start, state.pos)


def keep_link_label(state, start, _token_count):
    is_image = state.src[start] == "!"
    opening = start + is_image
    end = state.md.helpers.parseLinkLabel(state, opening, not is_image)
    block_type = state.env["block_type"]
    if not is_image and block_type == "inline":
        hide(state, start, opening + 1, join=True)
        hide(state, end, state.pos, join=True)
    else:
        hide(state, end + 1, state.pos)
    if is_image:
        # Image alt text is parsed in its own string. Apply its mask at the
        # known label offset because its inline positions start at zero.
        label = state.src[opening + 1:end]
        visible, _ = visible_inline(state.md, label, state.env, block_type=block_type)
        state.env["visible"][opening + 1:end] = visible


def mark_delimiters(state, start, token_count):
    position = state.pos
    for token in reversed(state.tokens[token_count:]):
        position -= len(token.content)
        token.meta["source_position"] = position
        if position == start:
            break


def hide_formatting(state):
    if state.src is not state.env["source"]:
        return
    join = state.env["block_type"] == "inline"
    for token in state.tokens:
        if token.type not in FORMATTING_TOKENS or (not join and token.markup not in {"_", "__"}):
            continue
        start = token.meta["source_position"]
        width = len(token.markup)
        # The emphasis rule puts strong_open on its second delimiter token.
        if token.type == "strong_open":
            start -= 1
        hide(state, start, start + width, join=join)


def wiki_link(state, silent):
    match = WIKI_LINK.match(state.src, state.pos, state.posMax)
    if match is None:
        return False
    if not silent and state.src is state.env["source"]:
        target, separator, _alias = match[1].partition("|")
        if separator:
            hide(state, match.start(), match.start(1) + len(target) + 1)
        elif any(char in target for char in "/\\#") or re.search(r"\.\w+$", target):
            hide(state, match.start(), match.end())
    state.pos = match.end()
    return True


def comment(state, silent):
    if not state.src.startswith("<!--", state.pos):
        return False
    end = state.src.find("-->", state.pos + 4, state.posMax)
    if not silent and state.src is state.env["source"]:
        if end < 0:
            state.env["open_comment"] = state.pos
        join = end >= 0 and state.env["block_type"] in {"inline", "html_block"}
        hide(state, state.pos, state.posMax if end < 0 else end + 3, join=join)
    state.pos = state.posMax if end < 0 else end + 3
    return True


def parser(literal=False):
    md = MarkdownIt("commonmark").disable("inline")
    # The report also excludes definitions immediately after prose. Use the
    # library's reference rule at paragraph boundaries for these files.
    md.block.ruler.at("reference", reference, {"alt": ["paragraph"]})
    # The report reads labels and never follows or renders link destinations.
    md.validateLink = lambda _url: True
    md.inline.ruler.before("link", "wiki_link", wiki_link)
    for name, rule, record in (
        ("autolink", autolink, hide_consumed),
        ("backticks", backticks.backtick, keep_code_word),
        ("emphasis", emphasis.tokenize, mark_delimiters),
        ("entity", entity, keep_decoded_text),
        ("escape", escape, keep_decoded_text),
        ("html_inline", html_inline, hide_html),
        ("image", image, keep_link_label),
        ("link", link, keep_link_label),
        ("strikethrough", strikethrough.tokenize, mark_delimiters),
    ):
        md.inline.ruler.at(name, observe(rule, record))
    md.inline.ruler2.after("emphasis", "hide_formatting", hide_formatting)
    if literal:
        # Keep fence markers and backticks literal while filtering links.
        md.disable(["backticks", "html_inline", "blockquote", "code", "fence",
                    "heading", "hr", "html_block", "lheading", "list"])
    else:
        md.inline.ruler.before("html_inline", "comment", comment)
        md.enable("strikethrough")
    return md


MARKDOWN = parser()
FENCED_MARKDOWN = parser(literal=True)


def visible_inline(md, source, env, *, block_type):
    inline_env = {"source": source, "visible": list(source), "block_type": block_type,
                  "references": env.get("references", {})}
    md.inline.parse(source, md, inline_env, [])
    return inline_env["visible"], inline_env.get("open_comment")


def inline_lines(md, body, env, source_lines, *, block_type):
    slots, open_comment = visible_inline(md, body, env, block_type=block_type)
    if open_comment is not None:
        return [], open_comment
    visible = []
    start = 0
    for line, source in zip(body.split("\n"), source_lines, strict=True):
        end = start + len(line)
        joins_next = end < len(slots) and slots[end] == ""
        visible.append((align_line(line, slots[start:end], source), joins_next))
        start = end + 1
    return visible, None


def visible_block(token, body, env, source_lines):
    if token.type != "fence":
        return inline_lines(MARKDOWN, body, env, source_lines, block_type=token.type)

    # A fenced example can contain Markdown links split across lines. Parse
    # its paragraphs and definitions without interpreting any inner fences.
    example_env = {"references": dict(env.get("references", {}))}
    lines = body.split("\n")
    visible = [([], False) for _ in lines]
    for part in FENCED_MARKDOWN.parse(token.content, example_env):
        if part.type == "inline":
            start, end = part.map
            visible[start:end], _ = inline_lines(FENCED_MARKDOWN, part.content, example_env,
                                                 lines[start:end], block_type="fence")
    return [(align_line(line, kept, source), joins_next)
            for line, (kept, joins_next), source in zip(lines, visible, source_lines, strict=True)], None


def source_column(line, fragment):
    # Search within the mapped source line, skipping container prefixes.
    return line.replace("\0", "\ufffd").rindex(fragment)


def align_line(line, slots, source):
    """Restore source columns after the block parser strips the container."""
    fragment = line.lstrip(" \t")
    if not fragment:
        return []
    # The block parser can add leading spaces when consuming part of a tab.
    column = source_column(source, fragment)
    return [""] * column + slots[len(line) - len(fragment):]


def hide_continuing_comment(lines, first, body, opening):
    line = first + body.count("\n", 0, opening)
    fragment = body[opening:].partition("\n")[0]
    column = source_column(lines[line], fragment)
    remainder = "\n".join(lines[line:])
    close = remainder.find("-->", column + 4)
    end = len(remainder) if close < 0 else close + 3
    lines[line:] = (remainder[:column] + blank_span(remainder[column:end]) + remainder[end:]).split("\n")


def source_slots(lines):
    """Return source-aligned characters and whether each newline is hidden."""
    start = 0
    if lines and lines[0].rstrip() == "---":
        start = next((number + 1 for number, line in enumerate(lines[1:], 1)
                      if line.rstrip() in {"---", "..."}), 0)

    source = lines[start:]
    while True:
        visible = [([], False) for _ in source]
        env = {}
        for token in MARKDOWN.parse("\n".join(source), env):
            if not token.content or token.type not in {"code_block", "fence", "html_block", "inline"}:
                continue
            if token.type == "fence" and token.markup.startswith("~"):
                continue
            first = token.map[0] + (token.type == "fence")
            body = token.content.removesuffix("\n")
            last = first + body.count("\n") + 1
            kept, open_comment = visible_block(token, body, env, source[first:last])
            if open_comment is not None:
                # An inline comment can span blocks in the report. Hide it and
                # let Markdown-it classify the surrounding blocks again.
                hide_continuing_comment(source, first, body, open_comment)
                break
            visible[first:last] = kept
        else:
            return [([], False) for _ in range(start)] + visible


def prose_lines(lines):
    """Yield visible lines with a source line index and column for each character."""
    parts, positions = [], []
    for number, (slots, joins_next) in enumerate(source_slots(lines)):
        parts.extend(slots)
        positions.extend((number, column) for column, text in enumerate(slots) for _ in text)
        if not joins_next:
            yield "".join(parts), positions
            parts, positions = [], []

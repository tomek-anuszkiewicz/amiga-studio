"""Extract reportable Markdown text while retaining source positions."""

import re

from markdown_it import MarkdownIt
from markdown_it.rules_block import reference
from markdown_it.rules_inline import autolink, backticks, emphasis, entity, escape, html_inline, image, link


WIKI_LINK = re.compile(r"!?\[\[([^\]\n]+)\]\]")


def blank_span(text):
    return re.sub(r"[^\n]", " ", text)


def hide(state, start, end):
    state.env["visible"][start:end] = blank_span(state.src[start:end])


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


def keep_decoded_text(state, start, _token_count):
    if not state.env["decode_text"]:
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
    width = len(state.tokens[-1].markup)
    body = state.src[start + width:state.pos - width]
    if re.fullmatch(r"[A-Za-z]+", body.strip(" \t")):
        hide(state, start, start + width)
        hide(state, state.pos - width, state.pos)
    else:
        hide(state, start, state.pos)


def keep_link_label(state, start, _token_count):
    is_image = state.src[start] == "!"
    opening = start + is_image
    end = state.md.helpers.parseLinkLabel(state, opening, not is_image)
    hide(state, end + 1, state.pos)
    if is_image:
        # Image alt text is parsed in its own string. Apply its mask at the
        # known label offset because its inline positions start at zero.
        label = state.src[opening + 1:end]
        visible, _ = visible_inline(state.md, label, state.env, decode_text=state.env["decode_text"])
        state.env["visible"][opening + 1:end] = visible


def mark_emphasis(state, start, _token_count):
    for position, token in zip(range(start, state.pos), state.tokens[-(state.pos - start):], strict=True):
        token.meta["source_position"] = position


def hide_emphasis(state):
    if state.src is not state.env["source"]:
        return
    for token in state.tokens:
        if token.markup in {"_", "__"}:
            position = token.meta["source_position"]
            width = len(token.markup)
            start = position - width + 1 if token.nesting == 1 else position
            hide(state, start, start + width)


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
        hide(state, state.pos, state.posMax if end < 0 else end + 3)
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
        ("emphasis", emphasis.tokenize, mark_emphasis),
        ("entity", entity, keep_decoded_text),
        ("escape", escape, keep_decoded_text),
        ("html_inline", html_inline, hide_consumed),
        ("image", image, keep_link_label),
        ("link", link, keep_link_label),
    ):
        md.inline.ruler.at(name, observe(rule, record))
    md.inline.ruler2.after("emphasis", "hide_emphasis", hide_emphasis)
    if literal:
        # Keep fence markers and backticks literal while filtering links.
        md.disable(["backticks", "html_inline", "blockquote", "code", "fence",
                    "heading", "hr", "html_block", "lheading", "list"])
    else:
        md.inline.ruler.before("html_inline", "comment", comment)
    return md


MARKDOWN = parser()
FENCED_MARKDOWN = parser(literal=True)


def visible_inline(md, source, env, *, decode_text):
    inline_env = {"source": source, "visible": list(source), "decode_text": decode_text,
                  "references": env.get("references", {})}
    md.inline.parse(source, md, inline_env, [])
    return inline_env["visible"], inline_env.get("open_comment")


def inline_lines(md, body, env, source_lines, *, decode_text):
    slots, open_comment = visible_inline(md, body, env, decode_text=decode_text)
    if open_comment is not None:
        return [], open_comment
    visible = []
    start = 0
    for line, source in zip(body.split("\n"), source_lines, strict=True):
        end = start + len(line)
        visible.append(align_line(line, slots[start:end], source))
        start = end + 1
    return visible, None


def visible_block(token, body, env, source_lines):
    if token.type != "fence":
        return inline_lines(MARKDOWN, body, env, source_lines, decode_text=token.type == "inline")

    # A fenced example can contain Markdown links split across lines. Parse
    # its paragraphs and definitions without interpreting any inner fences.
    example_env = {"references": dict(env.get("references", {}))}
    lines = body.split("\n")
    visible = [[] for _ in lines]
    for part in FENCED_MARKDOWN.parse(token.content, example_env):
        if part.type == "inline":
            start, end = part.map
            visible[start:end], _ = inline_lines(FENCED_MARKDOWN, part.content, example_env,
                                                 lines[start:end], decode_text=False)
    return [align_line(line, kept, source)
            for line, kept, source in zip(lines, visible, source_lines, strict=True)], None


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
    return [" "] * column + slots[len(line) - len(fragment):]


def hide_continuing_comment(lines, first, body, opening):
    line = first + body.count("\n", 0, opening)
    fragment = body[opening:].partition("\n")[0]
    column = source_column(lines[line], fragment)
    remainder = "\n".join(lines[line:])
    close = remainder.find("-->", column + 4)
    end = len(remainder) if close < 0 else close + 3
    lines[line:] = (remainder[:column] + blank_span(remainder[column:end]) + remainder[end:]).split("\n")


def prose_lines(lines):
    """Return visible text indexed by original line and column."""
    start = 0
    if lines and lines[0].rstrip() == "---":
        start = next((number + 1 for number, line in enumerate(lines[1:], 1)
                      if line.rstrip() in {"---", "..."}), 0)

    source = lines[start:]
    while True:
        visible = [[] for _ in source]
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
            return [[] for _ in range(start)] + visible

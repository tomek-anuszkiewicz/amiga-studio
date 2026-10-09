"""Single-pass templates preserving PDF text, with separate table-markup import."""

from difflib import SequenceMatcher
import re
from markdown_it import MarkdownIt

REFERENCE = re.compile(r"\{\{text page=(\d+) block=(\d+) chars=(\d+):(\d+)\}\}")
MARKDOWN_REFERENCE = re.compile(r"\{\{markdown stage=(04|05) page=(\d+) segment=(page_\d+_seg_\d+)\}\}")


def escape_literal(text):
    return text.replace("\\", "\\\\").replace("{", "\\{").replace("}", "\\}")


def compile_template(page, blocks):
    """Reference whole selected blocks without interpreting any source character."""
    return "".join("{{text " + f"page={page} block={block['block_id']} chars=0:{len(block['text'])}" + "}}"
                   for block in blocks if block["text"])


def compile_region_template(page, page_data, bbox, markdown=""):
    """Reference contiguous character ranges whose centers lie in the element region."""
    a, b, c, d, e, f = page_data["raster"]["display_to_pixels"]
    parts = []
    for block in region_blocks(page_data, bbox):
        boxes = block.get("char_bboxes")
        if boxes is None or len(boxes) != len(block["text"]):
            raise ValueError("Character geometry is missing or stale; regenerate Stage 02 before importing")
        selected_chars = []
        for x0, y0, x1, y1 in boxes:
            x, y = (x0 + x1) / 2, (y0 + y1) / 2
            px, py = a*x + c*y + e, b*x + d*y + f
            selected_chars.append(bbox[0] <= px < bbox[2] and bbox[1] <= py < bbox[3])
        # PDF boxes can put trailing spaces beyond a tight visual region. Keep
        # whitespace with the preceding text, and leading indentation with its line.
        for index, char in enumerate(block["text"]):
            if char.isspace():
                if index and block["text"][index - 1] != "\n":
                    selected_chars[index] = selected_chars[index - 1]
                else:
                    following = next((j for j in range(index + 1, len(boxes))
                                      if not block["text"][j].isspace() or block["text"][j] == "\n"), None)
                    if following is not None and block["text"][following] != "\n":
                        selected_chars[index] = selected_chars[following]
        start = None
        for index in range(len(boxes) + 1):
            selected = index < len(boxes) and selected_chars[index]
            if selected and start is None:
                start = index
            elif not selected and start is not None:
                parts.append("{{text " + f"page={page} block={block['block_id']} chars={start}:{index}" + "}}")
                start = None
    template = "".join(parts)
    return preserve_bold(template, {page: page_data["blocks"]}, markdown)


def preserve_bold(template, pages, markdown):
    """Project Markdown strong spans onto PDF references, preserving source bytes."""
    children = MarkdownIt('commonmark').parseInline(markdown)[0].children or []
    plain, spans, starts = "", [], []
    for token in children:
        if token.type == "strong_open":
            starts.append(len(plain))
        elif token.type == "strong_close":
            spans.append((starts.pop(), len(plain)))
        elif token.type in {"softbreak", "hardbreak"}:
            plain += "\n"
        elif not token.children:
            plain += token.content
    if not spans:
        return template
    source = resolve_template(template, pages)
    mapping = {}
    for block in SequenceMatcher(None, plain, source, autojunk=False).get_matching_blocks():
        for offset in range(block.size):
            mapping[block.a + offset] = block.b + offset
    markers = {}
    for start, end in spans:
        # Both boundaries must be grounded in matching source characters.
        if start < end and start in mapping and end - 1 in mapping:
            a, b = mapping[start], mapping[end - 1] + 1
            markers[a] = markers.get(a, "") + "**"
            markers[b] = markers.get(b, "") + "**"
    parts, position = [], 0
    for match in REFERENCE.finditer(template):
        page, block, start, end = map(int, match.groups())
        length = end - start
        cuts = [position] + sorted(p for p in markers if position < p < position + length) + [position + length]
        for a, b in zip(cuts, cuts[1:]):
            parts.append(markers.get(a, ""))
            parts.append("{{text " + f"page={page} block={block} chars={start+a-position}:{start+b-position}" + "}}")
        position += length
    parts.append(markers.get(position, ""))
    return "".join(parts)


def compile_markdown_template(page, blocks, target):
    """Keep markup literal and map text to exact PDF ranges, allowing reordering.

    Unmatched transcription/corrections remain explicit literals. Matching proves
    character equality, not semantic attribution between identical source strings.
    """
    def reference_text(text):
        best = None
        for block in blocks:
            match = SequenceMatcher(None, block['text'], text, autojunk=False).find_longest_match()
            if match.size and any(char.isalnum() for char in text[match.b:match.b + match.size]):
                if best is None or match.size > best[1].size:
                    best = block, match
        if best is None:
            return escape_literal(text)
        block, match = best
        reference = '{{text ' + f'page={page} block={block["block_id"]} chars={match.a}:{match.a + match.size}' + '}}'
        return (reference_text(text[:match.b]) + reference
                + reference_text(text[match.b + match.size:]))

    parts, fence = [], None
    for line in target.splitlines(keepends=True):
        marker = re.match(r'^ {0,3}(`{3,}|~{3,})(.*)$', line.rstrip('\r\n'))
        if marker and fence is None:
            fence = marker[1]
            parts.append(escape_literal(line))
        elif marker and marker[1][0] == fence[0] and len(marker[1]) >= len(fence) and not marker[2].strip():
            fence = None
            parts.append(escape_literal(line))
        elif fence is not None:
            parts.append(reference_text(line))
        else:
            # HTML tags/attributes describe cells and spans, not PDF text.
            position = 0
            for tag in re.finditer(r'</?[A-Za-z][^>]*>', line):
                parts.append(reference_text(line[position:tag.start()]))
                parts.append(escape_literal(tag[0]))
                position = tag.end()
            parts.append(reference_text(line[position:]))
    return ''.join(parts)


def resolve_template(template, pages, prepared=None):
    """Only template syntax is parsed; expanded PDF text is never parsed again."""
    blocks = {page: {b["block_id"]: b["text"] for b in entries} for page, entries in pages.items()}
    parts, position = [], 0
    while position < len(template):
        char = template[position]
        if char == "\\":
            if position + 1 >= len(template) or template[position + 1] not in "\\{}":
                raise ValueError("Invalid text-template escape")
            parts.append(template[position + 1])
            position += 2
        elif char == "{":
            converted = MARKDOWN_REFERENCE.match(template, position)
            if converted:
                stage, page, segment = converted.groups()
                try:
                    parts.append(prepared[(stage, int(page), segment)])
                except (KeyError, TypeError) as error:
                    raise ValueError("Markdown reference points to a missing prepared segment") from error
                position = converted.end()
                continue
            match = REFERENCE.match(template, position)
            if not match:
                raise ValueError("Invalid text reference; escape literal braces")
            page, block, start, end = map(int, match.groups())
            try:
                text = blocks[page][block]
            except KeyError as error:
                raise ValueError("Text reference points to a missing page or block") from error
            if not 0 <= start < end <= len(text):
                raise ValueError("Text reference range is outside its source block")
            parts.append(text[start:end])
            position = match.end()
        elif char == "}":
            raise ValueError("Escape literal closing braces")
        else:
            parts.append(char)
            position += 1
    return "".join(parts)


def region_blocks(page_data, bbox):
    """Select intersecting positioned blocks as candidates, in extraction order."""
    a, b, c, d, e, f = page_data["raster"]["display_to_pixels"]
    selected = []
    for block in page_data["blocks"]:
        x0, y0, x1, y1 = block["bbox"]
        corners = [(a*x + c*y + e, b*x + d*y + f) for x, y in ((x0, y0), (x1, y1))]
        left, right = sorted(p[0] for p in corners)
        top, bottom = sorted(p[1] for p in corners)
        if max(left, bbox[0]) < min(right, bbox[2]) and max(top, bbox[1]) < min(bottom, bbox[3]):
            selected.append(block)
    return selected

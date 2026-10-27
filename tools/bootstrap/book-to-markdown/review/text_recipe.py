"""PDF text recipe construction, resolution, editing, and source differences."""

from difflib import SequenceMatcher
import json
import re
from markdown_it import MarkdownIt

REFERENCE = re.compile(r'\{\{text page=(\d+) block=(\d+) chars=(\d+):(\d+)'
                       r'(?: replace=\d+:\d+ with="(?:\\.|[^"\\])*"| case="(?:upper|lower)")?\}\}')
CORRECTION = re.compile(r' replace=(\d+):(\d+) with=("(?:\\.|[^"\\])*")\}\}$')
CASE = re.compile(r' case="(upper|lower)"\}\}$')
ACCEPTED = re.compile(r'\{\{accepted text=("(?:\\.|[^"\\])*")\}\}')
LIGATURES = {'ﬀ': 'ff', 'ﬁ': 'fi', 'ﬂ': 'fl', 'ﬃ': 'ffi', 'ﬄ': 'ffl'}


def accepted_literal(value):
    return '{{accepted text=' + json.dumps(value, ensure_ascii=False) + '}}'


def source_text(match, text):
    """Resolve one reference and validate its source-relative substitution."""
    page, block, start, end = map(int, match.groups())
    if not 0 <= start < end <= len(text):
        raise ValueError("Text reference range is outside its source block")
    value = text[start:end]
    case = CASE.search(match[0])
    if case:
        convert = str.upper if case[1] == 'upper' else str.lower
        transformed = convert(value)
        if len(transformed) != len(value) or transformed != ''.join(map(convert, value)):
            raise ValueError("Case transformation does not preserve source character positions")
        return transformed
    correction = CORRECTION.search(match[0])
    if correction:
        left, right = int(correction[1]), int(correction[2])
        replacement = json.loads(correction[3])
        if (not isinstance(replacement, str) or not 0 <= left < right <= len(value) or
                (len(replacement) != right - left and (left != 0 or right != len(value)) and
                 ''.join(LIGATURES.get(char, char) for char in value[left:right]) != replacement)):
            raise ValueError("Text correction range or length is invalid")
        value = value[:left] + replacement + value[right:]
    return value


def source_reference(page, block, start, end, correction=None, *, case=None):
    value = f'{{{{text page={page} block={block} chars={start}:{end}'
    if correction is not None:
        left, right, replacement = correction
        value += f' replace={left}:{right} with={json.dumps(replacement, ensure_ascii=False)}'
    if case is not None:
        if case not in {'upper', 'lower'} or correction is not None:
            raise ValueError("Invalid case transformation")
        value += f' case="{case}"'
    return value + '}}'


def source_span_with_spacing(page, block, start, end, visible):
    """Localize sparse OCR differences within one already selected source span."""
    source = block['text'][start:end]
    if source == visible:
        return source_reference(page, block['block_id'], start, end)
    for mode, convert in (('upper', str.upper), ('lower', str.lower)):
        if (visible == convert(source) and len(visible) == len(source) and
                visible == ''.join(map(convert, source))):
            return source_reference(page, block['block_id'], start, end, case=mode)
    if re.fullmatch(r'[A-Za-z](?:[ \t]+[A-Za-z])+', source):
        letters = [(index, char) for index, char in enumerate(source) if char.isalpha()]
        compact = ''.join(char for _, char in letters)
        for mode, convert in (('upper', str.upper), ('lower', str.lower)):
            if visible == convert(compact) and visible != compact:
                return ''.join(source_reference(page, block['block_id'],
                                                start + index, start + index + 1,
                                                case=mode if char != convert(char) else None)
                               for index, char in letters)
    same_nonspace = (''.join(char for char in source if not char.isspace()) ==
                     ''.join(char for char in visible if not char.isspace()))
    matcher = SequenceMatcher(None, source, visible, autojunk=False)
    operations = matcher.get_opcodes()
    changed = sum(max(right - left, target_right - target_left)
                  for kind, left, right, target_left, target_right in operations
                  if kind != 'equal')
    if not same_nonspace:
        if (len(source) == len(visible) and matcher.ratio() >= .5 and
                changed <= max(3, len(source) // 4) and
                all(kind in {'equal', 'replace'} for kind, *_ in operations)):
            pieces = []
            for kind, left, right, target_left, target_right in operations:
                correction = (None if kind == 'equal' else
                              (0, right - left, visible[target_left:target_right]))
                pieces.append(source_reference(page, block['block_id'], start + left,
                                              start + right, correction))
            return ''.join(pieces)
        if matcher.ratio() < .8:
            return source_reference(page, block['block_id'], start, end,
                                    (0, end - start, visible))
        source_tokens = re.findall(r'(?u)\w+|\W+', source)
        visible_tokens = re.findall(r'(?u)\w+|\W+', visible)
        source_offsets, visible_offsets = [0], [0]
        for token in source_tokens:
            source_offsets.append(source_offsets[-1] + len(token))
        for token in visible_tokens:
            visible_offsets.append(visible_offsets[-1] + len(token))
        token_parts = []
        for kind, a, b, c, d in SequenceMatcher(
                None, source_tokens, visible_tokens, autojunk=False).get_opcodes():
            left, right = source_offsets[a], source_offsets[b]
            shown = visible[visible_offsets[c]:visible_offsets[d]]
            if kind == 'equal':
                token_parts.append(source_reference(page, block['block_id'],
                                                    start + left, start + right))
            elif left < right and right - left <= 40:
                token_parts.append(source_reference(page, block['block_id'],
                                                    start + left, start + right,
                                                    (0, right - left, shown)))
            elif left == right and not any(char.isalnum() for char in shown):
                token_parts.append(escape_literal(shown))
            else:
                return source_reference(page, block['block_id'], start, end,
                                        (0, end - start, visible))
        return ''.join(token_parts)
    if changed > max(2, len(source) // 20):
        if len(source) > 80:
            positions = [index for index, char in enumerate(source) if not char.isspace()]
            visible_counts = [0]
            for char in visible:
                visible_counts.append(visible_counts[-1] + (not char.isspace()))
            boundaries = []
            for match in re.finditer(r'\s+', visible):
                count = visible_counts[match.end()]
                source_cut = positions[count] if count < len(positions) else len(source)
                boundaries.append((source_cut, match.end()))
            boundaries.append((len(source), len(visible)))
            pieces, source_left, visible_left = [], 0, 0
            while source_left < len(source):
                remaining = [(source_cut, visible_cut) for source_cut, visible_cut in boundaries
                             if source_cut > source_left and visible_cut > visible_left]
                within = [cut for cut in remaining if cut[0] - source_left <= 80]
                source_cut, visible_cut = within[-1] if within else remaining[0]
                shown = visible[visible_left:visible_cut]
                original = source[source_left:source_cut]
                pieces.append(source_reference(page, block['block_id'],
                                               start + source_left, start + source_cut,
                                               None if shown == original else
                                               (0, source_cut - source_left, shown)))
                source_left, visible_left = source_cut, visible_cut
            return ''.join(pieces)
        return source_reference(page, block['block_id'], start, end,
                                (0, end - start, visible))
    pieces = []
    for kind, left, right, target_left, target_right in operations:
        if kind == 'equal':
            pieces.append(source_reference(page, block['block_id'], start + left, start + right))
        elif left < right:
            pieces.append(source_reference(page, block['block_id'], start + left, start + right,
                                           (0, right - left, visible[target_left:target_right])))
        elif target_left < target_right:
            pieces.append(escape_literal(visible[target_left:target_right]))
    return ''.join(pieces)


def join_adjacent_references(template, workspace):
    """Combine source ranges only when their rendered text stays the same."""
    pages = {}
    pattern = re.compile(f'(?P<left>{REFERENCE.pattern})(?P<space> ?)(?P<right>{REFERENCE.pattern})')

    def join(match):
        left = REFERENCE.fullmatch(match['left'])
        right = REFERENCE.fullmatch(match['right'])
        if (CORRECTION.search(match['left']) or CORRECTION.search(match['right']) or
                CASE.search(match['left']) or CASE.search(match['right'])):
            return match[0]
        page, block, start, end = map(int, left.groups())
        next_page, next_block, next_start, next_end = map(int, right.groups())
        if (page, block) != (next_page, next_block) or next_start < end:
            return match[0]
        if next_start == end and not match['space']:
            pass
        elif next_start == end + 1 and match['space'] == ' ':
            if page not in pages:
                path = workspace / '02_preprocess' / f'page_{page:04d}.json'
                pages[page] = json.loads(path.read_text(encoding='utf-8'))['blocks'] if path.is_file() else []
            source = next((item['text'] for item in pages[page]
                           if item['block_id'] == block), '')
            if end >= len(source) or source[end] != ' ':
                return match[0]
        else:
            return match[0]
        return f'{{{{text page={page} block={block} chars={start}:{next_end}}}}}'

    while True:
        joined = pattern.sub(join, template)
        if joined == template:
            return template
        template = joined


def escape_literal(text):
    return text.replace("\\", "\\\\").replace("{", "\\{").replace("}", "\\}")


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


def resolve_template(template, pages, *, block_index=None):
    """Only template syntax is parsed; expanded PDF text is never parsed again."""
    blocks = block_index if block_index is not None else {page: {b["block_id"]: b["text"] for b in entries} for page, entries in pages.items()}
    parts, position = [], 0
    while position < len(template):
        char = template[position]
        if char == "\\":
            if position + 1 >= len(template) or template[position + 1] not in "\\{}":
                raise ValueError("Invalid text-template escape")
            parts.append(template[position + 1])
            position += 2
        elif char == "{":
            match = REFERENCE.match(template, position)
            if match:
                page, block = int(match[1]), int(match[2])
                try:
                    text = blocks[page][block]
                except KeyError as error:
                    raise ValueError("Text reference points to a missing page or block") from error
                parts.append(source_text(match, text))
                position = match.end()
            else:
                accepted = ACCEPTED.match(template, position)
                if not accepted:
                    raise ValueError("Invalid text reference; escape literal braces")
                value = json.loads(accepted[1])
                if not isinstance(value, str) or not value:
                    raise ValueError("Accepted literal must contain text")
                parts.append(value)
                position = accepted.end()
        elif char == "}":
            raise ValueError("Escape literal closing braces")
        else:
            parts.append(char)
            position += 1
    return "".join(parts)


def visible_source_changes(source, visible):
    """Return visible character runs changed from OCR, ignoring layout whitespace."""
    source_chars = [(index, char) for index, char in enumerate(source) if not char.isspace()]
    visible_chars = [(index, char) for index, char in enumerate(visible) if not char.isspace()]
    source_compact = ''.join(char for _, char in source_chars)
    visible_compact = ''.join(char for _, char in visible_chars)
    if source_compact == visible_compact:
        return []
    changes = []
    for kind, source_start, source_end, visible_start, visible_end in SequenceMatcher(
            None, source_compact, visible_compact, autojunk=False).get_opcodes():
        if kind == 'equal' or visible_start == visible_end:
            continue
        left = visible_chars[visible_start][0]
        right = visible_chars[visible_end - 1][0] + 1
        old = source_compact[source_start:source_end]
        changes.append((left, visible[left:right], old))
    return changes


def source_differences(template, pages, *, block_index=None):
    """Locate source corrections and unanchored words in resolved Markdown."""
    blocks = block_index if block_index is not None else {
        page: {block['block_id']: block['text'] for block in entries}
        for page, entries in pages.items()}
    resolved = resolve_template(template, pages, block_index=blocks)
    marks, cursor, position = [], 0, 0
    while cursor < len(template):
        reference = REFERENCE.match(template, cursor)
        accepted = ACCEPTED.match(template, cursor) if not reference else None
        if reference:
            source = blocks[int(reference[1])][int(reference[2])]
            value = source_text(reference, source)
            correction = CORRECTION.search(reference[0])
            if correction:
                original = source[int(reference[3]):int(reference[4])]
                for left, changed, old in visible_source_changes(original, value):
                    marks.append({'kind': 'correction', 'offset': position + left,
                                  'text': changed, 'source_text': old})
            position += len(value)
            cursor = reference.end()
        elif accepted:
            value = json.loads(accepted[1])
            marks.append({'kind': 'unanchored', 'offset': position, 'text': value})
            position += len(value)
            cursor = accepted.end()
        elif template[cursor] == '\\' and cursor + 1 < len(template):
            position += 1
            cursor += 2
        else:
            position += 1
            cursor += 1
    for mark in marks:
        mark.update(difference_locator(resolved, mark.pop('offset'), mark['text']))
    return marks


def difference_locator(resolved, offset, text):
    start, end = offset, offset + len(text)
    if len(text) <= 80 and '\n' not in text:
        while start and resolved[start - 1].isalnum():
            start -= 1
        while end < len(resolved) and resolved[end].isalnum():
            end += 1
    needle = resolved[start:end]
    return {'needle': needle, 'within_needle': offset - start,
            'occurrence': len(list(re.finditer(re.escape(needle), resolved[:start])))}


def slice_template(template, start, end, pages=None):
    """Slice resolved Unicode offsets without losing or reassigning PDF ranges."""
    parts, position, offset = [], 0, 0
    while position < len(template):
        match = REFERENCE.match(template, position)
        if match:
            page, block, base, limit = map(int, match.groups())
            correction = CORRECTION.search(match[0])
            case = CASE.search(match[0])
            replacement = json.loads(correction[3]) if correction else None
            source_length = limit - base
            length = source_length - (int(correction[2]) - int(correction[1])) + len(replacement) if correction else source_length
            if length == 0:
                if start <= offset < end:
                    parts.append(match[0])
                position = match.end()
                continue
            left, right = max(start, offset), min(end, offset + length)
            if left < right:
                if correction and len(replacement) != int(correction[2]) - int(correction[1]):
                    if left == offset and right == offset + length:
                        parts.append(match[0])
                    else:
                        visible_start, visible_end = left - offset, right - offset
                        source = (next((entry['text'] for entry in pages.get(page, [])
                                        if entry['block_id'] == block), None)
                                  if pages else None)
                        original = source[base:limit] if source is not None else None
                        expanded = (''.join(LIGATURES.get(char, char) for char in original)
                                    if original is not None else None)
                        if (original is not None and expanded != original and
                                expanded == source_text(match, source)):
                            addresses = [index for index, char in enumerate(original)
                                         for _ in LIGATURES.get(char, char)]
                            source_start = addresses[visible_start]
                            source_end = addresses[visible_end - 1] + 1
                            visible = expanded[visible_start:visible_end]
                            original_piece = original[source_start:source_end]
                            parts.append(source_reference(page, block, base + source_start,
                                                          base + source_end,
                                                          None if original_piece == visible else
                                                          (0, source_end - source_start, visible)))
                            offset += length
                            position = match.end()
                            continue
                        nonspace_source = [(index, char) for index, char in enumerate(original)
                                           if not char.isspace()] if original is not None else []
                        nonspace_target = [(index, char) for index, char in enumerate(replacement)
                                           if not char.isspace()]
                        matcher = SequenceMatcher(
                            None, ''.join(char for _, char in nonspace_source),
                            ''.join(char for _, char in nonspace_target), autojunk=False)
                        operations = matcher.get_opcodes()
                        edits = sum(max(b - a, d - c) for kind, a, b, c, d in operations
                                    if kind != 'equal')
                        if original is not None and edits <= 2:
                            mapping = {}
                            for kind, a, b, c, d in operations:
                                if kind in {'equal', 'replace'}:
                                    for index in range(min(b - a, d - c)):
                                        mapping[nonspace_target[c + index][0]] = nonspace_source[a + index][0]
                            addresses = [source_index for target_index, source_index in mapping.items()
                                         if visible_start <= target_index < visible_end]
                            source_start = min(addresses) if addresses else len(original)
                            source_end = max(addresses) + 1 if addresses else len(original)
                            if source_start < source_end:
                                visible = replacement[visible_start:visible_end]
                                original_piece = original[source_start:source_end]
                                parts.append(source_reference(page, block, base + source_start,
                                                              base + source_end,
                                                              None if original_piece == visible else
                                                              (0, source_end - source_start, visible)))
                            else:
                                parts.append(accepted_literal(replacement[visible_start:visible_end]))
                        else:
                            parts.append(accepted_literal(replacement[visible_start:visible_end]))
                else:
                    sliced_correction = None
                    if correction:
                        a, b = int(correction[1]), int(correction[2])
                        cut_a, cut_b = max(a, left - offset), min(b, right - offset)
                        if cut_a < cut_b:
                            sliced_correction = (cut_a - (left - offset), cut_b - (left - offset),
                                                 replacement[cut_a - a:cut_b - a])
                    parts.append(source_reference(page, block, base + left - offset,
                                                  base + right - offset, sliced_correction,
                                                  case=case[1] if case else None))
            offset += length
            position = match.end()
        else:
            accepted = ACCEPTED.match(template, position)
            if accepted:
                value = json.loads(accepted[1])
                left, right = max(start, offset), min(end, offset + len(value))
                if left < right:
                    parts.append(accepted_literal(value[left - offset:right - offset]))
                offset += len(value)
                position = accepted.end()
                continue
            length = 2 if template[position] == "\\" else 1
            if start <= offset < end:
                parts.append(template[position:position + length])
            position += length
            offset += 1
    return ''.join(parts)


def literal_html_tags(template, pages):
    """Keep source references in visible text rather than HTML markup."""
    resolved = resolve_template(template, pages)
    parts, position = [], 0
    for tag in re.finditer(r'</?[A-Za-z][^>]*>', resolved):
        parts.append(slice_template(template, position, tag.start(), pages))
        parts.append(escape_literal(tag[0]))
        position = tag.end()
    parts.append(slice_template(template, position, len(resolved), pages))
    return ''.join(parts)


def edit_template(template, pages, target, *, allow_reorder=False):
    """Preserve the provenance of unchanged characters, including partial ranges."""
    source = resolve_template(template, pages)
    # Formatting-only insertion/removal must not realign repeated words to a
    # different occurrence just because a larger substring happens to match.
    def subsequence(needle, haystack):
        matches, position = [], 0
        for char in needle:
            position = haystack.find(char, position)
            if position < 0:
                return None
            matches.append(position)
            position += 1
        return matches
    inserted = subsequence(source, target)
    deleted = subsequence(target, source) if inserted is None else None
    if inserted is not None or deleted is not None:
        matches = list(enumerate(inserted)) if inserted is not None else [(a, b) for b, a in enumerate(deleted)]
        runs = []
        for a, b in matches:
            if runs and runs[-1][0] + runs[-1][2] == a and runs[-1][1] + runs[-1][2] == b:
                runs[-1][2] += 1
            else:
                runs.append([a, b, 1])
        parts, position = [], 0
        for a, b, size in runs:
            parts.extend((escape_literal(target[position:b]), slice_template(template, a, a + size, pages)))
            position = b + size
        parts.append(escape_literal(target[position:]))
        return ''.join(parts)
    def reordered_span(text):
        match = SequenceMatcher(None, source, text, autojunk=False).find_longest_match()
        if not match.size or not any(char.isalnum() for char in text[match.b:match.b + match.size]):
            return escape_literal(text)
        return (reordered_span(text[:match.b])
                + slice_template(template, match.a, match.a + match.size, pages)
                + reordered_span(text[match.b + match.size:]))

    parts = []
    for operation, a, b, c, d in SequenceMatcher(None, source, target, autojunk=False).get_opcodes():
        parts.append(slice_template(template, a, b, pages) if operation == 'equal' else
                     reordered_span(target[c:d]) if allow_reorder else escape_literal(target[c:d]))
    return ''.join(parts)


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

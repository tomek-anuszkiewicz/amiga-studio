"""Single-pass templates preserving PDF text, with separate table-markup import."""

from difflib import SequenceMatcher
import json
import re
from markdown_it import MarkdownIt

REFERENCE = re.compile(r'\{\{text page=(\d+) block=(\d+) chars=(\d+):(\d+)'
                       r'(?: replace=\d+:\d+ with="(?:\\.|[^"\\])*")?\}\}')
CORRECTION = re.compile(r' replace=(\d+):(\d+) with=("(?:\\.|[^"\\])*")\}\}$')
ACCEPTED = re.compile(r'\{\{accepted text=("(?:\\.|[^"\\])*")\}\}')
OCR_DIGIT_LETTER_PAIRS = {('0', 'D'), ('0', 'O'), ('O', 'D'), ('O', '0'), ('D', '0')}
LIGATURES = {'ﬀ': 'ff', 'ﬁ': 'fi', 'ﬂ': 'fl', 'ﬃ': 'ffi', 'ﬄ': 'ffl'}


def accepted_literal(value):
    return '{{accepted text=' + json.dumps(value, ensure_ascii=False) + '}}'


def source_text(match, text):
    """Resolve one reference and validate its source-relative substitution."""
    page, block, start, end = map(int, match.groups())
    if not 0 <= start < end <= len(text):
        raise ValueError("Text reference range is outside its source block")
    value = text[start:end]
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


def source_reference(page, block, start, end, correction=None):
    value = f'{{{{text page={page} block={block} chars={start}:{end}'
    if correction is not None:
        left, right, replacement = correction
        value += f' replace={left}:{right} with={json.dumps(replacement, ensure_ascii=False)}'
    return value + '}}'


def source_span_with_spacing(page, block, start, end, visible):
    """Localize sparse OCR differences within one already selected source span."""
    source = block['text'][start:end]
    if source == visible:
        return source_reference(page, block['block_id'], start, end)
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


def attach_adjacent_corrections(template, pages):
    """Keep a one-letter OCR substitution on its adjacent source span."""
    raw_ligature_suffix = re.compile(rf'(?P<reference>{REFERENCE.pattern})(?P<letter>ffi|ffl|fi|fl|ff)(?!\w)')
    raw_ligature_prefix = re.compile(rf'(?<!\w)(?P<letter>ffi|ffl|fi|fl|ff)(?P<reference>{REFERENCE.pattern})')
    template = raw_ligature_suffix.sub(
        lambda match: match['reference'] + accepted_literal(match['letter']), template)
    template = raw_ligature_prefix.sub(
        lambda match: accepted_literal(match['letter']) + match['reference'], template)
    raw_suffix = re.compile(rf'(?P<reference>{REFERENCE.pattern})(?P<letter>[^\W_])(?!\w)')
    raw_prefix = re.compile(rf'(?<!\w)(?P<letter>[^\W_])(?P<reference>{REFERENCE.pattern})')
    template = raw_suffix.sub(lambda match: match['reference'] + accepted_literal(match['letter']), template)
    template = raw_prefix.sub(lambda match: accepted_literal(match['letter']) + match['reference'], template)
    suffix = re.compile(f'(?P<reference>{REFERENCE.pattern})(?P<literal>{ACCEPTED.pattern})')
    prefix = re.compile(f'(?P<literal>{ACCEPTED.pattern})(?P<reference>{REFERENCE.pattern})')
    index = {page: {block['block_id']: block['text'] for block in blocks}
             for page, blocks in pages.items()}

    def attach(match, before):
        reference = REFERENCE.fullmatch(match['reference'])
        literal = json.loads(ACCEPTED.fullmatch(match['literal'])[1])
        if not literal.isalnum() or CORRECTION.search(reference[0]):
            return match[0]
        page, block, start, end = map(int, reference.groups())
        source = index[page][block]
        if len(literal) > 1:
            if before and start and LIGATURES.get(source[start - 1]) == literal:
                return (source_reference(page, block, start - 1, start,
                                         (0, 1, literal)) + reference[0])
            if not before and end < len(source) and LIGATURES.get(source[end]) == literal:
                return (reference[0] + source_reference(page, block, end, end + 1,
                                                        (0, 1, literal)))
            return match[0]
        if before:
            if end - start < 3 or start == 0 or not source[start - 1].isalnum():
                return match[0]
            return source_reference(page, block, start - 1, end,
                                    (0, 1, literal))
        if end - start < 3 or end >= len(source) or not source[end].isalnum():
            return match[0]
        return source_reference(page, block, start, end + 1,
                                (end - start, end - start + 1, literal))

    template = suffix.sub(lambda match: attach(match, False), template)
    template = prefix.sub(lambda match: attach(match, True), template)

    # A short accepted fragment between consecutive ranges of one OCR block has
    # an exact source address even when several characters were misrecognized.
    # Keep structural Markdown and table separators outside these substitutions.
    references = list(REFERENCE.finditer(template))
    bridges = []
    for left, right in zip(references, references[1:]):
        if ((left[1], left[2]) != (right[1], right[2]) or
                not ACCEPTED.search(template[left.end():right.start()])):
            continue
        page, block, source_end, next_start = (int(left[1]), int(left[2]),
                                               int(left[4]), int(right[3]))
        gap = next_start - source_end
        if not 0 < gap <= 6 or page not in index or block not in index[page]:
            continue
        middle = template[left.end():right.start()]
        try:
            visible = resolve_template(middle, {})
        except ValueError:
            continue
        if (not 0 < len(visible) <= 8 or abs(len(visible) - gap) > 1 or
                any(char in '|<>*#`{}[]$\\\n' for char in visible)):
            continue
        original = index[page][block][source_end:next_start]
        if not any(char.isalnum() or char in '~' for char in original):
            continue
        bridges.append((left.end(), right.start(),
                        source_reference(page, block, source_end, next_start,
                                         (0, gap, visible))))
    for start, end, replacement in reversed(bridges):
        template = template[:start] + replacement + template[end:]
    return template


def join_adjacent_references(template, workspace):
    """Combine source ranges only when their rendered text stays the same."""
    pages = {}
    pattern = re.compile(f'(?P<left>{REFERENCE.pattern})(?P<space> ?)(?P<right>{REFERENCE.pattern})')

    def join(match):
        left = REFERENCE.fullmatch(match['left'])
        right = REFERENCE.fullmatch(match['right'])
        if CORRECTION.search(match['left']) or CORRECTION.search(match['right']):
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


def math_semantic_parts(expression):
    """Separate supported math notation from its PDF-visible characters."""
    parts = []
    position = 0
    while position < len(expression):
        char = expression[position]
        if char == '\\':
            command = re.match(r'\\([A-Za-z]+|[, ])', expression[position:])
            if not command or command[1] not in {'mathrm', ',', ' '}:
                return None
            kind, value = 'syntax', command[0]
            position += len(value)
        else:
            kind = 'syntax' if char in '${}_^' else 'text'
            value = char
            position += 1
        if parts and parts[-1][0] == kind:
            parts[-1] = (kind, parts[-1][1] + value)
        else:
            parts.append((kind, value))
    return parts


def bind_unique_region_literals(page, blocks, template):
    """Bind exact, unused text from the selected PDF region after alignment."""
    literals = list(ACCEPTED.finditer(template))
    if not literals:
        return template
    sources = {block['block_id']: block['text'] for block in blocks}
    references = list(REFERENCE.finditer(template))
    claimed = {block_id: [] for block_id in sources}
    for reference in references:
        if int(reference[1]) == page and int(reference[2]) in claimed:
            claimed[int(reference[2])].append((int(reference[3]), int(reference[4])))
    counts = {}
    for token in literals:
        value = json.loads(token[1])
        counts[value] = counts.get(value, 0) + 1
    edits = []
    for token in literals:
        value = json.loads(token[1])
        if (len(value) < 4 or not any(char.isalpha() for char in value) or
                counts[value] > 1):
            continue
        before = next((ref for ref in reversed(references) if ref.end() <= token.start()), None)
        after = next((ref for ref in references if ref.start() >= token.end()), None)
        matches = []
        for block_id, source in sources.items():
            for hit in re.finditer(re.escape(value), source):
                start, end = hit.span()
                if (any(start < right and end > left for left, right in claimed[block_id]) or
                        (value[0].isalnum() and start and source[start - 1].isalnum()) or
                        (value[-1].isalnum() and end < len(source) and source[end].isalnum()) or
                        (before is not None and int(before[1]) == page and
                         int(before[2]) == block_id and start < int(before[4])) or
                        (after is not None and int(after[1]) == page and
                         int(after[2]) == block_id and end > int(after[3]))):
                    continue
                matches.append((block_id, start, end))
        if len(matches) == 1:
            block_id, start, end = matches[0]
            claimed[block_id].append((start, end))
            edits.append((token.start(), token.end(),
                          source_reference(page, block_id, start, end)))
    for start, end, reference in reversed(edits):
        template = template[:start] + reference + template[end:]
    return template


def compile_markdown_template(page, blocks, target):
    """Compile a template and bind unique literal phrases inside its source region."""
    return bind_unique_region_literals(page, blocks,
                                       _compile_markdown_template(page, blocks, target))


def _compile_markdown_template(page, blocks, target):
    """Keep markup literal and map text to exact PDF ranges, allowing reordering.

    Unmatched transcription/corrections remain explicit literals. Matching proves
    character equality, not semantic attribution between identical source strings.
    """
    if (len(target) >= 18 and not any(char in target for char in '<>{}`*_[]|#')):
        candidates = []
        for block in blocks:
            source = block['text']
            if source and min(len(source), len(target)) / max(len(source), len(target)) >= 0.7:
                matcher = SequenceMatcher(None, source, target, autojunk=False)
                if matcher.ratio() >= 0.8:
                    candidates.append((matcher.ratio(), block, matcher))
        candidates.sort(key=lambda item: -item[0])
        if candidates and (len(candidates) == 1 or
                           candidates[0][0] - candidates[1][0] >= 0.05):
            _, block, _ = candidates[0]
            source = block['text']
            source_tokens = re.findall(r'(?u)\w+|\W+', source)
            target_tokens = re.findall(r'(?u)\w+|\W+', target)
            source_offsets, target_offsets = [0], [0]
            for token in source_tokens:
                source_offsets.append(source_offsets[-1] + len(token))
            for token in target_tokens:
                target_offsets.append(target_offsets[-1] + len(token))
            matcher = SequenceMatcher(None, source_tokens, target_tokens, autojunk=False)
            parts = []
            operations = [(kind, source_offsets[a], source_offsets[b],
                           target_offsets[c], target_offsets[d])
                          for kind, a, b, c, d in matcher.get_opcodes()]
            position = 0
            while position < len(operations):
                kind, source_start, source_end, target_start, target_end = operations[position]
                if kind == 'equal':
                    parts.append(source_reference(page, block['block_id'], source_start, source_end))
                    position += 1
                    continue
                # Adjacent replace/insert/delete operations describe one OCR
                # discrepancy; merge them before assigning its source address.
                next_position = position + 1
                while next_position < len(operations) and operations[next_position][0] != 'equal':
                    next_position += 1
                source_end = operations[next_position - 1][2]
                target_end = operations[next_position - 1][4]
                visible = target[target_start:target_end]
                if source_end > source_start:
                    parts.append(source_reference(page, block['block_id'], source_start, source_end,
                                                  (0, source_end - source_start, visible)))
                elif visible:
                    parts.append(accepted_literal(visible) if any(char.isalnum() for char in visible)
                                 else escape_literal(visible))
                position = next_position
            return ''.join(parts)

        # Some OCR pages insert a space between nearly every printed letter and
        # split one paragraph across several consecutive blocks. A nearly exact
        # nonspace stream identifies that block run without guessing each word.
        compact_target = ''.join(char for char in target if not char.isspace())
        target_positions = [index for index, char in enumerate(target) if not char.isspace()]
        runs = []
        for first in range(len(blocks)):
            for last in range(first + 2, min(first + 12, len(blocks)) + 1):
                selected = blocks[first:last]
                compact_parts = [''.join(char for char in block['text'] if not char.isspace())
                                 for block in selected]
                if not all(compact_parts) or sum(map(len, compact_parts)) != len(compact_target):
                    continue
                compact_source = ''.join(compact_parts)
                differences = sum(a != b for a, b in zip(compact_source, compact_target))
                if differences <= max(2, len(compact_target) // 100):
                    runs.append((differences, selected, compact_parts))
        runs.sort(key=lambda item: item[0])
        if runs and (len(runs) == 1 or runs[0][0] < runs[1][0]):
            _, selected, compact_parts = runs[0]
            pieces, consumed = [], 0
            for block, compact in zip(selected, compact_parts):
                next_consumed = consumed + len(compact)
                start = 0 if not consumed else target_positions[consumed]
                end = (target_positions[next_consumed] if next_consumed < len(target_positions)
                       else len(target))
                visible = target[start:end]
                pieces.append(source_span_with_spacing(page, block, 0, len(block['text']),
                                                       visible))
                consumed = next_consumed
            return ''.join(pieces)

    preferred = blocks
    scores = {}
    compact_blocks = {}
    for block in blocks:
        positions = [index for index, char in enumerate(block['text']) if not char.isspace()]
        compact_blocks[id(block)] = (positions, ''.join(block['text'][index] for index in positions))

    def reference_text(text, bounds=None):
        bounds = bounds or {}
        def allowed(block):
            return bounds.get(id(block), (0, len(block['text'])))
        def unclaimed(block, start, end):
            return all(end <= left or start >= right
                       for left, right in claimed_ranges.get(block['block_id'], ()))

        core = text.strip(' \t\r\n')
        raw_hits = []
        if (core and len(core) >= 2 and any(char.isalnum() for char in core) and
                not any(char in '|<>#`{}[]&' for char in core) and
                ('*' not in core or re.fullmatch(r'\*[0-9A-Fa-f]{2,}', core))):
            compact_target = ''.join(char for char in core if not char.isspace())
            compact_hits = []
            for block in preferred:
                source = block['text']
                positions, compact_source = compact_blocks[id(block)]
                for hit in re.finditer(re.escape(compact_target), compact_source):
                    start, end = positions[hit.start()], positions[hit.end() - 1] + 1
                    low, high = allowed(block)
                    if (low <= start and end <= high and unclaimed(block, start, end) and
                            sum(char.isspace() for char in source[start:end]) >= (1 if len(core) == 2 else 2)):
                        compact_hits.append((block, start, end))
            if compact_hits and (len(compact_hits) == 1 or
                                 (scores[id(compact_hits[0][0])] >= 8 and
                                  scores[id(compact_hits[0][0])] > scores[id(compact_hits[1][0])])):
                block, start, end = compact_hits[0]
                offset = text.index(core)
                return (escape_literal(text[:offset]) +
                        source_span_with_spacing(page, block, start, end, core) +
                        escape_literal(text[offset + len(core):]))
            for block in preferred:
                low, high = allowed(block)
                for hit in re.finditer(re.escape(core), block['text'][low:high]):
                    start, end = low + hit.start(), low + hit.end()
                    if unclaimed(block, start, end):
                        raw_hits.append((block, start, end))
            if raw_hits and (len(raw_hits) == 1 or
                             (scores[id(raw_hits[0][0])] >= 8 and
                              scores[id(raw_hits[0][0])] > scores[id(raw_hits[1][0])])):
                block, start, end = raw_hits[0]
                offset = text.index(core)
                return (escape_literal(text[:offset]) +
                        source_reference(page, block['block_id'], start, end) +
                        escape_literal(text[offset + len(core):]))
            # Markdown may insert a hyphen inside one OCR word, for example
            # TABLE 3-2 from Stage 02 "TABLE  32". Bind its parts only when
            # their joined characters identify one complete source word.
            if not raw_hits and re.fullmatch(r'(?u)[^\W_]+(?:-[^\W_]+)+', core):
                joined = core.replace('-', '')
                candidates = []
                for block in preferred:
                    low, high = allowed(block)
                    pattern = r'(?<!\w)' + re.escape(joined) + r'(?!\w)'
                    for hit in re.finditer(pattern, block['text']):
                        if (low <= hit.start() and hit.end() <= high and
                                unclaimed(block, hit.start(), hit.end())):
                            candidates.append((block, hit.start()))
                if len(candidates) == 1:
                    block, position = candidates[0]
                    pieces = []
                    for part in re.split(r'(-)', core):
                        if part == '-':
                            pieces.append('-')
                        elif part:
                            pieces.append(source_reference(page, block['block_id'],
                                                           position, position + len(part)))
                            position += len(part)
                    offset = text.index(core)
                    return (escape_literal(text[:offset]) + ''.join(pieces) +
                            escape_literal(text[offset + len(core):]))
            if not raw_hits and len(compact_target) >= 4 and len(compact_target) <= 80:
                near_hits = []
                seed_size = 3 if len(compact_target) >= 7 else 2
                seeds = ((0, compact_target[:seed_size]),
                         (len(compact_target) - seed_size, compact_target[-seed_size:]))
                for block in preferred:
                    source = block['text']
                    positions, compact_source = compact_blocks[id(block)]
                    starts = {hit.start() - offset
                              for offset, seed in seeds
                              for hit in re.finditer(re.escape(seed), compact_source)}
                    low, high = allowed(block)
                    for compact_start in starts:
                        compact_end = compact_start + len(compact_target)
                        if compact_start < 0 or compact_end > len(positions):
                            continue
                        start, end = positions[compact_start], positions[compact_end - 1] + 1
                        if (start < low or end > high or not unclaimed(block, start, end) or
                                not any(char.isspace() for char in source[start:end])):
                            continue
                        window = compact_source[compact_start:compact_end]
                        differences = [(a, b) for a, b in zip(window, compact_target) if a != b]
                        if len(differences) == 1:
                            old, new = differences[0]
                            # OCR frequently reads zero as capital O. Prefer
                            # that explanation over a changed printed digit
                            # when both source rows otherwise fit the cell.
                            confusion = (old, new) in {('O', '0'), ('0', 'O')}
                            if len(compact_target) == 4 and not (
                                    confusion or not new.isalnum()):
                                continue
                            near_hits.append((block, start, end, confusion))
                near_hits.sort(key=lambda item: (-item[3], -scores[id(item[0])],
                                                     item[0]['block_id'] not in line_blocks))
                if near_hits and (len(near_hits) == 1 or
                                  near_hits[0][3] and not near_hits[1][3] or
                                  (scores[id(near_hits[0][0])] >= 8 and
                                   scores[id(near_hits[0][0])] > scores[id(near_hits[1][0])])):
                    block, start, end, _ = near_hits[0]
                    offset = text.index(core)
                    return (escape_literal(text[:offset]) +
                            source_span_with_spacing(page, block, start, end, core) +
                            escape_literal(text[offset + len(core):]))
        if not raw_hits and len(core) >= 4 and '\n' not in core:
            candidates = []
            for block in preferred:
                source = block['text']
                low, high = allowed(block)
                match = SequenceMatcher(None, source[low:high], core, autojunk=False).find_longest_match()
                start = low + match.a - match.b
                if (match.size < 3 or start < low or start + len(core) > high or
                        not unclaimed(block, start, start + len(core))):
                    continue
                window = source[start:start + len(core)]
                differences = [i for i, (a, b) in enumerate(zip(window, core)) if a != b]
                if (len(differences) == 1 and
                        core[differences[0]] not in '|*<>#`{}[]' and
                        window[differences[0]] not in '|*<>#`{}[]'):
                    candidates.append((block, start, differences[0],
                                       (window[differences[0]], core[differences[0]]) == ('O', 'Q')))
            candidates.sort(key=lambda item: (not item[3], -scores[id(item[0])]))
            if candidates and (len(candidates) == 1 or
                               candidates[0][3] and not candidates[1][3] or
                               scores[id(candidates[0][0])] >= 8 and
                               scores[id(candidates[0][0])] > scores[id(candidates[1][0])]):
                block, start, offset, _ = candidates[0]
                core_start = text.index(core)
                return (escape_literal(text[:core_start]) +
                        source_reference(page, block['block_id'], start, start + len(core),
                                         (offset, offset + 1, core[offset])) +
                        escape_literal(text[core_start + len(core):]))
        if (not raw_hits and len(core) >= 20 and '\n' not in core and
                not any(char in core for char in '<>{}[]|')):
            compact_target = ''.join(char for char in core if not char.isspace())
            fuzzy = []
            for block in preferred:
                positions, compact_source = compact_blocks[id(block)]
                longest = SequenceMatcher(None, compact_source, compact_target,
                                          autojunk=False).find_longest_match()
                if longest.size < len(compact_target) - 4:
                    continue
                low, high = allowed(block)
                for compact_start in range(longest.a - longest.b - 2,
                                           longest.a - longest.b + 3):
                    for length in range(len(compact_target) - 2, len(compact_target) + 3):
                        compact_end = compact_start + length
                        if compact_start < 0 or compact_end > len(positions):
                            continue
                        start, end = positions[compact_start], positions[compact_end - 1] + 1
                        if (start < low or end > high or not unclaimed(block, start, end) or
                                not any(char.isspace() for char in block['text'][start:end])):
                            continue
                        window = compact_source[compact_start:compact_end]
                        if window[0] != compact_target[0] or window[-1] != compact_target[-1]:
                            continue
                        matcher = SequenceMatcher(None, window, compact_target, autojunk=False)
                        edits = sum(max(b - a, d - c) for kind, a, b, c, d in matcher.get_opcodes()
                                    if kind != 'equal')
                        if edits <= 2:
                            fuzzy.append((edits, -matcher.ratio(), block, start, end))
            fuzzy.sort(key=lambda item: (item[0], item[1], -scores[id(item[2])]))
            if fuzzy and (len(fuzzy) == 1 or
                          fuzzy[0][0] < fuzzy[1][0] or
                          fuzzy[0][1] + .05 < fuzzy[1][1]):
                _, _, block, start, end = fuzzy[0]
                core_start = text.index(core)
                return (escape_literal(text[:core_start]) +
                        source_span_with_spacing(page, block, start, end, core) +
                        escape_literal(text[core_start + len(core):]))
        # OCR confusions need a full-word source address. In long prose, both
        # neighboring words must agree before taking a word from a large block.
        if '\n' not in core:
            words = list(re.finditer(r'(?u)[^\W_]{2,}', text))
            for word_index, word in enumerate(words):
                visible = word[0]
                if len(text) > 24 and len(visible) < 5:
                    continue
                candidates = []
                for block in preferred:
                    low, high = allowed(block)
                    if len(text) <= 24 and high - low > 80 and len(visible) < 6:
                        continue
                    source_words = list(re.finditer(r'(?u)[^\W_]+', block['text']))
                    for source_index, hit in enumerate(source_words):
                        source_word = hit[0]
                        context = 0
                        if len(text) > 24:
                            if word_index == 0:
                                for target_next, source_next in zip(words[1:], source_words[source_index + 1:]):
                                    if target_next[0] != source_next[0]:
                                        break
                                    context += 1
                                if context < 2:
                                    continue
                            elif word_index == len(words) - 1:
                                for target_prev, source_prev in zip(reversed(words[:word_index]),
                                                                      reversed(source_words[:source_index])):
                                    if target_prev[0] != source_prev[0]:
                                        break
                                    context += 1
                                if context < 2:
                                    continue
                            elif (source_index == 0 or source_index == len(source_words) - 1 or
                                  words[word_index - 1][0] != source_words[source_index - 1][0] or
                                  words[word_index + 1][0] != source_words[source_index + 1][0]):
                                continue
                            else:
                                context = 2
                        expanded = ''.join(LIGATURES.get(char, char) for char in source_word)
                        if expanded == visible and expanded != source_word:
                            ligatures = [offset for offset, char in enumerate(source_word)
                                         if char in LIGATURES]
                            first, last = ligatures[0], ligatures[-1] + 1
                            replacement = ''.join(LIGATURES.get(char, char)
                                                  for char in source_word[first:last])
                            correction = first, last, replacement
                        else:
                            if len(source_word) != len(visible):
                                continue
                            differences = [offset for offset, pair in enumerate(zip(source_word, visible))
                                           if pair[0] != pair[1]]
                            supported_confusion = all((source_word[offset], visible[offset])
                                                      in OCR_DIGIT_LETTER_PAIRS
                                                      for offset in differences)
                            contextual_typo = (len(text) > 24 and len(differences) <= 2 and
                                               all(source_word[offset].isalpha() and
                                                   visible[offset].isalpha()
                                                   for offset in differences))
                            if not differences or not (supported_confusion or contextual_typo):
                                continue
                            first, last = differences[0], differences[-1] + 1
                            correction = first, last, visible[first:last]
                        start, end = hit.span()
                        if low <= start and end <= high and unclaimed(block, start, end):
                            continuation = (low > 0 and start - low <= 2 and
                                            not any(char.isalnum() for char in block['text'][low:start]))
                            candidates.append((block, start, end, correction, context,
                                               continuation))
                candidates.sort(key=lambda candidate: (-candidate[4], -candidate[5]))
                if candidates and (len(candidates) == 1 or
                                   candidates[0][4:6] > candidates[1][4:6]):
                    block, start, end, correction, _, _ = candidates[0]
                    low, high = allowed(block)
                    left_bounds = {**bounds, id(block): (low, start)}
                    right_bounds = {**bounds, id(block): (end, high)}
                    return (reference_text(text[:word.start()], left_bounds) +
                            source_reference(page, block['block_id'], start, end, correction) +
                            reference_text(text[word.end():], right_bounds))
        best = None
        def word_boundary(value, index):
            return (index == 0 or index == len(value) or
                    not (value[index - 1].isalnum() and value[index].isalnum()))

        for block in preferred:
            low, high = allowed(block)
            source = list(block['text'][low:high])
            for left, right in claimed_ranges.get(block['block_id'], ()):
                masked_left, masked_right = max(low, left), min(high, right)
                if masked_left < masked_right:
                    source[masked_left - low:masked_right - low] = '\0' * (masked_right - masked_left)
            for match in SequenceMatcher(None, ''.join(source), text,
                                         autojunk=False).get_matching_blocks():
                # A visible word must have one contiguous OCR address. Matching
                # just its prefix or suffix would borrow the rest elsewhere.
                starts = (offset for offset in range(match.size)
                          if word_boundary(block['text'], low + match.a + offset) and
                          word_boundary(text, match.b + offset))
                start = next(starts, None)
                if start is None:
                    continue
                ends = (offset for offset in range(match.size, start, -1)
                        if word_boundary(block['text'], low + match.a + offset) and
                        word_boundary(text, match.b + offset))
                end = next(ends, None)
                if end is None:
                    continue
                size = end - start
                target_start = match.b + start
                if ((size >= 2 or size == len(text) == 1) and
                        any(char.isalnum() for char in text[target_start:target_start + size])):
                    if best is None or size > best[2]:
                        best = block, low + match.a + start, size, target_start
        if best is None:
            word = re.fullmatch(r'(?u)(\s*)([^\W_]{4,})(\s*)', text)
            return (escape_literal(word[1]) + accepted_literal(word[2]) +
                    escape_literal(word[3])) if word else escape_literal(text)
        block, source_start, size, target_start = best
        low, high = allowed(block)
        left_bounds = {**bounds, id(block): (low, source_start)}
        right_bounds = {**bounds, id(block): (source_start + size, high)}
        reference = source_reference(page, block['block_id'], source_start, source_start + size)
        return (reference_text(text[:target_start], left_bounds) + reference
                + reference_text(text[target_start + size:], right_bounds))

    parts, fence = [], None
    used_blocks = set()
    used_table_blocks = set()
    for line in target.splitlines(keepends=True):
        line_start = len(parts)
        visible = re.sub(r'<[^>]*>', '', line).replace('|', ' ').replace('*', '')
        compact_visible = ''.join(char for char in visible if not char.isspace())
        scores = {}
        for block in blocks:
            raw_score = SequenceMatcher(None, block['text'], visible,
                                        autojunk=False).find_longest_match().size
            compact_source = compact_blocks[id(block)][1]
            compact_score = SequenceMatcher(None, compact_source, compact_visible,
                                            autojunk=False).find_longest_match().size
            ocr_score = (SequenceMatcher(None, compact_source.replace('O', 'Q'), compact_visible,
                                         autojunk=False).find_longest_match().size
                         if 'Q' in compact_visible else 0)
            scores[id(block)] = max(raw_score, compact_score, ocr_score)
        preferred = sorted(blocks, key=lambda block: (-scores[id(block)],
                                                     block['block_id'] in used_blocks))
        line_blocks = set()
        claimed_ranges = {}
        def append_mapped(result):
            parts.append(result)
            for ref in REFERENCE.finditer(result):
                block = int(ref[2])
                line_blocks.add(block)
                claimed_ranges.setdefault(block, []).append((int(ref[3]), int(ref[4])))
            preferred.sort(key=lambda block: (-scores[id(block)],
                                              block['block_id'] not in line_blocks,
                                              block['block_id'] in used_blocks))

        def append_plain(value):
            append_mapped(reference_text(value))

        def append_styled_text(value):
            # Emphasis and inline-code delimiters are Markdown syntax; their
            # contents remain eligible for a source address.
            if value.count('**') % 2 or value.count('__') % 2:
                append_plain(value)
                return
            for piece in re.split(r'(?<!\\)(\*\*|__|`+)', value):
                if re.fullmatch(r'\*\*|__|`+', piece or ''):
                    parts.append(escape_literal(piece))
                else:
                    append_plain(piece)

        def append_text(value):
            math = re.fullmatch(r'(\s*)(\$[^$\n]+\$)(\s*)', value)
            if math:
                semantic_parts = math_semantic_parts(math[2])
                if semantic_parts is not None:
                    plain = ''.join(content for kind, content in semantic_parts if kind == 'text')
                    mapped = reference_text(plain)
                    if (REFERENCE.search(mapped) and not ACCEPTED.search(mapped) and
                            not re.search(r'[^\W_]', REFERENCE.sub('', mapped))):
                        append_styled_text(math[1])
                        offset, rebuilt = 0, []
                        for kind, content in semantic_parts:
                            if kind == 'syntax':
                                rebuilt.append(escape_literal(content))
                            else:
                                rebuilt.append(slice_template(mapped, offset, offset + len(content),
                                                              {page: blocks}))
                                offset += len(content)
                        append_mapped(''.join(rebuilt))
                        append_styled_text(math[3])
                        return
            append_styled_text(value)

        def append_inline_text(value):
            inline_tag = r'</?(?:sub|sup)\b[^>]*>'
            if not re.search(inline_tag, value, re.IGNORECASE):
                append_text(value)
                return
            pieces = re.split(f'(?i)({inline_tag})', value)
            plain = ''.join(piece for piece in pieces
                            if not re.fullmatch(inline_tag, piece, re.IGNORECASE))
            mapped = reference_text(plain)
            offset, rebuilt = 0, []
            for piece in pieces:
                if re.fullmatch(inline_tag, piece, re.IGNORECASE):
                    rebuilt.append(piece)
                else:
                    rebuilt.append(slice_template(mapped, offset, offset + len(piece),
                                                  {page: blocks}))
                    offset += len(piece)
            append_mapped(''.join(rebuilt))

        marker = re.match(r'^ {0,3}(`{3,}|~{3,})(.*)$', line.rstrip('\r\n'))
        if marker and fence is None:
            fence = marker[1]
            parts.append(escape_literal(line))
        elif marker and marker[1][0] == fence[0] and len(marker[1]) >= len(fence) and not marker[2].strip():
            fence = None
            parts.append(escape_literal(line))
        elif fence is not None:
            append_text(line)
        else:
            # HTML tags/attributes describe cells and spans, not PDF text.
            chunks = (re.split(r'(?<!\\)(\|)', line) if line.lstrip().startswith('|')
                      and line.count('|') >= 2 else [line])
            table_cells = {}
            if len(chunks) >= 5:
                cells = [(index, re.fullmatch(r'(\s*)(\w+)(\s*)', chunk))
                         for index, chunk in enumerate(chunks) if index % 2 == 0]
                candidates = {}
                for block in blocks:
                    if block['block_id'] in used_table_blocks:
                        continue
                    tokens = list(re.finditer(r'\S+', block['text']))
                    if len(tokens) < 2 or len(tokens) > len(cells):
                        continue
                    for first in range(len(cells) - len(tokens) + 1):
                        group = cells[first:first + len(tokens)]
                        if any(match is None or match[2] != token[0]
                               for (_, match), token in zip(group, tokens)):
                            continue
                        for (index, _), token in zip(group, tokens):
                            candidates.setdefault(index, []).append((block, token.start(), token.end()))
                for index, hits in candidates.items():
                    if len(hits) == 1:
                        table_cells[index] = hits[0]
            for index, chunk in enumerate(chunks):
                if chunk == '|':
                    parts.append('|')
                    continue
                if index in table_cells:
                    block, start, end = table_cells[index]
                    cell = re.fullmatch(r'(\s*)(\w+)(\s*)', chunk)
                    parts.append(escape_literal(cell[1]))
                    append_mapped(source_reference(page, block['block_id'], start, end))
                    parts.append(escape_literal(cell[3]))
                    used_table_blocks.add(block['block_id'])
                    continue
                position = 0
                for tag in re.finditer(r'</?(?!sub\b|sup\b)[A-Za-z][^>]*>', chunk,
                                       re.IGNORECASE):
                    append_inline_text(chunk[position:tag.start()])
                    parts.append(escape_literal(tag[0]))
                    position = tag.end()
                append_inline_text(chunk[position:])
        line_template = ''.join(parts[line_start:])
        used_blocks.update(int(ref[2]) for ref in REFERENCE.finditer(line_template)
                           if int(ref[4]) - int(ref[3]) >= 3)
    return attach_adjacent_corrections(''.join(parts), {page: blocks})


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
                                                  base + right - offset, sliced_correction))
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
        return attach_adjacent_corrections(''.join(parts), pages)
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
    return attach_adjacent_corrections(''.join(parts), pages)


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

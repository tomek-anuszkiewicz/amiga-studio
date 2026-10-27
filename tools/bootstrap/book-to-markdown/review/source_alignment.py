"""Page-local alignment of visible prepared Markdown with positioned PDF text."""

from difflib import SequenceMatcher
import re

from .text_recipe import (ACCEPTED, REFERENCE, accepted_literal, escape_literal,
                          region_blocks, resolve_template, source_span_with_spacing)


WORD = re.compile(r"(?u)[^\W_]+")
TEX_COMMAND = re.compile(r"\\[A-Za-z]+|\\.")
HTML_TAG = re.compile(r"<!--.*?-->|<[^>]*>", re.DOTALL)
ENTITY = re.compile(r"&(?:#[0-9]+|#x[0-9A-Fa-f]+|[A-Za-z]+);")
FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
TABLE_RULE = re.compile(r"^\s*\|?\s*:?-{3,}:?(?:\s*\|\s*:?-{3,}:?)*\s*\|?\s*$")
OCR_GLYPH_PAIRS = {frozenset(pair) for pair in ("o0", "l1", "i1", "s5", "b8", "z2", "g6")}


def visible_positions(markdown):
    """Return searchable text and its character offsets in unchanged Markdown."""
    keep = [True] * len(markdown)
    lines = markdown.splitlines(keepends=True)
    cursor = 0
    fence = None
    for line in lines:
        body = line.rstrip("\r\n")
        marker = FENCE.match(body)
        if marker and (fence is None or
                       marker[1][0] == fence[0] and len(marker[1]) >= len(fence)):
            for index in range(cursor, cursor + len(body)):
                keep[index] = False
            fence = marker[1] if fence is None else None
        elif fence is None and TABLE_RULE.fullmatch(body):
            for index in range(cursor, cursor + len(body)):
                keep[index] = False
        elif fence is None:
            prefix = re.match(r"^\s*(?:#{1,6}\s+|>\s+|[-+*]\s+|\d+[.)]\s+)", body)
            if prefix:
                for index in range(cursor, cursor + prefix.end()):
                    keep[index] = False
        cursor += len(line)
    for pattern in (HTML_TAG, ENTITY, TEX_COMMAND):
        for match in pattern.finditer(markdown):
            for index in range(match.start(), match.end()):
                keep[index] = False
    # Braces delimit math groups. Outside math they remain literal structure;
    # neither case should become a searchable PDF word.
    for index, char in enumerate(markdown):
        if char in "${}|":
            keep[index] = False
    # A Markdown destination is metadata, while its bracketed label is visible.
    for match in re.finditer(r"\]\([^\n)]*\)", markdown):
        for index in range(match.start(), match.end()):
            keep[index] = False
    positions = [index for index, allowed in enumerate(keep) if allowed]
    return "".join(markdown[index] for index in positions), positions


def compact_with_addresses(blocks):
    chars, addresses = [], []
    for block in blocks:
        for offset, char in enumerate(block["text"]):
            if char.isalnum():
                for folded in char.casefold():
                    chars.append(folded)
                    addresses.append((block, offset))
    return "".join(chars), addresses


def _rank_candidate(query, source, near):
    if not source:
        return (-1, -1, -1, -1)
    exact = int(len(query) >= 3 and query in source)
    target_grams = {query[index:index + 3] for index in range(max(0, len(query) - 2))}
    source_grams = {source[index:index + 3] for index in range(max(0, len(source) - 2))}
    recall = len(target_grams & source_grams) / max(1, len(target_grams))
    ratio = SequenceMatcher(None, query, source, autojunk=False).ratio()
    return (exact, int(near), recall, ratio - len(source) / 100000)


def _candidate_blocks(page_data, bbox):
    blocks = page_data["blocks"]
    nearby = (region_blocks(page_data, bbox) if "raster" in page_data and
              all("bbox" in block for block in blocks) else [])
    candidates = [(block,) for block in blocks]
    # The logical segment may draw text from several PDF blocks.
    for first in range(len(nearby)):
        for last in range(first + 2, min(first + 6, len(nearby) + 1)):
            candidates.append(tuple(nearby[first:last]))
    return candidates, {id(block) for block in nearby}


def _target_mapping(target, source):
    exact = source.find(target) if len(target) >= 3 else -1
    if exact >= 0:
        return {index: exact + index for index in range(len(target))}
    mapping = {}
    for kind, a, b, c, d in SequenceMatcher(None, target, source,
                                             autojunk=False).get_opcodes():
        if kind == "equal" or kind == "replace" and b - a == d - c and b - a <= 2:
            for offset in range(b - a):
                mapping[a + offset] = c + offset
    return mapping


def _word_ranges(visible, offsets):
    """Keep a markup boundary from swallowing syntax between two words."""
    for word in WORD.finditer(visible):
        start = word.start()
        for index in range(word.start() + 1, word.end()):
            if offsets[index] != offsets[index - 1] + 1:
                yield start, index
                start = index
        yield start, word.end()


def _recover_partial_word(word, mapped_indexes, addresses):
    """Recover one OCR glyph omitted by compact normalization from its neighbors."""
    if len(mapped_indexes) != len(word) or len(word) < 3:
        return None
    known = [(position, addresses[index]) for position, index in enumerate(mapped_indexes)
             if index is not None and index < len(addresses)]
    if len(known) != len(word) - 1:
        return None
    block = known[0][1][0]
    if any(location[0] is not block for _, location in known):
        return None
    starts = {location[1] - position for position, location in known}
    if len(starts) != 1:
        return None
    start = starts.pop()
    end = start + len(word)
    if start < 0 or end > len(block['text']):
        return None
    source = block['text'][start:end]
    if any(char.isspace() for char in source):
        return None
    differences = sum(left.casefold() != right.casefold()
                      for left, right in zip(source, word))
    return (block, start, end) if differences == 1 else None


def _find_fuzzy_word(word, blocks, near):
    """Find a close whole word on the pinned page after exact lookup fails."""
    if len(word) < 2:
        return None
    matches = []
    target = word.casefold()
    limit = 3 if len(target) >= 7 else 1
    for block in blocks:
        for match in WORD.finditer(block["text"]):
            candidate = match[0].casefold()
            if (abs(len(candidate) - len(target)) > limit or
                    len(candidate) != len(match[0])):
                continue
            comparison = SequenceMatcher(None, target, candidate, autojunk=False)
            changes = [(kind, a, b, c, d) for kind, a, b, c, d in comparison.get_opcodes()
                       if kind != "equal"]
            cost = sum(max(b - a, d - c) for _, a, b, c, d in changes)
            if len(target) == len(candidate):
                # Repeated letters can make SequenceMatcher count one substitution
                # as an insertion plus a deletion (for example, Scc vs Sec).
                cost = min(cost, sum(left != right for left, right in
                                     zip(target, candidate)))
            glyph_pairs = sum(
                frozenset((target[a + index], candidate[c + index])) in OCR_GLYPH_PAIRS
                for kind, a, b, c, d in changes
                if kind == "replace" and b - a == d - c
                for index in range(b - a))
            if not (0 < cost <= limit or
                    len(target) >= 3 and cost == glyph_pairs == 2):
                continue
            rank = (cost, -glyph_pairs, -comparison.ratio(),
                    id(block) not in near, block["block_id"], match.start())
            matches.append((rank, block, match.start(), match.end()))
    if not matches:
        return None
    _, block, start, end = min(matches)
    return block, start, end


def _localized_word_span(page, block, start, end, visible):
    """Keep matching letters addressed when OCR omits or inserts a glyph."""
    source = block["text"][start:end]
    if len(source) == len(visible):
        changed = [index for index, (left, right) in enumerate(zip(source, visible))
                   if left != right]
        if len(changed) == 1:
            index = changed[0]
            return "".join(source_span_with_spacing(page, block, start + first,
                                                    start + last, visible[first:last])
                           for first, last in ((0, index), (index, index + 1),
                                               (index + 1, len(visible)))
                           if first < last)
    parts = []
    for kind, a, b, c, d in SequenceMatcher(None, source, visible,
                                             autojunk=False).get_opcodes():
        if kind == "insert":
            parts.append(accepted_literal(visible[c:d]))
        elif kind == "replace" and b - a == d - c and b - a > 1:
            for index in range(b - a):
                parts.append(source_span_with_spacing(page, block, start + a + index,
                                                      start + a + index + 1,
                                                      visible[c + index:c + index + 1]))
        elif kind != "delete":
            parts.append(source_span_with_spacing(page, block, start + a,
                                                  start + b, visible[c:d]))
    return "".join(parts)


def _exact_page_word(word, blocks, near):
    """Prefer a complete, case-identical page word over a compact substring."""
    matches = []
    for block in blocks:
        for match in WORD.finditer(block["text"]):
            if match[0] == word:
                matches.append((id(block) not in near, block["block_id"],
                                match.start(), block, match.end()))
    if not matches:
        return None
    _, _, start, block, end = min(matches)
    return block, start, end


def _split_page_word(word, blocks, near):
    """Find a word broken across the ends of adjacent PDF text blocks."""
    matches = []
    for first, second in zip(blocks, blocks[1:]):
        first_words = list(WORD.finditer(first["text"]))
        second_word = WORD.search(second["text"])
        if not first_words or second_word is None:
            continue
        first_word = first_words[-1]
        first_text, second_text = first_word[0], second_word[0]
        if (len(first_text) + len(second_text) != len(word) or
                (first_text + second_text).casefold() != word.casefold()):
            continue
        exact = first_text + second_text == word
        nearby = id(first) in near and id(second) in near
        matches.append((not exact, not nearby, first["block_id"],
                        first, first_word.start(), first_word.end(),
                        second, second_word.start(), second_word.end(), len(first_text)))
    if not matches:
        return None
    _, _, _, first, a, b, second, c, d, split = min(matches)
    return first, a, b, second, c, d, split


def _spaced_page_word(word, blocks, near):
    """Find a register name whose OCR letters are spaced across one line."""
    if len(word) < 4:
        return None
    limit = 2 if len(word) >= 7 else 1
    matches = []
    for block in blocks:
        cursor = 0
        for line in block["text"].splitlines(keepends=True):
            body = line.rstrip("\r\n")
            if len(re.findall(r"[A-Za-z0-9][ \t]+[A-Za-z0-9]", body)) < 2:
                cursor += len(line)
                continue
            letters = [(index, char) for index, char in enumerate(body) if char.isalnum()]
            compact = "".join(char.casefold() for _, char in letters)
            if abs(len(compact) - len(word)) <= limit and letters:
                comparison = SequenceMatcher(None, word.casefold(), compact,
                                             autojunk=False)
                cost = sum(max(b - a, d - c)
                           for kind, a, b, c, d in comparison.get_opcodes()
                           if kind != "equal")
                if cost <= limit:
                    start = cursor + letters[0][0]
                    end = cursor + letters[-1][0] + 1
                    rank = (cost, -comparison.ratio(), id(block) not in near,
                            block["block_id"], start)
                    matches.append((rank, block, start, end))
            cursor += len(line)
    if not matches:
        return None
    _, block, start, end = min(matches)
    return block, start, end


def _embedded_uppercase_word(word, blocks, near):
    """Find a nearly identical code suffix inside a longer OCR identifier."""
    if len(word) < 4 or not word.isupper():
        return None
    matches = []
    target = word.casefold()
    for block in blocks:
        for token in re.finditer(r"[A-Z0-9\[\]]+", block["text"]):
            source = token[0]
            if len(source) < len(word) or (len(source) == len(word) and "[" not in source):
                continue
            for offset in range(len(source) - len(word) + 1):
                fragment = source[offset:offset + len(word)]
                differences = sum(a != b for a, b in zip(target, fragment.casefold()))
                limit = 2 if "[" in fragment else 1
                if differences <= limit:
                    rank = (differences, id(block) not in near,
                            block["block_id"], token.start() + offset)
                    matches.append((rank, block, token.start() + offset,
                                    token.start() + offset + len(word)))
    if not matches:
        return None
    _, block, start, end = min(matches)
    return block, start, end


def _align_fragment(page, page_data, bbox, markdown):
    visible, offsets = visible_positions(markdown)
    compact_target = "".join(char.casefold() for char in visible if char.isalnum())
    if not compact_target:
        return escape_literal(markdown)
    candidates, near = _candidate_blocks(page_data, bbox)
    best = None
    for selected in candidates:
        source, addresses = compact_with_addresses(selected)
        rank = _rank_candidate(compact_target, source,
                               all(id(block) in near for block in selected))
        if best is None or rank > best[0]:
            best = (rank, source, addresses)
    _, source, addresses = best
    mapping = _target_mapping(compact_target, source)
    compact_indexes = {}
    position = 0
    for index, char in enumerate(visible):
        if char.isalnum():
            compact_indexes[index] = list(range(position, position + len(char.casefold())))
            position += len(char.casefold())
    substitutions = {}
    words = list(_word_ranges(visible, offsets))
    for left, right in words:
        word = visible[left:right]
        indices = [position for index in range(left, right)
                   for position in compact_indexes.get(index, ())]
        found = [mapping.get(index) for index in indices]
        replacement = None
        if len(words) == 1 and len(word) >= 2:
            exact_word = _exact_page_word(word, page_data["blocks"], near)
            if exact_word is not None:
                block, start, end = exact_word
                replacement = source_span_with_spacing(page, block, start, end, word)
        if (replacement is None and found and
                all(index is not None and index < len(addresses) for index in found)):
            locations = [addresses[index] for index in found]
            block = locations[0][0]
            start, end = locations[0][1], locations[-1][1] + 1
            source_word = block["text"][start:end]
            compact_word = "".join(char.casefold() for char in source_word if char.isalnum())
            if (all(location[0] is block for location in locations) and
                    end > start and len(compact_word) <= len(word) + 2 and
                    SequenceMatcher(None, word.casefold(), compact_word,
                                    autojunk=False).ratio() >= .6):
                if source_word != word:
                    exact_word = _exact_page_word(word, page_data["blocks"], near)
                    embedded = (_embedded_uppercase_word(word, page_data["blocks"], near)
                                if exact_word is None else None)
                    if exact_word is not None:
                        block, start, end = exact_word
                    elif embedded is not None:
                        block, start, end = embedded
                        replacement = _localized_word_span(page, block, start, end, word)
                if replacement is None:
                    replacement = source_span_with_spacing(page, block, start, end, word)
        if replacement is None:
            recovered = _recover_partial_word(word, found, addresses)
            if recovered is not None:
                block, start, end = recovered
                replacement = source_span_with_spacing(page, block, start, end, word)
        if replacement is None:
            embedded = _embedded_uppercase_word(word, page_data["blocks"], near)
            if embedded is not None:
                block, start, end = embedded
                replacement = _localized_word_span(page, block, start, end, word)
        if replacement is None:
            # An exact word elsewhere on the selected page is preferable to an
            # invented address when formula or table order differs from OCR.
            matches = []
            for block in page_data["blocks"]:
                compact, addresses_in_block = compact_with_addresses((block,))
                start = compact.find(word.casefold())
                if start >= 0:
                    first = addresses_in_block[start][1]
                    last = addresses_in_block[start + len(word.casefold()) - 1][1] + 1
                    if (first and block["text"][first - 1].isalnum() or
                            last < len(block["text"]) and block["text"][last].isalnum()):
                        continue
                    matches.append((block["text"][first:last] != word,
                                    id(block) not in near, block["block_id"],
                                    block, first, last))
            if matches:
                _, _, _, block, first, last = min(matches)
                replacement = source_span_with_spacing(page, block, first, last, word)
            else:
                split = _split_page_word(word, page_data["blocks"], near)
                if split is not None:
                    first, a, b, second, c, d, pivot = split
                    replacement = (source_span_with_spacing(page, first, a, b, word[:pivot]) +
                                   source_span_with_spacing(page, second, c, d, word[pivot:]))
                else:
                    spaced = _spaced_page_word(word, page_data["blocks"], near)
                    if spaced is not None:
                        block, first, last = spaced
                        replacement = source_span_with_spacing(page, block, first, last, word)
                    else:
                        fuzzy = _find_fuzzy_word(word, page_data["blocks"], near)
                        if fuzzy is not None:
                            block, first, last = fuzzy
                            replacement = _localized_word_span(page, block, first, last, word)
                        else:
                            replacement = accepted_literal(word)
        substitutions[offsets[left]] = (offsets[right - 1] + 1, replacement)
    parts, cursor = [], 0
    for start, (end, replacement) in sorted(substitutions.items()):
        parts.append(escape_literal(markdown[cursor:start]))
        parts.append(replacement)
        cursor = end
    parts.append(escape_literal(markdown[cursor:]))
    return "".join(parts)


def compile_page_template(page, page_data, bbox, markdown):
    """Produce an exact visible recipe; unresolved words become accepted text."""
    # A table row or HTML cell can be independent of adjacent PDF text blocks.
    # Splitting the search query retains all original delimiters in the recipe.
    fragments = re.split(r"(<[^>]*>|\||\n)", markdown)
    template = "".join(_align_fragment(page, page_data, bbox, fragment)
                       for fragment in fragments if fragment)
    resolved = resolve_template(template, {page: page_data["blocks"]})
    if resolved != markdown:
        mismatch = next((index for index, (left, right) in enumerate(zip(resolved, markdown))
                         if left != right), min(len(resolved), len(markdown)))
        raise ValueError(f"Source alignment changed prepared Markdown at {mismatch}: "
                         f"{resolved[mismatch:mismatch + 24]!r} != "
                         f"{markdown[mismatch:mismatch + 24]!r}")
    return template


def align_template_literals(page, page_data, bbox, template, source_pages=None):
    """Bind newly written visible literals without relocating authored references."""
    parts, cursor = [], 0
    for match in re.finditer(f"{REFERENCE.pattern}|{ACCEPTED.pattern}", template):
        if match.start() > cursor:
            literal = resolve_template(template[cursor:match.start()], {})
            parts.append(compile_page_template(page, page_data, bbox, literal))
        parts.append(match[0])
        cursor = match.end()
    if cursor < len(template):
        literal = resolve_template(template[cursor:], {})
        parts.append(compile_page_template(page, page_data, bbox, literal))
    result = "".join(parts)
    source_pages = source_pages or {page: page_data["blocks"]}
    if resolve_template(result, source_pages) != resolve_template(template, source_pages):
        raise ValueError("Source alignment changed an authored recipe")
    return result

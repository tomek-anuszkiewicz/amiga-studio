"""Preview exact PDF text bindings for literal words in the Working recipe."""

import hashlib
import json
import re

from .model import HEADING_TYPES, read_json, toc_label
from .source_alignment import align_template_literals
from .source_validation import validate_book
from .text_recipe import (ACCEPTED, REFERENCE, accepted_literal,
                          difference_locator, resolve_template)


WORDS = re.compile(r'(?u)[^\W_]+(?:[ \t]+[^\W_]+)*[.,:;!?]*')
TAG = re.compile(r'<[^>]*>')


def _literal_spans(template):
    """Ignore references and markup while retaining offsets in the recipe."""
    masked = list(template)
    for match in REFERENCE.finditer(template):
        masked[match.start():match.end()] = '\0' * len(match[0])
    accepted = list(ACCEPTED.finditer(template))
    for match in accepted:
        masked[match.start():match.end()] = '\0' * len(match[0])
    for match in TAG.finditer(template):
        masked[match.start():match.end()] = '\0' * len(match[0])
    yield from ((match.start(), match.end(), match[0], match[0])
                for match in WORDS.finditer(''.join(masked)))


def preview(book, workspace, selected_paths=None):
    """Return matches and unresolved literals without changing book data."""
    directory = workspace / '02_preprocess'
    pages = {}
    audited = {}
    exceptions = {item['anchor'] for item in book.get('source_cleanup_exceptions', [])}
    for issue in validate_book(book, workspace)['issues']:
        if issue['code'] == 'literal_content':
            key = (issue['chapter_id'], issue['element_id'], issue['field'])
            audited.setdefault(key, set()).update(re.findall(r'[^\W_]+', issue['text']))

    def blocks(page):
        if page not in pages:
            path = directory / f'page_{page:04d}.json'
            pages[page] = read_json(path)['blocks'] if path.is_file() else []
        return pages[page]

    results = []
    chapter_rows = [(f'/chapters/{index}', chapter,
                     {region['page'] for element in chapter['elements'][:1]
                      for region in element.get('pdf_regions', [])})
                    for index, chapter in enumerate(book['chapters'])]
    if 'title' in book.get('toc', {}):
        source_pages = {region['page'] for chapter in book['chapters']
                        for element in chapter['elements']
                        for region in element.get('pdf_regions', [])}
        chapter_rows.insert(0, ('/toc', {'id': 'toc', 'title': book['toc']['title'],
                                        'elements': []}, source_pages))
    for prefix, chapter, title_pages in chapter_rows:
        fields = []
        fields.append((f'{prefix}/title', chapter['id'], None,
                       'title', chapter.get('title'), title_pages))
        for element_index, element in enumerate(chapter['elements']):
            fields.append((f'{prefix}/elements/{element_index}/md_text',
                           chapter['id'], element['id'], 'md_text', element.get('md_text'),
                           {region['page'] for region in element.get('pdf_regions', [])}))
        for path, chapter_id, element_id, field, template, anchors in fields:
            if selected_paths is not None and path not in selected_paths:
                continue
            if not isinstance(template, str):
                continue
            anchors = set(anchors)
            anchors.update(int(match[1]) for match in REFERENCE.finditer(template))
            text_pages = {page: blocks(page) for page in anchors}
            try:
                display = resolve_template(template, text_pages)
            except ValueError:
                display = None
            candidate_pages = sorted({page + delta for page in anchors for delta in (-1, 0, 1)
                                      if page + delta > 0})
            audited_words = audited.get((chapter_id, element_id, field), set())
            references = list(REFERENCE.finditer(template))
            cursors = {}
            occurrences = {}
            for start, end, literal, token in _literal_spans(template):
                words = literal.rstrip('.,:;!?')
                # Subscript and superscript digits express notation around a PDF
                # reference; they are not standalone words to bind to OCR text.
                if (not any(char.isalpha() or char.isdecimal() for char in words) or
                        not set(re.findall(r'[^\W_]+', words)).issubset(audited_words)):
                    continue
                owner = element_id or chapter_id
                preceding_ref = max((ref for ref in references if ref.end() <= start),
                                    key=lambda ref: ref.end(), default=None)
                following_ref = min((ref for ref in references if ref.start() >= end),
                                    key=lambda ref: ref.start(), default=None)
                context = (owner, field, literal,
                           preceding_ref[0] if preceding_ref else '',
                           template[max(start - 32, preceding_ref.end() if preceding_ref else 0):start],
                           template[end:min(end + 32, following_ref.start() if following_ref else len(template))],
                           following_ref[0] if following_ref else '')
                occurrence = occurrences.get(context, 0)
                occurrences[context] = occurrence + 1
                anchor = hashlib.sha256(json.dumps(
                    [*context, occurrence], ensure_ascii=False).encode()).hexdigest()
                if anchor in exceptions:
                    continue
                entry = {'path': path, 'element_id': element_id or chapter_id, 'literal': literal,
                         'token': token,
                         'offsets': [start, end], 'anchor': anchor}
                if display is not None:
                    try:
                        offset = len(resolve_template(template[:start], text_pages))
                        entry['context'] = (display[max(0, offset - 45):offset] +
                                            '[' + literal + ']' +
                                            display[offset + len(literal):offset + len(literal) + 45])
                        entry.update(difference_locator(display, offset, literal))
                    except ValueError:
                        pass
                entry['id'] = anchor
                found = []
                pattern = r'(?<!\w)' + re.escape(words) + r'(?!\w)'
                if len(words) == 1:
                    previous, following = preceding_ref, following_ref
                    if previous is not None:
                        page, block_id = int(previous[1]), int(previous[2])
                        block = next((item for item in blocks(page)
                                      if item['block_id'] == block_id), None)
                        if block is not None:
                            cursor = cursors.get(previous.start(), int(previous[4]))
                            limit = len(block['text'])
                            if following is not None and (int(following[1]), int(following[2])) == (page, block_id):
                                limit = int(following[3])
                            source = block['text']
                            match = (re.compile(re.escape(words)).match(source, cursor)
                                     if cursor < limit else None)
                            if match is None:
                                match = next((item for item in re.finditer(pattern, source)
                                              if cursor <= item.start() and item.end() <= limit), None)
                            if match is not None:
                                found.append((0, page, block, match, match.end()))
                                cursors[previous.start()] = match.end()
                else:
                    for page in candidate_pages:
                        for block in blocks(page):
                            for match in re.finditer(pattern, block['text']):
                                distance = min(abs(page - anchor) for anchor in anchors)
                                source_end = match.end()
                                if len(words) < len(literal):
                                    while source_end < len(block['text']) and block['text'][source_end] in '.,:;!?':
                                        source_end += 1
                                found.append((distance, page, block, match, source_end))
                if not found and len(words) == 1:
                    entry.update(status='ambiguous',
                                 reason=('One character has no preceding PDF reference' if previous is None
                                         else 'One character has no following occurrence before the next PDF reference'))
                elif not found:
                    entry.update(status='unmatched', reason='No exact text on the element or adjacent PDF pages')
                else:
                    anchored = []
                    if len(words) == 1:
                        matches = found
                    else:
                        nearby = ([max((ref for ref in references if ref.end() <= start),
                                       key=lambda ref: ref.end(), default=None),
                                   min((ref for ref in references if ref.start() >= end),
                                       key=lambda ref: ref.start(), default=None)])
                        for hit in found:
                            _, page, block, match, source_end = hit
                            source = block['text']
                            for ref in nearby:
                                if ref is None or (int(ref[1]), int(ref[2])) != (page, block['block_id']):
                                    continue
                                base, limit = int(ref[3]), int(ref[4])
                                if ((source_end <= base and not source[source_end:base].strip()) or
                                        (limit <= match.start() and not source[limit:match.start()].strip())):
                                    anchored.append(hit)
                                    break
                        previous = nearby[0]
                        if previous is not None:
                            page, block_id, limit = map(int, (previous[1], previous[2], previous[4]))
                            cursor = cursors.get(previous.start(), limit)
                            anchored = [hit for hit in anchored
                                        if (hit[1], hit[2]['block_id']) != (page, block_id) or
                                        hit[3].start() >= cursor]
                        nearest = min(hit[0] for hit in found)
                        matches = anchored
                        if not matches and previous is not None:
                            same_block = [hit for hit in found
                                          if (hit[1], hit[2]['block_id']) == (page, block_id)]
                            following = nearby[1]
                            upper = (int(following[3]) if following is not None and
                                     (int(following[1]), int(following[2])) == (page, block_id)
                                     else None)
                            forward = [hit for hit in same_block if hit[3].start() >= cursor and
                                       (upper is None or hit[4] <= upper)]
                            earlier = [hit for hit in same_block if hit[4] <= limit]
                            matches = forward[:1] or earlier
                            if not matches and same_block:
                                entry.update(status='ambiguous',
                                             reason='No unused match in the referenced PDF block')
                                results.append(entry)
                                continue
                        if not matches:
                            matches = [hit for hit in found if hit[0] == nearest]
                    if len(matches) != 1:
                        description = 'adjacent' if anchored else 'equally near'
                        reason = f'{len(matches)} {description} PDF matches'
                        entry.update(status='ambiguous', reason=reason)
                    else:
                        _, page, block, match, source_end = matches[0]
                        if (len(words) > 1 and previous is not None and
                                (int(previous[1]), int(previous[2])) == (page, block['block_id']) and
                                match.start() >= cursor):
                            cursors[previous.start()] = source_end
                        source = block['text']
                        reference = ('{{text ' + f'page={page} block={block["block_id"]} '
                                     + f'chars={match.start()}:{source_end}' + '}}')
                        entry.update(status='matched', page=page, block=block['block_id'],
                                     chars=[match.start(), source_end],
                                     source_text=source[match.start():source_end],
                                     reference=reference,
                                     local_match=(len(words) == 1 or matches[0] in anchored))
                        identity = [path, start, end, literal, reference, entry['source_text']]
                        entry['id'] = hashlib.sha256(json.dumps(identity, ensure_ascii=False).encode()).hexdigest()
                results.append(entry)
    return results


def reconcile_working(book, original, workspace):
    """Align newly written Working text without moving authored references."""
    old_elements = {element['id']: element for chapter in original['chapters']
                    for element in chapter['elements']}
    old_chapters = {chapter['id']: chapter for chapter in original['chapters']}
    prepared = {}

    def align(value, pages, bbox):
        for page in sorted(pages):
            source = workspace / '02_preprocess' / f'page_{page:04d}.json'
            if source.is_file() and page not in prepared:
                prepared[page] = read_json(source)
        if any(page in prepared for page in pages):
            page = next(page for page in sorted(pages) if page in prepared)
            source_pages = {source_page: prepared[source_page]['blocks']
                            for source_page in pages if source_page in prepared}
            return align_template_literals(page, prepared[page], bbox, value, source_pages)
        return value

    for chapter in book['chapters']:
        previous = old_chapters.get(chapter['id'])
        if previous is None or chapter['title'] != previous['title']:
            title_words = ' '.join(chapter['title'].split()).casefold()
            matching_heading = next((element for element in chapter['elements']
                                     if element.get('visible', True)
                                     and element['type'] in HEADING_TYPES
                                     and toc_label(element).casefold() == title_words), None)
            anchor_elements = [matching_heading] if matching_heading else chapter['elements'][:1]
            regions = [region for element in anchor_elements
                       for region in element.get('pdf_regions', [])]
            pages = {region['page'] for region in regions}
            pages.update(int(ref[1]) for ref in REFERENCE.finditer(chapter['title']))
            chapter['title'] = align(chapter['title'], pages,
                                     regions[0]['bbox'] if regions else [0, 0, 0, 0])
        for element in chapter['elements']:
            if 'md_text' not in element:
                continue
            previous = old_elements.get(element['id'])
            if previous is None or element['md_text'] != previous.get('md_text'):
                regions = element.get('pdf_regions', [])
                pages = {region['page'] for region in regions}
                pages.update(int(ref[1]) for ref in REFERENCE.finditer(element['md_text']))
                element['md_text'] = align(element['md_text'], pages,
                                           regions[0]['bbox'] if regions else [0, 0, 0, 0])


def accept_exception(book, report, finding_id, note, source_text='', corrected_text=''):
    """Record one explicit, currently unresolved source exception."""
    finding = next((item for item in report
                    if item['status'] != 'matched' and item['id'] == finding_id), None)
    if finding is None:
        raise ValueError('Select one current unresolved literal')
    for value, limit, label in ((note, 500, 'Explanation'),
                                (source_text, 120, 'Source text'),
                                (corrected_text, 120, 'Corrected text')):
        if not isinstance(value, str) or len(value) > limit or (label == 'Explanation' and not value.strip()):
            raise ValueError(f'{label} is missing or too long')
    entry = {'anchor': finding['anchor'], 'element_id': finding['element_id'],
             'field': finding['path'].split('/')[-1], 'literal': finding['literal'],
             'note': note.strip()}
    if source_text.strip():
        entry['source_text'] = source_text.strip()
    if corrected_text.strip():
        entry['corrected_text'] = corrected_text.strip()
    book.setdefault('source_cleanup_exceptions', []).append(entry)


def accept_literal(book, report, finding_id):
    """Record baseline content without hiding its missing source address."""
    finding = next((item for item in report if item['id'] == finding_id and
                    item['status'] != 'matched' and item['token'] == item['literal']), None)
    if finding is None:
        raise ValueError('Select one current unanchored literal')
    parts = finding['path'].split('/')
    target = book['toc'] if parts[1] == 'toc' else book['chapters'][int(parts[2])]
    if len(parts) == 6:
        target = target['elements'][int(parts[4])]
    field = parts[-1]
    start, end = finding['offsets']
    template = target[field]
    if template[start:end] != finding['literal']:
        raise ValueError('Working text changed after the source preview')
    target[field] = template[:start] + accepted_literal(finding['literal']) + template[end:]


def apply(book, report, selections):
    """Apply only proposals named in a freshly recomputed preview."""
    matched = {item['id']: item
               for item in report if item['status'] == 'matched'}
    if (not isinstance(selections, list) or not selections or
            any(not isinstance(key, str) or key not in matched for key in selections) or
            len(selections) != len(set(selections))):
        raise ValueError('Select one or more current, unambiguous PDF matches')
    by_path = {}
    for key in selections:
        item = matched[key]
        by_path.setdefault(item['path'], []).append(item)
    for path, entries in by_path.items():
        parts = path.split('/')
        target = book['toc'] if parts[1] == 'toc' else book['chapters'][int(parts[2])]
        if len(parts) == 6:
            target = target['elements'][int(parts[4])]
        field = parts[-1]
        template = target[field]
        for entry in sorted(entries, key=lambda item: item['offsets'][0], reverse=True):
            start, end = entry['offsets']
            if template[start:end] != entry['token']:
                raise ValueError('Working text changed after the source preview')
            template = template[:start] + entry['reference'] + template[end:]
        target[field] = template

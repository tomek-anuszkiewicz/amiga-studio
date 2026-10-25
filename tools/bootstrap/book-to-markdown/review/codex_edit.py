"""One-element Codex transformations with exact authored-content checks."""

from copy import deepcopy
from html.parser import HTMLParser
import io
import json
from pathlib import Path
import sys
import traceback
import uuid

from PIL import Image
import yaml

from .content_validation import significant_tokens, text_tokens, validate_content
from .markdown import PARSER
from .model import HEADING_TYPES, digest, elements, write_json
from .text_recipe import (REFERENCE, compile_region_template, edit_template,
                          resolve_template)


PROGRAM = Path(__file__).resolve().parents[1]
REPOSITORY = PROGRAM.parents[2]
FORMAT_GUIDANCE = {
    'preserve': ('Keep the element in its current representation. Do not convert Markdown, HTML, '
                 'CSV or text to another format. If the requested edit requires a different '
                 'representation, return an error explaining why no change was made.'),
    'allow_change': ('You may change the element representation when the requested edit requires it. '
                     'When producing HTML, convert Markdown and LaTeX formatting in HTML text and '
                     'every table cell to equivalent HTML: use <strong>, <em>, <sub>, <sup>, <br> and '
                     '<span style="text-decoration: overline">...</span> as appropriate. '
                     'Do not leave Markdown math delimiters, LaTeX commands or Markdown emphasis '
                     'markers as literal cell text. Preserve each original word and substantive symbol. '
                     'You may restructure table rows and cells, including splitting, merging and changing spans. '
                     'If a notation cannot be converted without changing meaning, return an error.')
}
ORDER_GUIDANCE = {
    False: 'Keep all words, numbers and substantive symbols in their original reading order.',
    True: ('You may reorder the existing words, numbers and substantive symbols when the '
           'user instruction requires it. Preserve each original item, its case and its number of occurrences.'),
}
ADDITION_GUIDANCE = ('You may add only the following explicitly permitted content: {items}. '
                     'You may repeat listed content where the user instruction requires it. Preserve all original content. '
                     'Do not add any other words, numbers or substantive symbols.')


class HTMLTableDetector(HTMLParser):
    def __init__(self):
        super().__init__()
        self.found = False

    def handle_starttag(self, tag, attrs):
        if tag == 'table':
            self.found = True


def has_html_table(text):
    detector = HTMLTableDetector()
    for token in PARSER.parse(text):
        if token.type == 'html_block':
            detector.feed(token.content)
    detector.close()
    return detector.found
sys.path.insert(0, str(PROGRAM.parent))
from conversion.client import CodexClient
from common.pdf_artifacts import read_json
from common.pdf_elements import ELEMENT_TYPES


def source_text(root, element):
    """Use the current authored text, or positioned OCR for a textless graphic."""
    result = deepcopy(element)
    if result.get('resolved_md_text', '').strip():
        return result
    if result['type'] != 'graphic':
        raise ValueError('Selected element has no text to preserve')
    pages, parts = {}, []
    for region in result['pdf_regions']:
        page = read_json(root / 'pdf-to-markdown/02_preprocess' / f"page_{region['page']:04d}.json")
        pages[region['page']] = page['blocks']
        parts.append(compile_region_template(region['page'], page, region['bbox']))
    result['md_text'] = '\n'.join(parts)
    result['resolved_md_text'] = resolve_template(result['md_text'], pages)
    result['text_format'] = 'text'
    if not result['resolved_md_text'].strip():
        raise ValueError('This graphic has no stored text or OCR in its PDF regions')
    return result


def pdf_crop(root, regions):
    crops = []
    for region in regions:
        with Image.open(root / 'pdf/pages' / f"page_{region['page']:04d}.png") as image:
            bbox = region['bbox']
            if len(bbox) != 4 or not (0 <= bbox[0] < bbox[2] <= image.width
                                     and 0 <= bbox[1] < bbox[3] <= image.height):
                raise ValueError('Element PDF region is outside its source page')
            crops.append(image.crop(tuple(bbox)).convert('RGB'))
    if not crops:
        return None
    combined = Image.new('RGB', (max(c.width for c in crops), sum(c.height for c in crops)), 'white')
    top = 0
    for crop in crops:
        combined.paste(crop, (0, top))
        top += crop.height
    data = io.BytesIO()
    combined.save(data, format='PNG')
    return data.getvalue()


def replacement(root, original, response, format_mode='preserve', allow_reorder=False,
                allowed_additions=()):
    if not isinstance(response, dict):
        raise ValueError('Codex must return one JSON object')
    if set(response) == {'error'}:
        raise ValueError('Codex: ' + str(response['error']))
    required = {'id', 'type', 'format', 'text'}
    if not required <= response.keys() or response.keys() - required - {'heading_level'}:
        raise ValueError('Codex response must contain only id, type, format, text and optional heading_level')
    if response['id'] != original['id']:
        raise ValueError('Codex returned a different element ID')
    if response['type'] not in ELEMENT_TYPES:
        raise ValueError('Codex returned an unsupported element type')
    if not isinstance(response['text'], str) or not response['text'].strip():
        raise ValueError('Codex returned empty text')
    if response['format'] == 'csv' and response['type'] not in {'table', 'graphic'}:
        raise ValueError('CSV representation requires a table or graphic element')
    before_format = original.get('text_format', original.get('table_format', 'markdown'))
    if format_mode == 'preserve' and response['format'] != before_format:
        raise ValueError('Codex changed the element format in preserve mode')
    if response['type'] == 'table':
        html_table = has_html_table(response['text'])
        if response['format'] == 'markdown' and html_table:
            raise ValueError('Codex returned an HTML table declared as Markdown')
        if response['format'] == 'html' and not html_table:
            raise ValueError('Codex declared an HTML table without HTML table markup')
    validation = validate_content(original['resolved_md_text'], response['text'], before_format,
                                  response['format'], allow_reorder=allow_reorder,
                                  allowed_additions=allowed_additions)
    if response['type'] in HEADING_TYPES:
        level = response.get('heading_level')
        if type(level) is not int or not 1 <= level <= 6:
            raise ValueError('Codex heading_level must be 1 through 6')
    elif 'heading_level' in response:
        raise ValueError('Only headings may set heading_level')
    pages = {int(m[1]) for m in REFERENCE.finditer(original['md_text'])}
    blocks = {page: read_json(root / 'pdf-to-markdown/02_preprocess' / f'page_{page:04d}.json')['blocks']
              for page in pages}
    candidate = deepcopy(original)
    candidate.update(type=response['type'], text_format=response['format'],
                     md_text=edit_template(original['md_text'], blocks, response['text'],
                                           allow_reorder=allow_reorder),
                     resolved_md_text=response['text'])
    candidate.pop('asset', None)
    candidate.pop('heading_level', None)
    candidate.pop('table_format', None)
    if candidate['type'] in HEADING_TYPES:
        candidate['heading_level'] = response['heading_level']
    if candidate['type'] == 'table' or (candidate['type'] == 'graphic' and original.get('table_format')):
        candidate['table_format'] = response['format']
    return candidate, validation


def run_command(state, command, *, client_factory=None):
    client_factory = client_factory or CodexClient
    instruction = command.get('instruction')
    if not isinstance(instruction, str) or not instruction.strip():
        raise ValueError('Enter a Codex instruction')
    if len(instruction) > 12000:
        raise ValueError('Codex instruction is too long')
    format_mode = command.get('format_mode', 'preserve')
    if format_mode not in FORMAT_GUIDANCE:
        raise ValueError('Choose a supported Codex format mode')
    allow_reorder = command.get('allow_reorder', False)
    if type(allow_reorder) is not bool:
        raise ValueError('Codex text reordering permission must be a boolean')
    allow_additions = command.get('allow_additions', False)
    if type(allow_additions) is not bool:
        raise ValueError('Codex content addition permission must be a boolean')
    allowed_additions = command.get('allowed_additions', [])
    if not isinstance(allowed_additions, list) or any(
            not isinstance(item, str) or not item.strip() or ';' in item or len(item) > 200
            for item in allowed_additions) or len(allowed_additions) > 50:
        raise ValueError('List permitted additions separated by semicolons')
    allowed_additions = [item.strip() for item in allowed_additions]
    if allow_additions != bool(allowed_additions):
        raise ValueError('Enable additions and list the permitted content')
    if any(not significant_tokens(text_tokens(item)) for item in allowed_additions):
        raise ValueError('Each permitted addition must contain a word, number or substantive symbol')
    with state.lock:
        book, _, revision = state.input_cache.load(state.root / 'working')
        if revision != command['expected_revision']:
            raise ValueError('Stale Codex command; reload the latest authored revision')
        selected = next((e for e in elements(book) if e['id'] == command['id']), None)
        if selected is None:
            raise ValueError('Element no longer exists')
        original = source_text(state.root, selected)
        chapter = next(c for c in book['chapters'] if selected in c['elements'])
        chapter_context = {'id': chapter['id'],
                           'title': chapter.get('toc_title') or chapter.get('resolved_title', chapter['title'])}
        # Snapshot source pixels before inference, just as the authored text is.
        crop = pdf_crop(state.root, original['pdf_regions'])
    job = REPOSITORY / '.agent/tmp/book-codex' / uuid.uuid4().hex
    job.mkdir(parents=True)
    request = {'instruction': instruction.strip(), 'format_mode': format_mode,
               'allow_reorder': allow_reorder, 'allowed_additions': allowed_additions,
               'element': original,
               'chapter': chapter_context, 'allowed_types': ELEMENT_TYPES}
    write_json(job / 'request.json', {'revision': revision, **request})
    prompt = ((PROGRAM / 'prompts/codex-edit.md').read_text(encoding='utf-8') +
              '\n\nFORMAT RULE:\n' + FORMAT_GUIDANCE[format_mode] +
              '\n\nTEXT ORDER RULE:\n' + ORDER_GUIDANCE[allow_reorder] +
              '\n\nADDITION RULE:\n' +
              (ADDITION_GUIDANCE.format(items='; '.join(allowed_additions)) if allow_additions else
               'Do not add new words, numbers or substantive symbols.') +
              '\n\nREQUEST JSON:\n' + json.dumps(request, ensure_ascii=False))
    images = None
    if crop is not None:
        (job / 'source.png').write_bytes(crop)
        images = job / 'source.png'
    from .service import log
    log(f"[codex {original['id']}] started; diagnostics {job.relative_to(REPOSITORY)}")
    try:
        def validate(response):
            write_json(job / 'response.json', response)
            checked, _ = replacement(state.root, original, response, format_mode, allow_reorder,
                                     allowed_additions)
            if checked['type'] == 'graphic' and crop is None:
                raise ValueError('A graphic requires an original PDF region')

        config = yaml.safe_load((PROGRAM / 'config.yaml').read_text(encoding='utf-8'))
        with client_factory(config, stage='book_codex_edit', operation='book.codex_edit') as client:
            response = client.generate_json(prompt, image_path=images, validator=validate)
        candidate, report = replacement(state.root, original, response, format_mode, allow_reorder,
                                        allowed_additions)
        write_json(job / 'validation.json', report)
        asset = None
        if candidate['type'] == 'graphic':
            if crop is None:
                raise ValueError('A graphic requires an original PDF region')
            candidate['asset'] = f"assets/codex-{original['id']}-{digest(crop)[:12]}.png"
            asset = state.root / 'working' / candidate['asset']
        # Inference is outside the state lock. Existing mutation checks reject
        # changes to any book dependency made while the request was running.
        with state.lock:
            if state.input_cache.load(state.root / 'working')[2] != revision:
                raise ValueError('Working changed while Codex was running; result was not applied')
            created_asset = asset is not None and not asset.exists()
            try:
                if created_asset:
                    asset.parent.mkdir(parents=True, exist_ok=True)
                    asset.write_bytes(crop)
                def update(current):
                    element = next(e for e in elements(current) if e['id'] == original['id'])
                    element.clear()
                    element.update(candidate)
                state.mutate(revision, update)
            except BaseException:
                if created_asset:
                    asset.unlink(missing_ok=True)
                raise
        log(f"[codex {original['id']}] content verified; saved Working")
        return {'id': original['id'], 'validation': report}
    except Exception as error:
        detail = ''.join(traceback.format_exception(error))
        (job / 'error.log').write_text(detail, encoding='utf-8')
        log(f"[codex {original['id']}] failed: {detail}")
        raise

"""One-element Codex transformations with exact authored-content checks."""

from copy import deepcopy
import io
import json
from pathlib import Path
import sys
import traceback
import uuid

from PIL import Image
import yaml

from .content_validation import validate_content
from .model import HEADING_TYPES, digest, elements, write_json
from .text_recipe import (REFERENCE, compile_region_template, edit_template,
                          resolve_template)


PROGRAM = Path(__file__).resolve().parents[1]
REPOSITORY = PROGRAM.parents[2]
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


def replacement(root, original, response):
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
    validation = validate_content(original['resolved_md_text'], response['text'], before_format, response['format'])
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
                     md_text=edit_template(original['md_text'], blocks, response['text']),
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
    with state.lock:
        book, _, revision = state.input_cache.load(state.root / 'working')
        if revision != command['expected_revision']:
            raise ValueError('Stale Codex command; reload the latest authored revision')
        selected = next((e for e in elements(book) if e['id'] == command['id']), None)
        if selected is None:
            raise ValueError('Element no longer exists')
        original = source_text(state.root, selected)
        chapter = next(c for c in book['chapters'] if selected in c['elements'])
        chapter_context = {'id': chapter['id'], 'title': chapter.get('resolved_title', chapter['title'])}
        # Snapshot source pixels before inference, just as the authored text is.
        crop = pdf_crop(state.root, original['pdf_regions'])
    job = REPOSITORY / '.agent/tmp/book-codex' / uuid.uuid4().hex
    job.mkdir(parents=True)
    request = {'instruction': instruction.strip(), 'element': original,
               'chapter': chapter_context, 'allowed_types': ELEMENT_TYPES}
    write_json(job / 'request.json', {'revision': revision, **request})
    prompt = (PROGRAM / 'prompts/codex-edit.md').read_text(encoding='utf-8') + '\n\nREQUEST JSON:\n' + json.dumps(request, ensure_ascii=False)
    images = None
    if crop is not None:
        (job / 'source.png').write_bytes(crop)
        images = job / 'source.png'
    from .service import log
    log(f"[codex {original['id']}] started; diagnostics {job.relative_to(REPOSITORY)}")
    try:
        def validate(response):
            write_json(job / 'response.json', response)
            checked, _ = replacement(state.root, original, response)
            if checked['type'] == 'graphic' and crop is None:
                raise ValueError('A graphic requires an original PDF region')

        config = yaml.safe_load((PROGRAM / 'config.yaml').read_text(encoding='utf-8'))
        with client_factory(config, stage='book_codex_edit', operation='book.codex_edit') as client:
            response = client.generate_json(prompt, image_path=images, validator=validate)
        candidate, report = replacement(state.root, original, response)
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

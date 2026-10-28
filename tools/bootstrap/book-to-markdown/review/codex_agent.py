"""Bounded Codex tool loop over the authored Working book."""

from copy import deepcopy
import json
from pathlib import Path
import re
import sys
import time
import uuid

import yaml

from .model import elements, read_json, write_json
from .content_validation import validate_content
from .source_validation import validate_book
from .text_recipe import REFERENCE, resolve_template


PROGRAM = Path(__file__).resolve().parents[1]
REPOSITORY = PROGRAM.parents[2]
sys.path.insert(0, str(PROGRAM.parent))
from conversion.client import CodexClient


EDITOR_COMMANDS = {
    'heading': 'heading', 'type': 'change_type', 'role': 'change_role',
    'markdown': 'markdown', 'new_chapter': 'start_chapter', 'split': 'split',
    'merge': 'merge', 'merge_tables': 'merge_tables', 'move': 'move',
    'visibility': 'visibility', 'hide_page': 'hide_page', 'toc': 'toc',
}
PROMPT_PRESETS = {
    'describe_images': 'describe-images.md',
    'describe_tables': 'describe-tables.md',
}
SUPPORTING_TABLE_ROLES = {'legend', 'note', 'footnote'}
SUGGESTED_TASKS = {
    'join_paragraphs': 'join-paragraphs-across-pdf-pages.md',
    'source_cleanup': 'clean-direct-book-text.md',
    'describe_images': 'describe-images-working.md',
    'describe_tables': 'describe-tables-working.md',
}


def suggested_task_instructions():
    return {key: (PROGRAM / 'prompts' / filename).read_text(encoding='utf-8').strip()
            for key, filename in SUGGESTED_TASKS.items()}
MAX_STEPS = 500
MAX_ERRORS = 8


def validate_action(value):
    if not isinstance(value, dict) or value.get('action') not in {
            'list', 'read', 'image', 'source_audit', 'source_search',
            'editor', 'patch', 'source_replace', 'element_transform', 'finish'}:
        raise ValueError('Codex must return a supported tool action')
    if 'memory' in value and (not isinstance(value['memory'], str) or len(value['memory']) > 4000):
        raise ValueError('Codex memory must be at most 4000 characters')
    return value


def pointer(root, path, *, parent=False):
    if not isinstance(path, str) or not path.startswith(('/chapters/', '/toc/')):
        raise ValueError('Patch paths must be inside authored Working chapters or TOC')
    parts = [part.replace('~1', '/').replace('~0', '~') for part in path[1:].split('/')]
    current = root
    for part in parts[:-1] if parent else parts:
        if isinstance(current, list):
            if not part.isdecimal() or int(part) >= len(current):
                raise ValueError('Patch array index is out of range')
            current = current[int(part)]
        elif isinstance(current, dict):
            current = current[part]
        else:
            raise ValueError('Patch path does not identify a container')
    return (current, parts[-1]) if parent else current


def patch_book(book, operations):
    if not isinstance(operations, list) or not 1 <= len(operations) <= 20:
        raise ValueError('Patch requires 1 to 20 operations')
    for operation in operations:
        if not isinstance(operation, dict) or operation.get('op') not in {'add', 'replace', 'remove', 'move'}:
            raise ValueError('Unsupported patch operation')
        kind, path = operation['op'], operation.get('path')
        if kind == 'move':
            source, key = pointer(book, operation.get('from'), parent=True)
            if isinstance(source, list):
                if not key.isdecimal() or int(key) >= len(source):
                    raise ValueError('Patch array index is out of range')
                value = source.pop(int(key))
            else:
                value = source.pop(key)
            kind = 'add'
        else:
            value = deepcopy(operation.get('value'))
        target, key = pointer(book, path, parent=True)
        if isinstance(target, list):
            if kind == 'add':
                if key != '-' and (not key.isdecimal() or int(key) > len(target)):
                    raise ValueError('Patch array insertion is out of range')
                target.insert(len(target) if key == '-' else int(key), value)
            else:
                if not key.isdecimal() or int(key) >= len(target):
                    raise ValueError('Patch array index is out of range')
                if kind == 'remove':
                    target.pop(int(key))
                else:
                    target[int(key)] = value
        elif isinstance(target, dict):
            if kind == 'remove':
                del target[key]
            elif kind == 'replace':
                if key not in target:
                    raise ValueError('Patch replacement target does not exist')
                target[key] = value
            else:
                target[key] = value
        else:
            raise ValueError('Patch target is not a container')


def patch_description(book, action, target_id):
    operations = action.get('operations')
    if not isinstance(operations, list) or len(operations) != 1:
        raise ValueError('Description task requires one patch operation')
    operation = operations[0]
    match = next(((chapter_index, element_index, element)
                  for chapter_index, chapter in enumerate(book['chapters'])
                  for element_index, element in enumerate(chapter['elements'])
                  if element['id'] == target_id), None)
    if match is None:
        raise ValueError('Selected element no longer exists')
    chapter_index, element_index, element = match
    path = f'/chapters/{chapter_index}/elements/{element_index}/description'
    if (not isinstance(operation, dict) or operation.get('op') not in {'add', 'replace'}
            or operation.get('path') != path or not isinstance(operation.get('value'), str)
            or not operation['value'].strip()):
        raise ValueError('Description task can edit only the selected element description')
    description = operation['value'].strip()
    fenced = re.fullmatch(r'```text\r?\n(.+?)\r?\n```', description, re.DOTALL)
    if fenced is None or not fenced.group(1).strip():
        raise ValueError('Description requires one fenced text block')
    description = fenced.group(1)
    element['description'] = description


def patch_bulk_descriptions(book, action, prompt_preset, inspected_assets):
    operations = action.get('operations')
    if not isinstance(operations, list) or not 1 <= len(operations) <= 20:
        raise ValueError('Description patch requires 1 to 20 operations')
    expected_type = 'graphic' if prompt_preset == 'describe_images' else 'table'
    for operation in operations:
        if not isinstance(operation, dict):
            raise ValueError('Description patch requires element descriptions')
        match = re.fullmatch(r'/chapters/(\d+)/elements/(\d+)/description',
                             str(operation.get('path')))
        if match is None:
            raise ValueError('Description task can edit only element descriptions')
        chapter_index, element_index = map(int, match.groups())
        try:
            element = book['chapters'][chapter_index]['elements'][element_index]
        except (IndexError, KeyError):
            raise ValueError('Description target no longer exists') from None
        if element['type'] != expected_type:
            raise ValueError('Description target has the wrong element type')
        if expected_type == 'table' and element['role'] in SUPPORTING_TABLE_ROLES:
            raise ValueError('Cannot describe a supporting table')
        if expected_type == 'graphic' and element.get('asset') not in inspected_assets:
            raise ValueError('Inspect each Working image before writing its description')
        patch_description(book, {'operations': [operation]}, element['id'])


def inventory(book, action):
    ordered = [(chapter, element) for chapter in book['chapters'] for element in chapter['elements']]
    cursor, limit = action.get('cursor', 0), action.get('limit', 20)
    if type(cursor) is not int or type(limit) is not int or cursor < 0 or not 1 <= limit <= 50:
        raise ValueError('List requires a nonnegative cursor and limit from 1 to 50')
    rows = []
    for chapter, element in ordered[cursor:cursor + limit]:
        content = element.get('resolved_md_text', element.get('md_text', ''))
        rows.append({'id': element['id'], 'chapter': chapter['id'], 'type': element['type'],
                     'role': element['role'], 'visible': element.get('visible', True),
                     'pages': [region['page'] for region in element['pdf_regions']],
                     'length': len(content), 'beginning': content[:100], 'ending': content[-100:]})
    return {'total': len(ordered), 'cursor': cursor, 'next_cursor': min(cursor + limit, len(ordered)),
            'elements': rows}


def read_elements(book, action):
    ids = action.get('ids')
    if not isinstance(ids, list) or not 1 <= len(ids) <= 8 or any(not isinstance(i, str) for i in ids):
        raise ValueError('Read requires 1 to 8 element IDs')
    offset, length = action.get('offset', 0), action.get('length', 12000)
    if type(offset) is not int or type(length) is not int or offset < 0 or not 1 <= length <= 12000:
        raise ValueError('Read requires a nonnegative offset and length up to 12000')
    by_id = {element['id']: (chapter, element, f'/chapters/{chapter_index}/elements/{element_index}')
             for chapter_index, chapter in enumerate(book['chapters'])
             for element_index, element in enumerate(chapter['elements'])}
    result = []
    for identifier in ids:
        chapter, element, path = by_id[identifier]
        item = deepcopy(element)
        for name in ('md_text', 'resolved_md_text'):
            if name in item:
                text = item[name]
                item[name] = text[offset:offset + length]
                item[name + '_length'] = len(text)
        result.append({'element_path': path,
                       'chapter': {'id': chapter['id'], 'title': chapter.get('resolved_title', chapter['title']),
                                   'markdown_file': chapter.get('resolved_markdown_file', chapter['markdown_file'])},
                       'element': item})
    return {'offset': offset, 'elements': result}


def source_audit(book, workspace, action):
    cursor, limit = action.get('cursor', 0), action.get('limit', 20)
    if type(cursor) is not int or type(limit) is not int or cursor < 0 or not 1 <= limit <= 50:
        raise ValueError('Source audit requires a nonnegative cursor and limit from 1 to 50')
    identifier = action.get('id')
    if identifier is not None and not isinstance(identifier, str):
        raise ValueError('Source audit ID must be an element ID')
    report = validate_book(book, workspace)
    paths = {(chapter['id'], None, 'title'): f'/chapters/{chapter_index}/title'
             for chapter_index, chapter in enumerate(book['chapters'])}
    paths.update({(chapter['id'], element['id'], 'md_text'):
                  f'/chapters/{chapter_index}/elements/{element_index}/md_text'
                  for chapter_index, chapter in enumerate(book['chapters'])
                  for element_index, element in enumerate(chapter['elements'])})
    fields = {}
    for issue in report['issues']:
        if issue['code'] not in {'literal_content', 'literal_whitespace'}:
            continue
        if identifier is not None and issue['element_id'] != identifier:
            continue
        key = (issue['chapter_id'], issue['element_id'], issue['field'])
        entry = fields.setdefault(key, {'chapter_id': key[0], 'element_id': key[1],
                                        'field': key[2], 'path': paths[key],
                                        'issue_count': 0, 'literals': []})
        entry['issue_count'] += 1
        if issue['text'] not in entry['literals'] and len(entry['literals']) < 10:
            entry['literals'].append(issue['text'])
    rows = list(fields.values())
    return {'total': len(rows), 'cursor': cursor,
            'next_cursor': min(cursor + limit, len(rows)), 'fields': rows[cursor:cursor + limit]}


def source_search(workspace, action):
    query = action.get('query')
    pages = action.get('pages')
    if not isinstance(query, str) or not 3 <= len(query) <= 100 or not query.strip():
        raise ValueError('Source search requires 3 to 100 query characters')
    if pages is not None and (not isinstance(pages, list) or not 1 <= len(pages) <= 10
                              or any(type(page) is not int or page < 1 for page in pages)):
        raise ValueError('Source search accepts up to 10 positive page numbers')
    directory = workspace / '02_preprocess'
    if not directory.is_dir():
        raise ValueError('Prepared Stage 02 text is missing')
    paths = ([directory / f'page_{page:04d}.json' for page in pages] if pages is not None
             else sorted(directory.glob('page_*.json')))
    matches = []
    def result(truncated):
        response = {'matches': matches, 'truncated': truncated}
        if pages is not None and not any(match['exact_case'] for match in matches):
            response['hint'] = 'No exact-case match on these pages; search without pages to check the whole prepared book.'
        return response
    for path in paths:
        if not path.is_file():
            raise ValueError(f'Source page {path.stem} does not exist')
        page = int(path.stem.split('_')[-1])
        for block in read_json(path)['blocks']:
            content = block['text']
            for match in re.finditer(re.escape(query), content, re.IGNORECASE):
                found = match.start()
                matches.append({'page': page, 'block': block['block_id'],
                                'chars': [found, match.end()],
                                'exact_case': content[found:match.end()] == query,
                                'reference': ('{{text ' + f'page={page} block={block["block_id"]} '
                                              + f'chars={found}:{match.end()}' + '}}'),
                                'block_text': content[:1000], 'block_length': len(content)})
                if len(matches) == 20:
                    return result(True)
    return result(False)


def editor_arguments(root, name, args):
    """Convert source recipes in agent text arguments to editor-visible Markdown."""
    fields = ('first', 'second') if name == 'split' else ('text',) if name in {'markdown', 'merge'} else ()
    converted = dict(args)
    for field in fields:
        value = converted.get(field)
        if not isinstance(value, str) or not REFERENCE.search(value):
            continue
        pages = {}
        for page in {int(match[1]) for match in REFERENCE.finditer(value)}:
            path = root / 'pdf-to-markdown' / '02_preprocess' / f'page_{page:04d}.json'
            if not path.is_file():
                raise ValueError(f'Text reference points to missing PDF page {page}')
            pages[page] = read_json(path)['blocks']
        converted[field] = resolve_template(value, pages)
    return converted


def replace_source_text(book, root, action):
    path, template = action.get('path'), action.get('template')
    if not isinstance(path, str) or not path.endswith(('/md_text', '/title')):
        raise ValueError('Source replacement requires an md_text or title path')
    if not isinstance(template, str) or not REFERENCE.search(template):
        raise ValueError('Source replacement requires a PDF text reference')
    parent, key = pointer(book, path, parent=True)
    if not isinstance(parent, dict) or key not in parent:
        raise ValueError('Source replacement target does not exist')
    before = parent.get('resolved_md_text' if key == 'md_text' else 'resolved_title', parent[key])
    after = editor_arguments(root, 'markdown', {'text': template})['text']
    representation = (parent.get('text_format', parent.get('table_format', 'markdown'))
                      if key == 'md_text' else 'markdown')
    validate_content(before, after, representation, representation)
    parent[key] = template
    if key == 'md_text':
        parent['resolved_md_text'] = after


def agent_prompt(guide, instruction, selected, target_id, context_ids, step,
                 memory, result, revision, read_context):
    task_state = {
        'instruction': instruction.strip(), 'selected_id': selected, 'target_id': target_id,
        'context_ids': context_ids, 'step': step,
        'memory': memory, 'last_result': result, 'working_revision': revision,
    }
    if target_id is not None:
        task_state['read_context'] = {'elements': list(read_context.values())}
    return guide + '\n\nTASK STATE JSON:\n' + json.dumps(task_state, ensure_ascii=False)


def run_agent(state, command, *, client_factory=None, progress=None, cancelled=None,
              preview_prompt=False):
    """Let Codex request context and Working edits until it finishes or fails."""
    instruction = command.get('instruction')
    if not isinstance(instruction, str) or not instruction.strip() or len(instruction) > 12000:
        raise ValueError('Enter a Codex instruction of at most 12000 characters')
    task_kind = command.get('task_kind')
    if task_kind not in (None, 'source_cleanup'):
        raise ValueError('Unknown Codex Working task kind')
    prompt_preset = command.get('prompt_preset')
    if prompt_preset is not None and (not isinstance(prompt_preset, str)
                                      or prompt_preset not in PROMPT_PRESETS):
        raise ValueError('Unknown Codex Working prompt preset')
    target_id = command.get('target_id')
    reference_prompt = ((PROGRAM / 'prompts' / PROMPT_PRESETS[prompt_preset]).read_text(encoding='utf-8')
                        if target_id is not None and prompt_preset is not None else None)
    if target_id is not None and (not isinstance(target_id, str) or
                                  prompt_preset not in PROMPT_PRESETS or task_kind is not None):
        raise ValueError('Description task requires a selected element and description preset')
    context_ids = command.get('context_ids', [])
    if (not isinstance(context_ids, list) or any(not isinstance(i, str) for i in context_ids)
            or len(context_ids) != len(set(context_ids)) or
            (target_id is None and context_ids)):
        raise ValueError('Description context must contain distinct element IDs')
    with state.lock:
        book, _, revision = state.input_cache.load(state.root / 'working')
        if revision != command.get('expected_revision'):
            raise ValueError('Stale Codex command; reload the latest authored revision')
        selected = command.get('id')
        if selected is not None and selected not in {e['id'] for e in elements(book)}:
            raise ValueError('Selected element no longer exists')
        target_chapter = next((c for c in book['chapters']
                               if any(e['id'] == target_id for e in c['elements'])), None)
        target = (next((e for e in target_chapter['elements'] if e['id'] == target_id), None)
                  if target_chapter else None)
        if target_id is not None and (selected != target_id or target is None or
                                      target['type'] != ('graphic' if prompt_preset == 'describe_images' else 'table')):
            raise ValueError('Description target must match the selected image or table')
        if target_id is not None and target['type'] == 'table' and target['role'] in SUPPORTING_TABLE_ROLES:
            raise ValueError('Cannot describe a supporting table')
        context = {e['id']: e for e in target_chapter['elements']} if target_chapter else {}
        if any(identifier == target_id or identifier not in context or
               context[identifier]['role'] not in {'caption', 'note', 'legend', 'footnote'}
               for identifier in context_ids):
            raise ValueError('Description context must be labeled elements in the selected chapter')
        opening = {'toc': deepcopy(book['toc']),
                   'chapters': [{'id': c['id'], 'title': c.get('resolved_title', c['title']),
                                  'markdown_file': c.get('resolved_markdown_file', c['markdown_file']),
                                  'element_count': len(c['elements'])} for c in book['chapters']],
                   'element_count': sum(len(c['elements']) for c in book['chapters']),
                   'selected_id': selected, 'context_ids': context_ids}
    guide = (PROGRAM / 'prompts/codex-agent.md').read_text(encoding='utf-8')
    if reference_prompt is not None:
        wrapper = (PROGRAM / 'prompts/codex-agent-selected-task.md').read_text(encoding='utf-8').strip()
        guide += '\n\n' + wrapper.replace('{filename}', PROMPT_PRESETS[prompt_preset]).replace(
            '{prompt}', reference_prompt)
    if target_id is not None:
        guide += '\n\n' + (PROGRAM / 'prompts/codex-agent-one-description.md').read_text(encoding='utf-8')
    if preview_prompt:
        return agent_prompt(guide, instruction, selected, target_id, context_ids,
                            1, '', {'opening': opening}, revision, {})
    job = REPOSITORY / '.agent/tmp/book-codex-agent' / uuid.uuid4().hex
    job.mkdir(parents=True)
    write_json(job / 'request.json', {'revision': revision, 'instruction': instruction,
                                      'task_kind': task_kind,
                                      'prompt_preset': prompt_preset,
                                      'target_id': target_id,
                                      'context_ids': context_ids,
                                      'opening': opening})
    config = yaml.safe_load((PROGRAM / 'config.yaml').read_text(encoding='utf-8'))
    client_factory = client_factory or CodexClient
    memory, result, next_image, edits, errors = '', {'opening': opening}, None, 0, 0
    read_context = {}
    selected_image = None
    inspected_image = False
    inspected_assets = set()
    started = time.monotonic()
    with client_factory(config, stage='book_codex_agent', operation='book.codex_agent',
                        cancelled=cancelled) as client:
        for step in range(1, MAX_STEPS + 1):
            if cancelled is not None and cancelled.is_set():
                raise RuntimeError('Codex task was stopped')
            with state.lock:
                book, _, actual = state.input_cache.load(state.root / 'working')
            if actual != revision:
                raise ValueError('Working changed outside the Codex task; stopped before another action')
            if progress:
                progress(step=step, action='thinking', edits=edits)
            prompt = agent_prompt(guide, instruction, selected, target_id, context_ids,
                                  step, memory, result, revision, read_context)
            action = client.generate_json(prompt, image_path=next_image or selected_image,
                                          validator=validate_action)
            next_image = None
            if cancelled is not None and cancelled.is_set():
                raise RuntimeError('Codex task was stopped before applying its next action')
            with state.lock:
                if state.input_cache.load(state.root / 'working')[2] != revision:
                    raise ValueError('Working changed while Codex was thinking; action was not applied')
            if 'memory' in action:
                memory = action['memory']
            write_json(job / f'action_{step:04d}.json', action)
            kind = action['action']
            if target_id is not None and kind not in {'read', 'image', 'patch', 'finish'}:
                result = {'tool_error': 'Description task can only read the selected item, inspect its image, and patch its description',
                          'revision': revision}
                write_json(job / f'result_{step:04d}.json', result)
                errors += 1
                if errors >= MAX_ERRORS:
                    raise ValueError('Codex repeated invalid tool requests')
                continue
            if target_id is None and prompt_preset is not None and kind not in {
                    'list', 'read', 'image', 'patch', 'finish'}:
                result = {'tool_error': 'Description task can only list, read, inspect images, and patch descriptions',
                          'revision': revision}
                write_json(job / f'result_{step:04d}.json', result)
                errors += 1
                if errors >= MAX_ERRORS:
                    raise ValueError('Codex repeated invalid tool requests')
                continue
            if task_kind == 'source_cleanup' and kind in {'patch', 'editor', 'element_transform'}:
                result = {'tool_error': 'Use source_replace for this source cleanup task',
                          'revision': revision}
                write_json(job / f'result_{step:04d}.json', result)
                errors += 1
                if errors >= MAX_ERRORS:
                    raise ValueError('Codex repeated invalid tool requests')
                continue
            if progress:
                progress(step=step, action=kind, edits=edits)
            if kind == 'finish':
                summary = action.get('summary')
                if not isinstance(summary, str) or not summary.strip():
                    raise ValueError('Codex must summarize its completed task')
                return {'summary': summary, 'steps': step, 'edits': edits, 'revision': revision,
                        'duration_seconds': round(time.monotonic() - started, 2),
                        'diagnostics': str(job.relative_to(REPOSITORY))}
            try:
                if kind == 'list':
                    result = inventory(book, action)
                elif kind == 'read':
                    if target_id is not None and (not isinstance(action.get('ids'), list) or
                                                  any(identifier not in {target_id, *context_ids}
                                                      for identifier in action['ids'])):
                        raise ValueError('Description task can read only the selected group')
                    result = read_elements(book, action)
                    if len(json.dumps(result, ensure_ascii=False)) > 50000:
                        raise ValueError('Read result is too large; request fewer IDs or a shorter slice')
                    if target_id is not None:
                        read_context.update((item['element']['id'], item)
                                            for item in result['elements'])
                elif kind == 'source_audit':
                    result = source_audit(book, state.root / 'pdf-to-markdown', action)
                elif kind == 'source_search':
                    result = source_search(state.root / 'pdf-to-markdown', action)
                elif kind == 'image':
                    if 'page' in action:
                        page = action['page']
                        if target_id is not None and page not in {r['page'] for r in target['pdf_regions']}:
                            raise ValueError('Description task can inspect only the selected source page')
                        if type(page) is not int or page not in {p['page'] for p in state.pdf_pages}:
                            raise ValueError('Requested PDF page does not exist')
                        next_image = state.root / 'pdf/pages' / f'page_{page:04d}.png'
                        result = {'page': page, 'image_attached_to_next_turn': True}
                    else:
                        asset = action.get('asset')
                        if target_id is not None and asset != target.get('asset'):
                            raise ValueError('Description task can inspect only the selected image')
                        if not isinstance(asset, str) or not asset.startswith('assets/') or Path(asset).suffix != '.png':
                            raise ValueError('Image requires a PDF page or Working PNG asset')
                        next_image = (state.root / 'working' / asset).resolve()
                        if not next_image.is_relative_to((state.root / 'working' / 'assets').resolve()) or not next_image.is_file():
                            raise ValueError('Working image asset does not exist')
                        if target_id is not None:
                            inspected_image = True
                        if target_id is not None or prompt_preset == 'describe_images':
                            selected_image = next_image
                        inspected_assets.add(asset)
                        result = {'asset': asset, 'image_attached_to_next_turn': True}
                elif kind == 'patch':
                    if target_id is not None:
                        if target['type'] == 'graphic' and not inspected_image:
                            raise ValueError('Inspect the selected image before writing its description')
                        state.mutate(revision, lambda current: patch_description(current, action, target_id))
                    elif prompt_preset is not None:
                        state.mutate(revision, lambda current: patch_bulk_descriptions(
                            current, action, prompt_preset, inspected_assets))
                    else:
                        state.mutate(revision, lambda current: patch_book(current, action.get('operations')))
                    with state.lock:
                        revision = state.input_cache.load(state.root / 'working')[2]
                    edits += 1
                    if target_id is None and prompt_preset == 'describe_images':
                        selected_image = None
                    result = {'applied': True, 'revision': revision}
                elif kind == 'source_replace':
                    state.mutate(revision, lambda current: replace_source_text(current, state.root, action))
                    with state.lock:
                        revision = state.input_cache.load(state.root / 'working')[2]
                    edits += 1
                    result = {'applied': True, 'revision': revision}
                elif kind == 'editor':
                    name = action.get('command')
                    args = action.get('args')
                    if name not in EDITOR_COMMANDS or not isinstance(args, dict) or 'expected_revision' in args:
                        raise ValueError('Unsupported editor command')
                    converted = editor_arguments(state.root, name, args)
                    getattr(state, EDITOR_COMMANDS[name])({**converted, 'expected_revision': revision})
                    with state.lock:
                        revision = state.input_cache.load(state.root / 'working')[2]
                    edits += 1
                    result = {'applied': True, 'revision': revision}
                elif kind == 'element_transform':
                    from .codex_edit import run_command
                    identifier, request = action.get('id'), action.get('instruction')
                    if not isinstance(identifier, str) or not isinstance(request, str):
                        raise ValueError('Element transformation requires an ID and instruction')
                    result = run_command(state, {'id': identifier, 'instruction': request,
                                                 'expected_revision': revision,
                                                 'format_mode': 'allow_change'})
                    with state.lock:
                        revision = state.input_cache.load(state.root / 'working')[2]
                    edits += 1
                errors = 0
            except (KeyError, IndexError, TypeError, ValueError) as error:
                errors += 1
                result = {'tool_error': str(error), 'revision': revision}
                if errors >= MAX_ERRORS:
                    raise ValueError('Codex repeated invalid tool requests') from error
            write_json(job / f'result_{step:04d}.json', result)
            if target_id is not None and kind == 'patch' and result.get('applied'):
                target_type = 'image' if prompt_preset == 'describe_images' else 'table'
                return {'summary': f'Saved the selected {target_type} description.',
                        'steps': step, 'edits': edits, 'revision': revision,
                        'duration_seconds': round(time.monotonic() - started, 2),
                        'diagnostics': str(job.relative_to(REPOSITORY))}
    raise RuntimeError(f'Codex exceeded {MAX_STEPS} actions without finishing')

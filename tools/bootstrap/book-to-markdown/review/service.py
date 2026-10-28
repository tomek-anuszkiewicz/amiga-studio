"""Coalescing render queue, local HTTP/SSE and complete-bundle acceptance."""

from copy import deepcopy
from io import BytesIO
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import mimetypes
from pathlib import Path
import re
import shutil
import sys
import threading
from urllib.parse import unquote, urlparse
import uuid

from PIL import Image

from .model import (EDITABLE_TYPES, ELEMENT_ROLES, HEADING_TYPES, PROTECTED_TYPES, TOC_DEFAULTS, InputCache, authored_element, changes, confined, digest, edit_markdown, elements, export_markdown, json_bytes,
                    load_input, new_chapter, read_json, set_element_role, set_element_type, set_heading_level, toc_value, update_markdown_heading, write_json)
from .text_recipe import REFERENCE, slice_template
from .markdown import format_html_table, html_table_structure, table_structure
from .render import HERE, KATEX, Renderer


def log(message):
    """Console encoding must not turn a recoverable render error into a dead queue."""
    encoding = sys.stdout.encoding or "utf-8"
    print(message.encode(encoding, errors="backslashreplace").decode(encoding), flush=True)


def slice_source(root, template, start, end):
    pages = {int(ref[1]) for ref in REFERENCE.finditer(template)}
    sources = {page: read_json(root / 'pdf-to-markdown' / '02_preprocess' /
                               f'page_{page:04d}.json')['blocks'] for page in pages}
    return slice_template(template, start, end, sources)


def is_html_table(element):
    text = element.get('md_text', '')
    return (element.get('type') == 'table' and isinstance(text, str)
            and re.search(r'<table\b', text, re.I) is not None
            and re.search(r'</table\s*>', text, re.I) is not None)


def read_snapshot(directory):
    path = directory / "rendered.json"
    return json.loads(path.read_text(encoding="utf-8")) if path.exists() else None


def render_accepted(root, renderer):
    """Update presentation settings from Accepted's own authored model."""
    accepted = root / "accepted"
    previous = read_snapshot(accepted)
    if not previous:
        return previous
    book, assets, fingerprint = load_input(accepted)
    if previous["settings"] == renderer.settings and previous["input_fingerprint"] == fingerprint:
        return previous
    snapshot = renderer.render(accepted, book, assets, fingerprint, previous)
    export_markdown(book, accepted, previous)
    write_json(accepted / "rendered.json", snapshot)
    log(f"[accepted presentation] {snapshot['metrics']}")
    return snapshot


def replace_bundle(root, snapshot, source_name, target_name, expected_working, publish_book=False):
    """Prepare a coherent bundle before replacement; roll back a failed rename."""
    prepared = root / f".{target_name}-prepare-{uuid.uuid4().hex}"
    prepared_book = root / f".book-prepare-{uuid.uuid4().hex}.json" if publish_book else None
    backup = root / f".{target_name}-backup"
    target = root / target_name
    source = root / source_name
    prepared.mkdir()
    try:
        book = {k: deepcopy(snapshot[k]) for k in ("toc", "chapters")}
        if 'source_cleanup_exceptions' in snapshot:
            book['source_cleanup_exceptions'] = deepcopy(snapshot['source_cleanup_exceptions'])
        write_json(prepared / "book.json", book)
        write_json(prepared / "rendered.json", snapshot)
        export_markdown(book, prepared)
        revision = snapshot["revision"]
        shutil.copytree(source / "renders" / revision, prepared / "renders" / revision)
        source_assets = source / "renders" / revision / "assets"
        if source_assets.exists():
            shutil.copytree(source_assets, prepared / "assets")
        if load_input(source)[2] != snapshot["input_fingerprint"]:
            raise ValueError("Source changed during bundle preparation; reload and retry")
        if load_input(root / "working")[2] != expected_working:
            raise ValueError("Working changed during bundle preparation; reload and retry")
        if backup.exists():
            raise RuntimeError("Previous bundle recovery is required before replacement")
        if prepared_book is not None:
            prepared_book.write_bytes((prepared / "book.json").read_bytes())
        moved_renders = []
        replaced_target = False
        try:
            if target.exists():
                target.rename(backup)
            if target_name == "working" and backup.exists():
                # Keep old immutable URLs without duplicating every prior render.
                for old in (backup / "renders").iterdir():
                    destination = prepared / "renders" / old.name
                    if old.is_dir() and not destination.exists():
                        old.rename(destination)
                        moved_renders.append((destination, old))
            prepared.rename(target)
            replaced_target = True
            if prepared_book is not None:
                prepared_book.replace(root / "book.json")
        except BaseException:
            for destination, old in reversed(moved_renders):
                destination.rename(old)
            if replaced_target:
                shutil.rmtree(target)
            if backup.exists():
                backup.rename(target)
            raise
    finally:
        if prepared.exists():
            shutil.rmtree(prepared)
        if prepared_book is not None:
            prepared_book.unlink(missing_ok=True)
    if backup.exists():
        try:
            shutil.rmtree(backup)
        except OSError as error:
            # The replacement is already committed; report cleanup separately.
            log(f"[{target_name} backup cleanup] {error}")


def replace_accepted(root, snapshot, publish_book=False):
    replace_bundle(root, snapshot, "working", "accepted", snapshot["input_fingerprint"], publish_book)


class ReviewState:
    def __init__(self, root, width=1200, channel="msedge"):
        self.root = root
        self.width, self.channel = width, channel
        self.lock = threading.RLock()
        self.condition = threading.Condition(self.lock)
        self.stopping = threading.Event()
        self.input_cache = InputCache()
        for version in ("working", "accepted"):
            backup, target = root / f".{version}-backup", root / version
            if backup.exists():
                if target.exists():
                    shutil.rmtree(backup)
                else:
                    backup.rename(target)
        self.working = read_snapshot(root / "working")
        self.accepted = read_snapshot(root / "accepted")
        self.error, self.pending, self.sequence = None, True, 0
        self.failed_fingerprint = None
        self.codex_agent = None
        self.codex_stop = None
        self.pdf_pages = []
        for path in sorted((root / "pdf" / "pages").glob("page_*.png")):
            with Image.open(path) as image:
                self.pdf_pages.append({"page": int(path.stem.split("_")[1]), "image": f"pdf/pages/{path.name}",
                                       "width": image.width, "height": image.height})

    def announce(self):
        self.sequence += 1
        self.condition.notify_all()

    def status(self):
        from .codex_edit import FORMAT_GUIDANCE, ORDER_GUIDANCE, ADDITION_GUIDANCE
        from .codex_agent import suggested_task_instructions
        with self.lock:
            try:
                authored = self.input_cache.load(self.root / "working")[2]
                current = bool(self.working and self.working["input_fingerprint"] == authored)
                input_error = None
            except Exception as error:
                authored, current, input_error = None, False, str(error)
            diff = changes(self.accepted, self.working) if self.accepted and self.working else []
            accepted_elements = {e["id"]: authored_element(e) for e in elements(self.accepted)} if self.accepted else {}
            discardable_elements = [e["id"] for e in elements(self.working)
                                    if e["id"] in accepted_elements
                                    and authored_element(e) != accepted_elements[e["id"]]] if self.working else []
            agent_running = bool(self.codex_agent and self.codex_agent['state'] == 'running')
            editor_html = {}
            if self.working:
                for element in elements(self.working):
                    if is_html_table(element):
                        text = element.get('resolved_md_text', element['md_text'])
                        formatted = format_html_table(text)
                        if formatted != text:
                            editor_html[element['id']] = formatted
            return {"sequence": self.sequence, "working": self.working, "accepted": self.accepted,
                    "editor_html": editor_html,
                    "authored_revision": authored, "pending": self.pending or not current,
                    "error": input_error or self.error,
                    "can_accept": current and not self.pending and not self.error and not agent_running,
                    "can_discard": bool(current and not self.pending and not self.error
                                        and self.accepted and authored != self.accepted["input_fingerprint"]),
                    "discardable_elements": discardable_elements,
                    "changes": diff, "pdf_pages": self.pdf_pages, "heading_types": sorted(HEADING_TYPES),
                    "editable_types": EDITABLE_TYPES, "protected_types": PROTECTED_TYPES,
                    "roles": ELEMENT_ROLES, "codex_agent": deepcopy(self.codex_agent),
                    "codex_format_guidance": FORMAT_GUIDANCE,
                    "codex_order_guidance": ORDER_GUIDANCE,
                    "codex_addition_guidance": ADDITION_GUIDANCE,
                    "codex_suggested_tasks": suggested_task_instructions()}

    def render_loop(self):
        renderer = None
        try:
            renderer = Renderer(self.width, self.channel)
            refreshed_accepted = render_accepted(self.root, renderer)
            # Startup presentation refresh must not overwrite later acceptance.
            with self.lock:
                if refreshed_accepted:
                    self.accepted = refreshed_accepted
            while not self.stopping.is_set():
                try:
                    with self.lock:
                        observed_sequence = self.sequence
                        book, assets, fingerprint = self.input_cache.load(self.root / "working")
                        previous = self.working
                        needs_render = (not previous or previous["input_fingerprint"] != fingerprint
                                        or previous["settings"] != renderer.settings)
                        if self.failed_fingerprint == fingerprint or not needs_render:
                            if not needs_render and (self.pending or self.error):
                                self.pending = False
                                self.error = None
                                self.failed_fingerprint = None
                                self.announce()
                            continue_render = False
                        else:
                            self.pending, self.error = True, None
                            self.announce()
                            continue_render = True
                    if continue_render:
                        snapshot = renderer.render(self.root / "working", book, assets, fingerprint, previous)
                        with self.lock:
                            write_json(self.root / "working" / "rendered.json", snapshot)
                            export_markdown(book, self.root / "working", previous)
                            self.working = snapshot
                            self.pending = self.input_cache.load(self.root / "working")[2] != fingerprint
                            if self.accepted is None and not self.pending:
                                replace_accepted(self.root, snapshot)
                                self.accepted = read_snapshot(self.root / "accepted")
                            self.failed_fingerprint, self.error = None, None
                            self.announce()
                            log(f"[render] {snapshot['revision']}: {snapshot['metrics']}")
                except Exception as error:
                    with self.lock:
                        message = str(error)
                        if message != self.error:
                            self.error, self.pending = message, False
                            self.announce()
                            log(f"[render failed] {message}")
                        self.failed_fingerprint = locals().get("fingerprint")
                with self.condition:
                    self.condition.wait_for(
                        lambda: self.stopping.is_set() or self.sequence != observed_sequence,
                        timeout=0.5)
        except Exception as error:
            with self.lock:
                self.error, self.pending = str(error), False
                self.announce()
        finally:
            if renderer:
                renderer.close()

    def mutate(self, expected_revision, update, *, reconcile=True):
        with self.lock:
            original, _, fingerprint = self.input_cache.load(self.root / "working")
            book = deepcopy(original)
            if fingerprint != expected_revision:
                raise ValueError("Stale edit command; reload the latest authored revision")
            update(book)
            original_elements = {element['id']: element for element in elements(original)}
            for element in elements(book):
                previous = original_elements.get(element['id'])
                if (is_html_table(element) and (previous is None or
                        element['md_text'] != previous.get('md_text') or
                        element.get('text_format', element.get('table_format')) !=
                        previous.get('text_format', previous.get('table_format')))):
                    formatted = format_html_table(element.get('resolved_md_text', element['md_text']))
                    edit_markdown(self.root, element, formatted)
            if reconcile:
                from .source_cleanup import reconcile_working
                reconcile_working(book, original, self.root / 'pdf-to-markdown')
            self.input_cache.load(self.root / "working", book)
            if self.input_cache.load(self.root / "working")[2] != fingerprint:
                raise ValueError("Working changed before save; retry the command")
            write_json(self.root / "working" / "book.json", book)
            self.pending, self.error, self.failed_fingerprint = True, None, None
            self.announce()

    def heading(self, command):
        def update(book):
            element = next((e for e in elements(book) if e["id"] == command["id"]), None)
            if element is None:
                raise ValueError("Heading no longer exists")
            set_heading_level(element, command["level"])
        self.mutate(command["expected_revision"], update)

    def change_type(self, command):
        if command['type'] == 'graphic':
            from .codex_edit import pdf_crop
            with self.lock:
                book, _, revision = self.input_cache.load(self.root / 'working')
                if revision != command['expected_revision']:
                    raise ValueError('Stale edit command; reload the latest authored revision')
                element = next((e for e in elements(book) if e['id'] == command['id']), None)
                if element is None:
                    raise ValueError('Element no longer exists')
                if element['type'] != 'table' or element.get('asset'):
                    raise ValueError('Only a table without an image asset can change to an image')
                crop = pdf_crop(self.root, element['pdf_regions'])
                if crop is None:
                    raise ValueError('An image requires an original PDF region')
                asset_name = f"assets/image-{element['id']}-{digest(crop)[:12]}.png"
                asset = self.root / 'working' / asset_name
                created = not asset.exists()
                if not created and asset.read_bytes() != crop:
                    raise ValueError('Existing image asset does not match its source PDF crop')
                try:
                    if created:
                        asset.parent.mkdir(parents=True, exist_ok=True)
                        asset.write_bytes(crop)
                    def update(current):
                        target = next(e for e in elements(current) if e['id'] == command['id'])
                        target['type'] = 'graphic'
                        target['asset'] = asset_name
                    self.mutate(revision, update)
                except BaseException:
                    if created:
                        asset.unlink(missing_ok=True)
                    raise
            return
        def update(book):
            element = next((e for e in elements(book) if e['id'] == command['id']), None)
            if element is None:
                raise ValueError('Element no longer exists')
            set_element_type(element, command['type'])
        self.mutate(command['expected_revision'], update)

    def change_role(self, command):
        def update(book):
            element = next((e for e in elements(book) if e['id'] == command['id']), None)
            if element is None:
                raise ValueError('Element no longer exists')
            set_element_role(element, command['role'])
        self.mutate(command['expected_revision'], update)

    def visibility(self, command):
        if type(command["visible"]) is not bool:
            raise ValueError("Visible must be a boolean")

        def update(book):
            element = next((e for e in elements(book) if e["id"] == command["id"]), None)
            if element is None:
                raise ValueError("Element no longer exists")
            element["visible"] = command["visible"]
        self.mutate(command["expected_revision"], update)

    def markdown(self, command):
        def update(book):
            element = next((e for e in elements(book) if e['id'] == command['id']), None)
            if element is None:
                raise ValueError('Element no longer exists')
            text = format_html_table(command['text']) if (is_html_table(element) and
                   isinstance(command['text'], str) and re.search(r'<table\b', command['text'], re.I)
                   and re.search(r'</table\s*>', command['text'], re.I)) else command['text']
            edit_markdown(self.root, element, text)
        self.mutate(command['expected_revision'], update)

    def rename_chapters(self, command):
        entries = command.get('chapters')
        toc = command.get('toc')

        def update(book):
            if (not isinstance(entries, list) or len(entries) != len(book['chapters']) or
                    [entry.get('id') for entry in entries if isinstance(entry, dict)] !=
                    [chapter['id'] for chapter in book['chapters']]):
                raise ValueError('Provide every chapter in its current order')
            for chapter, entry in zip(book['chapters'], entries):
                title, filename = entry.get('title'), entry.get('markdown_file')
                if not isinstance(title, str) or not title.strip():
                    raise ValueError('Chapter title cannot be empty')
                if not isinstance(filename, str) or not filename.strip():
                    raise ValueError('Chapter Markdown filename cannot be empty')
                chapter['title'] = title
                chapter['markdown_file'] = filename

            if not isinstance(toc, dict):
                raise ValueError('Provide the generated table of contents')
            for field in ('title', 'markdown_file'):
                value = toc.get(field)
                if not isinstance(value, str) or not value.strip():
                    raise ValueError('Table of contents title and filename cannot be empty')
                if field in book['toc'] or value != TOC_DEFAULTS[field]:
                    book['toc'][field] = value

        self.mutate(command['expected_revision'], update)

    def codex(self, command):
        from .codex_edit import run_command
        return run_command(self, command)

    def preview_source_cleanup(self, command):
        from .source_cleanup import preview
        with self.lock:
            book, _, revision = self.input_cache.load(self.root / 'working')
            if revision != command.get('expected_revision'):
                raise ValueError('Stale source cleanup preview; reload Working')
            return {'revision': revision, 'findings': preview(book, self.root / 'pdf-to-markdown')}

    def apply_source_cleanup(self, command):
        from .source_cleanup import apply, preview
        def update(book):
            report = preview(book, self.root / 'pdf-to-markdown')
            apply(book, report, command.get('selections'))
        self.mutate(command.get('expected_revision'), update, reconcile=False)

    def accept_source_cleanup_exception(self, command):
        from .source_cleanup import accept_exception, preview
        def update(book):
            report = preview(book, self.root / 'pdf-to-markdown')
            accept_exception(book, report, command.get('finding_id'), command.get('note'),
                             command.get('source_text', ''), command.get('corrected_text', ''))
        self.mutate(command.get('expected_revision'), update)

    def accept_source_literal(self, command):
        from .source_cleanup import accept_literal, preview
        def update(book):
            report = preview(book, self.root / 'pdf-to-markdown')
            accept_literal(book, report, command.get('finding_id'))
        self.mutate(command.get('expected_revision'), update, reconcile=False)

    def start_codex_agent(self, command):
        from .codex_agent import run_agent
        instruction = command.get('instruction')
        if not isinstance(instruction, str) or not instruction.strip() or len(instruction) > 12000:
            raise ValueError('Enter a Codex instruction of at most 12000 characters')
        with self.lock:
            if self.codex_agent and self.codex_agent['state'] == 'running':
                raise ValueError('A Codex Working task is already running')
            if self.input_cache.load(self.root / 'working')[2] != command.get('expected_revision'):
                raise ValueError('Stale Codex command; reload the latest authored revision')
            task_id = uuid.uuid4().hex
            self.codex_stop = threading.Event()
            self.codex_agent = {'id': task_id, 'state': 'running', 'instruction': instruction.strip(),
                                'step': 0, 'action': 'starting', 'edits': 0}
            self.announce()

        def progress(**fields):
            with self.lock:
                if self.codex_agent['id'] == task_id:
                    self.codex_agent.update(fields)
                    self.announce()

        def work():
            try:
                result = run_agent(self, command, progress=progress, cancelled=self.codex_stop)
                with self.lock:
                    self.codex_agent.update(state='completed', **result)
                    self.announce()
            except Exception as error:
                log(f'[codex working {task_id}] failed: {error}')
                with self.lock:
                    self.codex_agent.update(state='failed', error=str(error))
                    self.announce()

        threading.Thread(target=work, name='book-codex-agent', daemon=True).start()
        return {'id': task_id}

    def preview_codex_agent(self, command):
        from .codex_agent import run_agent
        if command.get('target_id') is None:
            raise ValueError('Prompt preview requires a selected description target')
        return {'prompt': run_agent(self, command, preview_prompt=True)}

    def stop_codex_agent(self, command):
        with self.lock:
            if not self.codex_agent or self.codex_agent['id'] != command.get('id'):
                raise ValueError('Codex Working task no longer exists')
            if self.codex_agent['state'] != 'running':
                raise ValueError('Codex Working task has already finished')
            self.codex_stop.set()
            self.codex_agent['action'] = 'stopping'
            self.announce()

    def start_chapter(self, command):
        def update(book):
            for chapter_index, chapter in enumerate(book['chapters']):
                position = next((i for i, element in enumerate(chapter['elements'])
                                 if element['id'] == command['id']), None)
                if position is None:
                    continue
                if position == 0:
                    raise ValueError('This element already starts a chapter')
                following = chapter['elements'][position:]
                created = new_chapter(following, 'chapter-' + uuid.uuid4().hex,
                                      [c.get('resolved_markdown_file', c['markdown_file']) for c in book['chapters']]
                                      + [toc_value(book, 'markdown_file')])
                chapter['elements'] = chapter['elements'][:position]
                book['chapters'].insert(chapter_index + 1, created)
                return
            raise ValueError('Element no longer exists')
        self.mutate(command['expected_revision'], update)

    def split(self, command):
        def update(book):
            for chapter in book['chapters']:
                for index, element in enumerate(chapter['elements']):
                    if element['id'] != command['id']:
                        continue
                    first_text, second_text = command['first'], command['second']
                    if not all(isinstance(text, str) and text.strip() for text in (first_text, second_text)):
                        raise ValueError('Both split sections must contain text')
                    edit_markdown(self.root, element, first_text + second_text)
                    second = deepcopy(element)
                    # Both sections retain the source area; no new PDF geometry is inferred.
                    template = element['md_text']
                    for part, text, start, end in ((element, first_text, 0, len(first_text)),
                                                  (second, second_text, len(first_text), len(first_text + second_text))):
                        part['md_text'] = slice_source(self.root, template, start, end)
                        part['resolved_md_text'] = text
                        update_markdown_heading(part, text)
                    second['id'] = 'split-' + uuid.uuid4().hex
                    chapter['elements'].insert(index + 1, second)
                    return
            raise ValueError('Element no longer exists')
        self.mutate(command['expected_revision'], update)

    def duplicate(self, command):
        def update(book):
            for chapter in book['chapters']:
                for index, element in enumerate(chapter['elements']):
                    if element['id'] == command['id']:
                        copy = deepcopy(element)
                        copy['id'] = 'copy-' + uuid.uuid4().hex
                        chapter['elements'].insert(index + 1, copy)
                        return
            raise ValueError('Element no longer exists')
        self.mutate(command['expected_revision'], update)

    def move(self, command):
        if command['direction'] not in ('up', 'down'):
            raise ValueError('Move direction must be up or down')

        def update(book):
            ordered = [(chapter, element) for chapter in book['chapters'] for element in chapter['elements']]
            index = next((i for i, (_, e) in enumerate(ordered) if e['id'] == command['id']), None)
            if index is None:
                raise ValueError('Element no longer exists')
            offset = -1 if command['direction'] == 'up' else 1
            target = index + offset
            if not 0 <= target < len(ordered):
                raise ValueError('Element is already at the edge of the book')
            chapter, element = ordered[index]
            destination, neighbor = ordered[target]
            chapter['elements'].remove(element)
            if chapter is destination:
                position = destination['elements'].index(neighbor)
                destination['elements'].insert(position + (offset == 1), element)
            elif offset < 0:
                destination['elements'].append(element)
            else:
                destination['elements'].insert(0, element)
            if not chapter['elements']:
                book['chapters'].remove(chapter)
        self.mutate(command['expected_revision'], update)

    def merge_tables(self, command):
        def update(book):
            ordered = [(chapter, element) for chapter in book['chapters'] for element in chapter['elements']]
            index = next((i for i, (_, e) in enumerate(ordered) if e['id'] == command['id']), None)
            if index is None or index + 1 >= len(ordered):
                raise ValueError('No next table to merge')
            _, first = ordered[index]
            chapter, second = ordered[index + 1]
            if any(e['type'] != 'table' for e in (first, second)):
                raise ValueError('Both elements must be tables; move intervening elements first')
            if first['role'] != second['role']:
                raise ValueError('Elements have different roles; change their roles before merging')
            table_format = first.get('table_format')
            if table_format != second.get('table_format') or table_format not in {'markdown', 'html'}:
                raise ValueError('Tables must use the same Markdown or HTML format')
            first_text = first.get('resolved_md_text', first['md_text'])
            second_text = second.get('resolved_md_text', second['md_text'])
            if table_format == 'markdown':
                headers, alignment, _ = table_structure(first_text)
                next_headers, next_alignment, body_start = table_structure(second_text)
                if len(headers) != len(next_headers):
                    raise ValueError('Tables have different numbers of columns')
                if headers != next_headers:
                    raise ValueError('Table column names or their order differ')
                if alignment != next_alignment:
                    raise ValueError('Table column alignment differs')
                first_end = len(first_text.rstrip())
                second_end = len(second_text)
            else:
                columns, _, first_end = html_table_structure(first_text)
                next_columns, body_start, second_end = html_table_structure(second_text)
                if columns != next_columns:
                    raise ValueError('Tables have different numbers of columns')
            # Slice each original recipe so PDF references stay attached to their rows.
            edit_markdown(self.root, first, first_text)
            edit_markdown(self.root, second, second_text)
            first['md_text'] = (slice_source(self.root, first['md_text'], 0, first_end) + '\n'
                                + slice_source(self.root, second['md_text'], body_start, second_end)
                                + (slice_source(self.root, first['md_text'], first_end, len(first_text))
                                   if table_format == 'html' else ''))
            first['resolved_md_text'] = (first_text[:first_end] + '\n' + second_text[body_start:second_end]
                                         + (first_text[first_end:] if table_format == 'html' else ''))
            if table_format == 'html':
                first['md_text'] = format_html_table(first['md_text'])
                first['resolved_md_text'] = format_html_table(first['resolved_md_text'])
            for region in second['pdf_regions']:
                if region not in first['pdf_regions']:
                    first['pdf_regions'].append(deepcopy(region))
            chapter['elements'].remove(second)
        self.mutate(command['expected_revision'], update)

    def merge(self, command):
        def update(book):
            ordered = [(chapter, element) for chapter in book['chapters'] for element in chapter['elements']]
            index = next((i for i, (_, e) in enumerate(ordered) if e['id'] == command['id']), None)
            if index is None or index + 1 >= len(ordered):
                raise ValueError('Element has no next element to merge')
            _, first = ordered[index]
            chapter, second = ordered[index + 1]
            if second['id'] != command['next_id']:
                raise ValueError('Next element changed; reopen the merge dialog')
            if first['role'] != second['role']:
                raise ValueError('Elements have different roles; change their roles before merging')
            merged = deepcopy(first)
            following = deepcopy(second)
            # Normalize each recipe independently before concatenating provenance.
            edit_markdown(self.root, merged, first.get('resolved_md_text', ''))
            edit_markdown(self.root, following, second.get('resolved_md_text', ''))
            merged['md_text'] += '\n' + following['md_text']
            merged['resolved_md_text'] += '\n' + following['resolved_md_text']
            for region in following['pdf_regions']:
                if region not in merged['pdf_regions']:
                    merged['pdf_regions'].append(deepcopy(region))
            edit_markdown(self.root, merged, command['text'])
            first.clear()
            first.update(merged)
            chapter['elements'].remove(second)
        self.mutate(command['expected_revision'], update)

    def merge_graphics(self, command):
        from .codex_edit import pdf_crop
        with self.lock:
            book, _, revision = self.input_cache.load(self.root / 'working')
            if revision != command['expected_revision']:
                raise ValueError('Stale edit command; reload the latest authored revision')
            ordered = [(chapter, element) for chapter in book['chapters'] for element in chapter['elements']]
            index = next((i for i, (_, element) in enumerate(ordered)
                          if element['id'] == command['id']), None)
            if index is None or index + 1 >= len(ordered):
                raise ValueError('Graphic has no next element to merge')
            _, first = ordered[index]
            chapter, second = ordered[index + 1]
            if second['id'] != command['next_id']:
                raise ValueError('Next element changed; reopen the menu')
            if first['type'] != 'graphic' or second['type'] != 'graphic':
                raise ValueError('Both elements must be graphics')
            if second.get('md_text'):
                raise ValueError('The next graphic has stored text that cannot be merged')
            regions = first['pdf_regions'] + second['pdf_regions']
            pages = {region['page'] for region in regions}
            if len(pages) != 1:
                raise ValueError('Graphics must occupy the same PDF page')
            crop = pdf_crop(self.root, regions)
            asset_name = f"assets/image-{first['id']}-{digest(crop)[:12]}.png"
            asset = self.root / 'working' / asset_name
            created = not asset.exists()
            if not created and asset.read_bytes() != crop:
                raise ValueError('Existing image asset does not match its source PDF crop')
            try:
                if created:
                    asset.parent.mkdir(parents=True, exist_ok=True)
                    asset.write_bytes(crop)

                def update(current):
                    entries = [(owner, element) for owner in current['chapters']
                               for element in owner['elements']]
                    owner, target = entries[index + 1]
                    entries[index][1]['pdf_regions'] = deepcopy(regions)
                    entries[index][1]['asset'] = asset_name
                    owner['elements'].remove(target)

                self.mutate(revision, update)
            except BaseException:
                if created:
                    asset.unlink(missing_ok=True)
                raise

    def rotate_graphic(self, command):
        rotations = {'clockwise': Image.Transpose.ROTATE_270,
                     'counterclockwise': Image.Transpose.ROTATE_90,
                     'half_turn': Image.Transpose.ROTATE_180}
        if command.get('direction') not in rotations:
            raise ValueError('Unsupported image rotation')
        with self.lock:
            book, _, revision = self.input_cache.load(self.root / 'working')
            if revision != command['expected_revision']:
                raise ValueError('Stale edit command; reload the latest authored revision')
            element = next((e for e in elements(book) if e['id'] == command['id']), None)
            if element is None or element['type'] != 'graphic' or not element.get('asset'):
                raise ValueError('Element is not an image')
            source = confined(self.root / 'working', element['asset'])
            with Image.open(source) as image:
                rotated = image.transpose(rotations[command['direction']])
                output = BytesIO()
                rotated.save(output, format='PNG')
            data = output.getvalue()
            asset_name = f"assets/rotated-{element['id']}-{digest(data)[:12]}.png"
            asset = self.root / 'working' / asset_name
            created = not asset.exists()
            if not created and asset.read_bytes() != data:
                raise ValueError('Existing rotated image does not match its content')
            try:
                if created:
                    asset.parent.mkdir(parents=True, exist_ok=True)
                    asset.write_bytes(data)
                def update(current):
                    target = next(e for e in elements(current) if e['id'] == command['id'])
                    target['asset'] = asset_name
                self.mutate(revision, update)
            except BaseException:
                if created:
                    asset.unlink(missing_ok=True)
                raise

    def discard_element(self, command):
        with self.lock:
            if not self.accepted or self.accepted["revision"] != command["accepted_revision"]:
                raise ValueError("Discard requires the current Accepted revision")
            accepted, _, fingerprint = load_input(self.root / "accepted")
            if fingerprint != self.accepted["input_fingerprint"]:
                raise ValueError("Accepted changed; reload and retry")
            original = next((e for e in elements(accepted) if e["id"] == command["id"]), None)
            if original is None:
                raise ValueError("Element has no Accepted version")

            def update(book):
                element = next((e for e in elements(book) if e["id"] == command["id"]), None)
                if element is None:
                    raise ValueError("Element no longer exists")
                element.clear()
                element.update(deepcopy(authored_element(original)))
            self.mutate(command["expected_revision"], update)

    def hide_page(self, command):
        page = command["page"]
        if type(page) is not int or not any(item["page"] == page for item in self.pdf_pages):
            raise ValueError("Expected an existing PDF page number")

        def update(book):
            matched = [e for e in elements(book) if any(r["page"] == page for r in e["pdf_regions"])]
            if not matched:
                raise ValueError("PDF page has no Working elements to hide")
            for element in matched:
                element["visible"] = False
        self.mutate(command["expected_revision"], update)

    def toc(self, command):
        if type(command["depth"]) is not int or command["depth"] not in (1, 2, 3):
            raise ValueError("TOC depth must be 1, 2 or 3")
        self.mutate(command["expected_revision"],
                    lambda book: book['toc'].update(depth=command['depth']))

    def accept(self, command):
        with self.lock:
            status = self.status()
            if not status["can_accept"] or self.working["revision"] != command["revision"]:
                raise ValueError("Acceptance requires the newest complete Working revision")
            replace_accepted(self.root, self.working, publish_book=True)
            self.accepted = read_snapshot(self.root / "accepted")
            self.announce()

    def discard(self, command):
        with self.lock:
            status = self.status()
            if (not status["can_discard"] or status["authored_revision"] != command["expected_revision"]
                    or self.accepted["revision"] != command["accepted_revision"]):
                raise ValueError("Discard requires the current Working and Accepted revisions")
            replace_bundle(self.root, self.accepted, "accepted", "working", command["expected_revision"])
            self.working = read_snapshot(self.root / "working")
            self.pending, self.error, self.failed_fingerprint = False, None, None
            self.announce()


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):
        pass

    @property
    def state(self):
        return self.server.state

    def send_bytes(self, data, content_type, status=200):
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        path = unquote(urlparse(self.path).path)
        if path == "/api/status":
            self.send_bytes(json_bytes(self.state.status()), "application/json")
        elif path == "/api/events":
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream")
            self.send_header("Cache-Control", "no-cache")
            self.send_header("Connection", "close")
            self.end_headers()
            sequence = -1
            try:
                while not self.state.stopping.is_set():
                    with self.state.condition:
                        if self.state.sequence == sequence:
                            self.state.condition.wait(15)
                        sequence = self.state.sequence
                    self.wfile.write(f"data: {sequence}\n\n".encode())
                    self.wfile.flush()
            except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
                pass
            self.close_connection = True
        else:
            try:
                with self.state.lock:
                    if path in ("/", "/viewer.js", "/viewer.css"):
                        file = HERE / ("viewer.html" if path == "/" else path[1:])
                    elif path.startswith("/katex/"):
                        relative = path.removeprefix("/katex/")
                        if relative != "katex.min.css" and not (relative.startswith("fonts/") and Path(relative).suffix in {".woff", ".woff2", ".ttf"}):
                            raise FileNotFoundError()
                        file = confined(KATEX, relative)
                    elif path.startswith("/files/"):
                        relative = path.removeprefix("/files/")
                        if relative.split("/", 1)[0] not in {"working", "accepted", "pdf"}:
                            raise FileNotFoundError()
                        file = confined(self.state.root, relative)
                        if file.suffix not in {".png", ".md", ".json", ".html", ".css"}:
                            raise FileNotFoundError()
                    else:
                        raise FileNotFoundError()
                    data = file.read_bytes()
                self.send_bytes(data, mimetypes.guess_type(file.name)[0] or "application/octet-stream")
            except (ValueError, OSError):
                self.send_bytes(b"Not found", "text/plain", 404)

    def do_POST(self):
        try:
            origin = self.headers.get("Origin")
            if self.headers.get("Sec-Fetch-Site") == "cross-site" or (origin and origin != f"http://{self.headers.get('Host')}"):
                raise ValueError("Cross-origin commands are not allowed")
            if self.headers.get("Content-Type", "").split(";")[0] != "application/json":
                raise ValueError("Expected application/json")
            length = int(self.headers.get("Content-Length", "0"))
            if not 0 < length < 65536:
                raise ValueError("Invalid command length")
            command = json.loads(self.rfile.read(length))
            result = {"ok": True}
            if self.path == "/api/heading":
                self.state.heading(command)
            elif self.path == "/api/type":
                self.state.change_type(command)
            elif self.path == "/api/role":
                self.state.change_role(command)
            elif self.path == "/api/markdown":
                self.state.markdown(command)
            elif self.path == "/api/rename-chapters":
                self.state.rename_chapters(command)
            elif self.path == "/api/codex":
                result.update(self.state.codex(command))
            elif self.path == "/api/source-cleanup/preview":
                result.update(self.state.preview_source_cleanup(command))
            elif self.path == "/api/source-cleanup/apply":
                self.state.apply_source_cleanup(command)
            elif self.path == "/api/source-cleanup/exception":
                self.state.accept_source_cleanup_exception(command)
            elif self.path == "/api/source-cleanup/accept-literal":
                self.state.accept_source_literal(command)
            elif self.path == "/api/codex-agent":
                result.update(self.state.start_codex_agent(command))
            elif self.path == "/api/codex-agent/preview":
                result.update(self.state.preview_codex_agent(command))
            elif self.path == "/api/codex-agent/stop":
                self.state.stop_codex_agent(command)
            elif self.path == "/api/new-chapter":
                self.state.start_chapter(command)
            elif self.path == "/api/split":
                self.state.split(command)
            elif self.path == "/api/duplicate":
                self.state.duplicate(command)
            elif self.path == "/api/merge":
                self.state.merge(command)
            elif self.path == "/api/merge-graphics":
                self.state.merge_graphics(command)
            elif self.path == "/api/rotate-graphic":
                self.state.rotate_graphic(command)
            elif self.path == "/api/move":
                self.state.move(command)
            elif self.path == "/api/merge-tables":
                self.state.merge_tables(command)
            elif self.path == "/api/visibility":
                self.state.visibility(command)
            elif self.path == "/api/hide-page":
                self.state.hide_page(command)
            elif self.path == "/api/toc":
                self.state.toc(command)
            elif self.path == "/api/discard":
                self.state.discard(command)
            elif self.path == "/api/discard-element":
                self.state.discard_element(command)
            elif self.path == "/api/accept":
                self.state.accept(command)
            else:
                self.send_bytes(b"Not found", "text/plain", 404)
                return
            self.send_bytes(json_bytes(result), "application/json")
        except Exception as error:
            self.send_bytes(json_bytes({"error": str(error)}), "application/json", 409)


def serve(root, port, width, channel):
    state = ReviewState(root, width, channel)
    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    server.daemon_threads = True
    server.state = state
    worker = threading.Thread(target=state.render_loop, name="book-renderer", daemon=True)
    worker.start()
    print(f"Book review: http://127.0.0.1:{server.server_port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        state.stopping.set()
        if state.codex_stop:
            state.codex_stop.set()
        with state.condition:
            state.condition.notify_all()
        server.server_close()
        worker.join(timeout=10)

"""Authored model, prepared-page import and deterministic Markdown export."""

from copy import deepcopy
import hashlib
import json
from pathlib import Path
import re
import shutil
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "pdf-to-markdown"))
from common.pdf_artifacts import read_json
from common.pdf_page_conversion import read_conversion
from common.pdf_elements import ELEMENT_ROLES, ELEMENT_TYPES
from .markdown import PARSER, preserve_line_breaks
from .source_alignment import compile_page_template
from .text_recipe import (REFERENCE, escape_literal, edit_template,
                          literal_html_tags, join_adjacent_references,
                          resolve_template, source_differences)

HEADING_TYPES = {"heading", "toc_heading", "index_heading",
                 "list_of_tables_heading", "list_of_figures_heading"}
TOC_DEFAULTS = {"title": "Table of Contents", "markdown_file": "table-of-contents.md"}
WINDOWS_RESERVED_STEMS = {"CON", "PRN", "AUX", "NUL", *(f"COM{i}" for i in range(1, 10)),
                          *(f"LPT{i}" for i in range(1, 10))}
PROTECTED_TYPES = ('table', 'graphic')
EDITABLE_TYPES = ('heading', 'callout', 'callout_text', 'prose', 'code_block',
                  'preformatted_text')


def set_element_role(element, role):
    if role not in ELEMENT_ROLES:
        raise ValueError('Unsupported element role')
    element['role'] = role


def json_bytes(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":")).encode("utf-8")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def write_json(path, value):
    """Replace a single file only after its complete contents are written."""
    if path.name == "book.json":
        value = deepcopy(value)
        bundle = path.parent.name in {"working", "accepted"} or path.parent.name.startswith('.')
        root = path.parent.parent if bundle else path.parent
        workspace = root / 'pdf-to-markdown'
        for field in TOC_DEFAULTS:
            if field in value['toc']:
                value['toc'][field] = join_adjacent_references(value['toc'][field], workspace)
            value['toc'].pop('resolved_' + field, None)
        for chapter in value["chapters"]:
            for field in ("title", "markdown_file"):
                chapter[field] = join_adjacent_references(chapter[field], workspace)
            for element in chapter["elements"]:
                if "md_text" in element:
                    element["md_text"] = join_adjacent_references(element["md_text"], workspace)
        for element in elements(value):
            element.pop("resolved_md_text", None)
            element.pop("source_differences", None)
        for chapter in value["chapters"]:
            chapter.pop("resolved_title", None)
            chapter.pop("resolved_markdown_file", None)
            chapter.pop("toc_title", None)
        value = {"toc": value["toc"], **{k: v for k, v in value.items() if k != "toc"}}
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n",
                         encoding="utf-8")
    temporary.replace(path)


def confined(root, relative):
    path = (root / relative).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValueError(f"Path escapes bundle: {relative}")
    return path


def toc_value(book, field):
    toc = book['toc']
    return toc.get('resolved_' + field, toc.get(field, TOC_DEFAULTS[field]))


def valid_markdown_filename(filename):
    return (isinstance(filename, str) and filename.endswith('.md') and bool(filename[:-3].strip(' .'))
            and Path(filename).name == filename
            and filename.split('.', 1)[0].rstrip(' ').upper() not in WINDOWS_RESERVED_STEMS
            and re.search(r'[<>:"/\\|?*\x00-\x1f]', filename) is None)


def heading_text(element):
    return re.sub(r"^#{1,6}[ \t]*", "", element.get("resolved_md_text", element.get("md_text", ""))).strip()


def set_heading_level(element, level):
    if element["type"] not in HEADING_TYPES or type(level) is not int or not 1 <= level <= 6:
        raise ValueError("Expected a heading and an integer level from 1 to 6")
    element["heading_level"] = level
    element["md_text"] = "#" * level + " " + re.sub(r"^#{1,6}[ \t]*", "", element.get("md_text", "")).strip()
    if "resolved_md_text" in element:
        element["resolved_md_text"] = "#" * level + " " + re.sub(r"^#{1,6}[ \t]*", "", element["resolved_md_text"])


def set_element_type(element, kind):
    """Reclassify text roles while preserving source text and heading semantics."""
    if kind not in EDITABLE_TYPES:
        raise ValueError('Unsupported target type; table and graphic conversions are not allowed')
    if element['type'] in PROTECTED_TYPES or element.get('asset') or not element.get('md_text', '').strip():
        raise ValueError('Tables, graphics and elements without editable text cannot change type')
    previous = element['type']
    if previous == kind:
        return
    if previous in {'code_block', 'preformatted_text'} and kind not in {'code_block', 'preformatted_text'}:
        def without_outer_fence(value):
            lines = value.splitlines()
            if len(lines) < 2:
                return value
            opener = re.fullmatch(r' {0,3}(`{3,}|~{3,})[^\n]*', lines[0])
            if not opener:
                return value
            marker = opener[1]
            closer = r' {0,3}' + re.escape(marker[0]) + '{' + str(len(marker)) + r',}[ \t]*'
            return '\n'.join(lines[1:-1]) if re.fullmatch(closer, lines[-1]) else value
        element['md_text'] = without_outer_fence(element['md_text'])
        if 'resolved_md_text' in element:
            element['resolved_md_text'] = without_outer_fence(element['resolved_md_text'])
    element['type'] = kind
    if kind in HEADING_TYPES:
        level = element.get('heading_level', 1) if kind == 'heading' else 1
        set_heading_level(element, level)
    else:
        if previous in HEADING_TYPES:
            element['md_text'] = re.sub(r'^#{1,6}[ \t]*', '', element['md_text'])
            if 'resolved_md_text' in element:
                element['resolved_md_text'] = re.sub(r'^#{1,6}[ \t]*', '', element['resolved_md_text'])
        element.pop('heading_level', None)


def new_chapter(elements, chapter_id, existing_files=(TOC_DEFAULTS['markdown_file'],)):
    heading = next((e for e in elements if e["type"] in HEADING_TYPES), None)
    title = " ".join(heading_text(heading).split()) if heading else "Draft Chapter"
    title = title or "Draft Chapter"
    stem = re.sub(r"[^\w-]+", "-", title.lower(), flags=re.UNICODE).strip("-.")[:100]
    stem = stem or "chapter"
    # Windows reserved names must also remain usable on the initial machine.
    if stem.upper() in WINDOWS_RESERVED_STEMS:
        stem = "chapter-" + stem
    filename = stem + ".md"
    if filename.casefold() in {f.casefold() for f in existing_files}:
        filename = f"{stem}-{chapter_id}.md"
    title_template = re.sub(r"^#{1,6}[ \t]*", "", heading["md_text"]).strip() if heading else escape_literal(title)
    return {"id": chapter_id, "title": title_template, "resolved_title": title,
            "markdown_file": filename, "elements": elements}


def import_book(root):
    """Replace book outputs with final Stage 08 objects and Stage 06 assets."""
    working = root / "working"
    workspace = root / "pdf-to-markdown"
    status = read_json(workspace / "stage_status.json")
    for stage in ("02", "06", "07", "08"):
        if status.get(stage, {}).get("status") != "success":
            raise ValueError(f"Preparation stage {stage} is not successful")
    outputs = [confined(root, name) for name in ("working", "accepted", "pdf/pages")]
    if any(path == root.resolve() for path in outputs):
        raise ValueError("Import cleanup must not target the book root")
    for path in outputs:
        if path.exists():
            shutil.rmtree(path)
    elements = []
    assets = working / "assets"
    assets.mkdir(parents=True, exist_ok=True)
    for entry, page in read_conversion(workspace, "08_proofreading"):
        text_page = read_json(workspace / '02_preprocess' / f"{entry['page_id']}.json")
        for segment in page["segments"]:
            element = {"id": segment["segment_id"], "type": segment["type"],
                       "role": segment['role'],
                       "visible": True,
                       "pdf_regions": deepcopy(segment.get("pdf_regions",
                           [{"page": page["page"], "bbox": segment["bbox"]}]))}
            for field in ("md_text", "heading_level", "table_format"):
                if segment.get(field) is not None:
                    element[field] = segment[field]
            if element["type"] in HEADING_TYPES:
                set_heading_level(element, element.get("heading_level", 1))
            if "md_text" in element:
                # Conversion supplies formatting; source text comes from PDF ranges.
                prefix = "#" * element["heading_level"] + " " if element["type"] in HEADING_TYPES else ""
                template = prefix + compile_page_template(
                    page['page'], text_page, segment['bbox'], segment['md_text'])
                element.update(md_text=template, resolved_md_text=prefix + segment['md_text'])
            if element["type"] == "graphic":
                name = f"{element['id']}.png"
                shutil.copy2(workspace / "06_extract_crops" / "assets" / name, assets / name)
                element["asset"] = f"assets/{name}"
            elements.append(element)
    destination = root / "pdf" / "pages"
    destination.mkdir(parents=True, exist_ok=True)
    for image in sorted((workspace / "02_preprocess").glob("page_*.png")):
        shutil.copy2(image, destination / image.name)
    book = {"toc": {"depth": 2},
            "chapters": [new_chapter(elements, "chapter-001")]}
    write_json(working / "book.json", book)
    return book


def elements(book):
    for chapter in book["chapters"]:
        yield from chapter["elements"]


def edit_markdown(root, element, text):
    """Save visible Markdown while retaining source ranges and literal corrections."""
    if not isinstance(text, str) or not text.strip():
        raise ValueError("Markdown must contain text")
    if 'md_text' not in element or element.get('asset'):
        raise ValueError("This element has no editable Markdown text")
    template = element['md_text']
    pages = {int(m[1]) for m in REFERENCE.finditer(template)}
    blocks = {page: read_json(root / 'pdf-to-markdown' / '02_preprocess' / f'page_{page:04d}.json')['blocks']
              for page in pages}
    element['md_text'] = edit_template(template, blocks, text)
    if element.get('table_format') == 'html':
        element['md_text'] = literal_html_tags(element['md_text'], blocks)
    element['resolved_md_text'] = text
    update_markdown_heading(element, text)


def update_markdown_heading(element, text):
    """Keep heading metadata consistent with explicitly edited Markdown syntax."""
    heading = re.match(r'^(#{1,6})[ \t]+', text)
    if heading:
        if element['type'] not in HEADING_TYPES:
            element['type'] = 'heading'
        element['heading_level'] = len(heading[1])
    elif element['type'] in HEADING_TYPES:
        element['type'] = 'prose'
        element.pop('heading_level', None)


def authored_element(element):
    """Return element properties without resolved text."""
    return {k: v for k, v in element.items()
            if k not in {"resolved_md_text", "source_differences"}}


def chapter_value(chapter, field):
    return chapter.get("resolved_" + field, chapter[field])


def asset_names(element):
    names = set(re.findall(r"assets/[^\s)\"'<>]+", element.get("resolved_md_text", element.get("md_text", ""))))
    if element.get("asset"):
        names.add(element["asset"])
    return names


class InputCache:
    """Reuse immutable inputs while tracking external edits to every dependency."""
    def __init__(self):
        self.files = {}
        self.snapshots = {}
        self.dependencies = {}

    @staticmethod
    def signature(path):
        stat = path.stat()
        return stat.st_mtime_ns, stat.st_ctime_ns, stat.st_size, stat.st_ino

    def read(self, path, reader):
        signature = self.signature(path)
        self.dependencies[path] = signature
        cached = self.files.get(path)
        if cached is None or cached[0] != signature:
            value = reader(path)
            if self.signature(path) != signature:
                raise ValueError('Input changed while being read; retry')
            cached = signature, value
            self.files[path] = cached
        return cached[1]

    def load(self, working, book=None):
        cached = self.snapshots.get(working)
        if book is None and cached and all(self.signature(path) == signature for path, signature in cached[1].items()):
            return cached[0]
        self.dependencies = {}
        result = load_input(working, book, cache=self)
        if book is None:
            dependencies = dict(self.dependencies)
            if any(self.signature(path) != signature for path, signature in dependencies.items()):
                raise ValueError('Input changed during loading; retry')
            self.snapshots[working] = result, dependencies
        return result


def load_input(working, book=None, *, cache=None):
    """Read an immutable model and asset byte snapshot, with a content revision."""
    read_bytes = (lambda path: cache.read(path, Path.read_bytes)) if cache else Path.read_bytes
    read_source = (lambda path: cache.read(path, read_json)) if cache else read_json
    book = json.loads(read_bytes(working / "book.json")) if book is None else deepcopy(book)
    exceptions = book.get('source_cleanup_exceptions', [])
    if not isinstance(exceptions, list):
        raise ValueError('Source cleanup exceptions must be a list')
    anchors = set()
    for entry in exceptions:
        if (not isinstance(entry, dict) or
                not isinstance(entry.get('anchor'), str) or
                not re.fullmatch(r'[0-9a-f]{64}', entry['anchor']) or
                entry['anchor'] in anchors or
                not isinstance(entry.get('element_id'), str) or
                entry.get('field') not in {'md_text', 'title'} or
                not isinstance(entry.get('literal'), str) or not entry['literal'] or
                not isinstance(entry.get('note'), str) or not entry['note'].strip() or
                len(entry['note']) > 500 or
                any(not isinstance(entry.get(key), str) or len(entry[key]) > 120
                    for key in ('source_text', 'corrected_text') if key in entry)):
            raise ValueError('Invalid source cleanup exception')
        anchors.add(entry['anchor'])
    templates = [element.get("md_text", "") for element in elements(book)]
    templates += [chapter[field] for chapter in book["chapters"] for field in ("title", "markdown_file")]
    templates += [book['toc'][field] for field in TOC_DEFAULTS if field in book['toc']]
    referenced_pages = {int(match[1]) for template in templates for match in REFERENCE.finditer(template)}
    text_pages = {}
    for page in sorted(referenced_pages):
        blocks = read_source(working.parent / "pdf-to-markdown" / "02_preprocess" / f"page_{page:04d}.json")["blocks"]
        text_pages[page] = blocks
    block_index = {page: {b['block_id']: b['text'] for b in entries} for page, entries in text_pages.items()}
    resolve = lambda template: resolve_template(template, text_pages, block_index=block_index)
    for element in elements(book):
        if "md_text" in element:
            element["resolved_md_text"] = resolve(element["md_text"])
            element["source_differences"] = source_differences(
                element["md_text"], text_pages, block_index=block_index)
    for field in TOC_DEFAULTS:
        if field in book['toc']:
            if not isinstance(book['toc'][field], str) or not book['toc'][field].strip():
                raise ValueError('Table of contents title and filename cannot be empty')
            book['toc']['resolved_' + field] = resolve(book['toc'][field])
    ids, filenames, chapters = set(), set(), set()
    assets = {}
    if type(book["toc"]["depth"]) is not int or book["toc"]["depth"] not in (1, 2, 3):
        raise ValueError("TOC depth must be 1, 2 or 3")
    toc_filename = toc_value(book, 'markdown_file')
    if not valid_markdown_filename(toc_filename):
        raise ValueError("TOC Markdown filename must be a bundle-local .md file")
    filenames.add(toc_filename.casefold())
    for chapter in book["chapters"]:
        for field in ("title", "markdown_file"):
            chapter["resolved_" + field] = resolve(chapter[field])
        if chapter["id"] in chapters:
            raise ValueError("Duplicate chapter ID")
        chapters.add(chapter["id"])
        filename = chapter_value(chapter, "markdown_file")
        if not valid_markdown_filename(filename) or filename.casefold() in filenames:
            raise ValueError("Chapter Markdown filenames must be unique bundle-local .md files")
        filenames.add(filename.casefold())
        for element in chapter["elements"]:
            if element["type"] not in ELEMENT_TYPES:
                raise ValueError("Unsupported element type")
            if "description" in element and (element["type"] not in {"graphic", "table"}
                                              or not isinstance(element["description"], str)
                                              or not element["description"].strip()):
                raise ValueError("Only images and tables can have a description")
            if element["role"] not in ELEMENT_ROLES:
                raise ValueError("Unsupported element role")
            identifier = element["id"]
            if identifier in ids or not re.fullmatch(r"[A-Za-z0-9_-]+", identifier):
                raise ValueError("Element IDs must be unique anchor-safe identifiers")
            ids.add(identifier)
            if element["type"] in HEADING_TYPES:
                level = element["heading_level"]
                if type(level) is not int or not 1 <= level <= 6:
                    raise ValueError("Heading level must be 1 through 6")
            for name in asset_names(element):
                if not name.startswith("assets/"):
                    raise ValueError("Assets must be under assets/")
                assets[name] = read_bytes(confined(working, name))
    for chapter in book["chapters"]:
        chapter["toc_title"] = None
    for chapter, element in toc_entries(book):
        if chapter["toc_title"] is None:
            chapter["toc_title"] = toc_label(element) or None
    fingerprint = digest(json_bytes({"model": book, "assets": {k: digest(v) for k, v in sorted(assets.items())}}))
    return book, assets, fingerprint


def description_markdown(element):
    if not element.get('description'):
        return ''
    description = element['description'].rstrip('\r\n')
    fence = '`' * max(3, max((len(run) + 1 for run in re.findall(r'`+', description)), default=3))
    label = 'Image' if element['type'] == 'graphic' else 'Table'
    return (f'\n\n<details>\n<summary>{label} description</summary>\n\n'
            f'{fence}text\n{description}\n{fence}\n\n</details>')


def element_markdown(element):
    if not element.get("visible", True):
        return ""
    if element.get("asset"):
        image = f"![{element['id']}]({element['asset']})"
        return image + description_markdown(element)
    if element["type"] in HEADING_TYPES:
        text = element.get("resolved_md_text", element.get("md_text", ""))
        return preserve_line_breaks("#" * element["heading_level"] + " " + re.sub(r"^#{1,6}[ \t]*", "", text))
    text = element.get("resolved_md_text", element.get("md_text", ""))
    if element['type'] == 'table' and element.get('text_format', element.get('table_format')) == 'csv':
        fence = '`' * max(3, max((len(run) + 1 for run in re.findall(r'`+', text)), default=3))
        table = f'{fence}csv\n{text.rstrip()}\n{fence}'
        return table + description_markdown(element)
    if element["type"] in {"code_block", "preformatted_text"} and not any(
            token.type == 'fence' for token in PARSER.parse(text)):
        fence = "`" * max(3, max((len(run) + 1 for run in re.findall(r"`+", text)), default=3))
        return f"{fence}text\n{text}\n{fence}"
    result = preserve_line_breaks(text)
    return result + description_markdown(element)


def toc_entries(book):
    return [(chapter, e) for chapter in book["chapters"] for e in chapter["elements"]
            if e.get("visible", True) and e["type"] in HEADING_TYPES
            and e["heading_level"] <= book["toc"]["depth"]]


def toc_label(element):
    """Use heading words without carrying inline styling into contents links."""
    tokens = PARSER.parseInline(heading_text(element))[0].children or []
    return " ".join("".join(
        " " if token.type in {"softbreak", "hardbreak"} else token.content
        for token in tokens if token.type in {"text", "code_inline", "image", "math_inline",
                                       "softbreak", "hardbreak"}).split())


def toc_markdown(book):
    lines = ["# " + toc_value(book, 'title'), ""]
    for chapter, e in toc_entries(book):
        title = toc_label(e).replace("[", r"\[").replace("]", r"\]")
        lines.append("  " * (e["heading_level"] - 1) + f"- [{title}]({chapter_value(chapter, 'markdown_file')}#element-{e['id']})")
    return "\n".join(lines) + "\n"


def export_markdown(book, destination, previous=None):
    if previous is not None:
        current_files = {toc_value(book, 'markdown_file'),
                         *(chapter_value(chapter, 'markdown_file') for chapter in book['chapters'])}
        old_files = {toc_value(previous, 'markdown_file'),
                     *(chapter_value(chapter, 'markdown_file') for chapter in previous['chapters'])}
        for filename in old_files - current_files:
            confined(destination, filename).unlink(missing_ok=True)
    for chapter in book["chapters"]:
        parts = [f'<a id="element-{e["id"]}"></a>\n\n{element_markdown(e)}'
                 for e in chapter["elements"] if e.get("visible", True)]
        confined(destination, chapter_value(chapter, "markdown_file")).write_text("\n\n".join(parts) + "\n", encoding="utf-8")
    confined(destination, toc_value(book, 'markdown_file')).write_text(toc_markdown(book), encoding="utf-8")


def changes(accepted, working):
    """Compare authored values and placement, using current authored properties."""
    common = {e["id"] for e in elements(accepted)} & {e["id"] for e in elements(working)}
    common_chapters = {c["id"] for c in accepted["chapters"]} & {c["id"] for c in working["chapters"]}
    def indexed(snapshot):
        result = {}
        chapter_position, position = 0, 0
        for chapter in snapshot["chapters"]:
            for e in chapter["elements"]:
                data = dict(e)
                result[e["id"]] = (chapter["id"], position, data,
                                   chapter_position, chapter["title"], chapter["markdown_file"],
                                   chapter_value(chapter, "title"), chapter_value(chapter, "markdown_file"))
                if e["id"] in common:
                    position += 1
            if chapter["id"] in common_chapters:
                chapter_position += 1
        return result
    old, new = indexed(accepted), indexed(working)
    old_chapters = {c['id'] for c in accepted['chapters']}
    new_chapters = {c['id'] for c in working['chapters']} - old_chapters
    boundaries = {}
    for chapter in working['chapters']:
        if chapter['id'] in new_chapters and chapter['elements']:
            anchor = chapter['elements'][0]['id']
            previous = old.get(anchor)
            boundaries[anchor] = [previous[0], chapter['id']] if previous else [chapter['id']]
    old_hashes = accepted.get("asset_hashes", {})
    new_hashes = working.get("asset_hashes", {})
    result = []
    if (accepted["toc"]["depth"] != working["toc"]["depth"]
            or any(toc_value(accepted, field) != toc_value(working, field) for field in TOC_DEFAULTS)
            or toc_markdown(accepted) != toc_markdown(working)):
        result.append({"id": "generated-toc", "kind": "changed",
                       "accepted": {"depth": accepted["toc"]["depth"]},
                       "working": {"depth": working["toc"]["depth"]}})
    order = list(new)
    # Keep removed elements near their surviving successor in the old order.
    old_order = list(old)
    for index, identifier in enumerate(old_order):
        if identifier not in new:
            successor = next((key for key in old_order[index + 1:] if key in new), None)
            order.insert(order.index(successor) if successor else len(order), identifier)
    for identifier in order:
        before, after = old.get(identifier), new.get(identifier)
        names = asset_names(after[2] if after else before[2])
        comparison = after
        if before and after and after[0] in new_chapters:
            # A new chapter is one boundary change, not a text edit to every
            # member. Global order and authored content still compare normally.
            comparison = (before[0], after[1], after[2], *before[3:])
        edited = before != comparison or any(old_hashes.get(name) != new_hashes.get(name) for name in names)
        if edited or identifier in boundaries:
            result.append({"id": identifier, "kind": "added" if before is None else "removed" if after is None else "changed" if edited else "chapter_boundary",
                           "accepted": "missing" if before is None else "visible" if before[2].get("visible", True) else "hidden",
                           "working": "missing" if after is None else "visible" if after[2].get("visible", True) else "hidden",
                           **({'chapter_ids': boundaries[identifier]} if identifier in boundaries else {})})
    return result

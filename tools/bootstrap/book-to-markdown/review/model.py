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
from .markdown import PARSER, preserve_line_breaks
from .text_recipe import (MARKDOWN_REFERENCE, REFERENCE, escape_literal,
                          compile_markdown_template, region_blocks, resolve_template)

HEADING_TYPES = {"heading", "chapter", "toc_heading", "index_heading",
                 "list_of_tables_heading", "list_of_figures_heading"}


def json_bytes(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":")).encode("utf-8")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def write_json(path, value):
    """Replace a single file only after its complete contents are written."""
    if path.name == "book.json":
        value = deepcopy(value)
        for element in elements(value):
            element.pop("resolved_md_text", None)
        for chapter in value["chapters"]:
            chapter.pop("resolved_title", None)
            chapter.pop("resolved_markdown_file", None)
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


def heading_text(element):
    return re.sub(r"^#{1,6}[ \t]*", "", element.get("resolved_md_text", element.get("md_text", ""))).strip()


def set_heading_level(element, level):
    if element["type"] not in HEADING_TYPES or type(level) is not int or not 1 <= level <= 6:
        raise ValueError("Expected a heading and an integer level from 1 to 6")
    element["heading_level"] = level
    element["md_text"] = "#" * level + " " + re.sub(r"^#{1,6}[ \t]*", "", element.get("md_text", "")).strip()
    if "resolved_md_text" in element:
        element["resolved_md_text"] = "#" * level + " " + re.sub(r"^#{1,6}[ \t]*", "", element["resolved_md_text"])


def new_chapter(elements, chapter_id, existing_files=()):
    heading = next((e for e in elements if e["type"] in HEADING_TYPES), None)
    title = " ".join(heading_text(heading).split()) if heading else "Draft Chapter"
    title = title or "Draft Chapter"
    stem = re.sub(r"[^\w-]+", "-", title.lower(), flags=re.UNICODE).strip("-.")[:100]
    stem = stem or "chapter"
    # Windows reserved names must also remain usable on the initial machine.
    if stem.upper() in {"CON", "PRN", "AUX", "NUL", *(f"COM{i}" for i in range(1, 10)),
                       *(f"LPT{i}" for i in range(1, 10))}:
        stem = "chapter-" + stem
    filename = stem + ".md"
    if filename.casefold() in {f.casefold() for f in existing_files} or filename == "table-of-contents.md":
        filename = f"{stem}-{chapter_id}.md"
    title_template = re.sub(r"^#{1,6}[ \t]*", "", heading["md_text"]).strip() if heading else escape_literal(title)
    return {"id": chapter_id, "title": title_template, "resolved_title": title,
            "markdown_file": filename, "elements": elements}


def import_book(root, previous=None):
    """Use full Stage 05 page replacements; never consult Stage 03."""
    working = root / "working"
    if (working / "book.json").exists() or (root / "accepted").exists():
        raise FileExistsError("Book already imported; edit working/book.json instead")
    workspace = root / "pdf-to-markdown"
    status = read_json(workspace / "stage_status.json")
    for stage in ("02", "04", "05", "06"):
        if status.get(stage, {}).get("status") != "success":
            raise ValueError(f"Preparation stage {stage} is not successful")
    visibility = {e['id']: e.get('visible', True) for c in previous['chapters']
                  for e in c['elements']} if previous else {}
    elements = []
    assets = working / "assets"
    assets.mkdir(parents=True, exist_ok=True)
    for entry, baseline in read_conversion(workspace, "04_filter_page_content"):
        replacement = workspace / "05_transform_page_tables" / f"{entry['page_id']}_segments.json"
        page = read_json(replacement) if replacement.exists() else baseline
        text_page = read_json(workspace / '02_preprocess' / f"{entry['page_id']}.json")
        for segment in page["segments"]:
            if segment["type"] == "thumb_index":
                continue
            element = {"id": segment["segment_id"], "type": segment["type"],
                       "visible": visibility.get(segment['segment_id'], True),
                       "pdf_regions": [{"page": page["page"], "bbox": deepcopy(segment["bbox"])}]}
            for field in ("md_text", "heading_level", "table_format"):
                if segment.get(field) is not None:
                    element[field] = segment[field]
            if element["type"] in HEADING_TYPES:
                set_heading_level(element, element.get("heading_level", 1))
            if "md_text" in element:
                # Conversion supplies formatting; source text comes from PDF ranges.
                prefix = "#" * element["heading_level"] + " " if element["type"] in HEADING_TYPES else ""
                template = prefix + compile_markdown_template(
                    page['page'], region_blocks(text_page, segment['bbox']), segment['md_text'])
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
    book = {"toc": deepcopy(previous['toc']) if previous else {"depth": 2},
            "chapters": [new_chapter(elements, "chapter-001")]}
    write_json(working / "book.json", book)
    return book


def elements(book):
    for chapter in book["chapters"]:
        yield from chapter["elements"]


def authored_element(element):
    """Return element properties without resolved text or legacy render geometry."""
    return {k: v for k, v in element.items() if k not in {"resolved_md_text", "render_regions"}}


def chapter_value(chapter, field):
    return chapter.get("resolved_" + field, chapter[field])


def asset_names(element):
    names = set(re.findall(r"assets/[^\s)\"'<>]+", element.get("resolved_md_text", element.get("md_text", ""))))
    if element.get("asset"):
        names.add(element["asset"])
    return names


def load_input(working):
    """Read an immutable model and asset byte snapshot, with a content revision."""
    raw = (working / "book.json").read_bytes()
    book = json.loads(raw)
    templates = [element.get("md_text", "") for element in elements(book)]
    templates += [chapter[field] for chapter in book["chapters"] for field in ("title", "markdown_file")]
    referenced_pages = {int(match[1]) for template in templates for match in REFERENCE.finditer(template)}
    text_pages = {}
    for page in sorted(referenced_pages):
        blocks = read_json(working.parent / "pdf-to-markdown" / "02_preprocess" / f"page_{page:04d}.json")["blocks"]
        text_pages[page] = blocks
    prepared = {}
    converted_pages = {(match[1], int(match[2])) for template in templates
                       for match in MARKDOWN_REFERENCE.finditer(template)}
    for stage, page in sorted(converted_pages):
        directory = '04_filter_page_content' if stage == '04' else '05_transform_page_tables'
        data = read_json(working.parent / 'pdf-to-markdown' / directory / f'page_{page:04d}_segments.json')
        for segment in data['segments']:
            prepared[(stage, page, segment['segment_id'])] = segment.get('md_text', '')
    for element in elements(book):
        if "md_text" in element:
            element["resolved_md_text"] = resolve_template(element["md_text"], text_pages, prepared)
    ids, filenames, chapters = set(), {"table-of-contents.md"}, set()
    assets = {}
    # Older authored books may carry the retired visibility option. TOC is mandatory.
    book["toc"].pop("enabled", None)
    if type(book["toc"]["depth"]) is not int or book["toc"]["depth"] not in (1, 2, 3):
        raise ValueError("TOC depth must be 1, 2 or 3")
    for chapter in book["chapters"]:
        for field in ("title", "markdown_file"):
            chapter["resolved_" + field] = resolve_template(chapter[field], text_pages, prepared)
        if chapter["id"] in chapters:
            raise ValueError("Duplicate chapter ID")
        chapters.add(chapter["id"])
        filename = chapter_value(chapter, "markdown_file")
        if Path(filename).name != filename or not filename.endswith(".md") or filename.casefold() in filenames:
            raise ValueError("Chapter Markdown filenames must be unique bundle-local .md files")
        filenames.add(filename.casefold())
        for element in chapter["elements"]:
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
                assets[name] = confined(working, name).read_bytes()
    fingerprint = digest(json_bytes({"model": book, "assets": {k: digest(v) for k, v in sorted(assets.items())}}))
    return book, assets, fingerprint


def element_markdown(element):
    if not element.get("visible", True):
        return ""
    if element.get("asset"):
        return f"![{element['id']}]({element['asset']})"
    if element["type"] in HEADING_TYPES:
        text = element.get("resolved_md_text", element.get("md_text", ""))
        return preserve_line_breaks("#" * element["heading_level"] + " " + re.sub(r"^#{1,6}[ \t]*", "", text))
    text = element.get("resolved_md_text", element.get("md_text", ""))
    if element["type"] in {"code_block", "preformatted_text"} and not any(
            token.type == 'fence' for token in PARSER.parse(text)):
        fence = "`" * max(3, max((len(run) + 1 for run in re.findall(r"`+", text)), default=3))
        return f"{fence}text\n{text}\n{fence}"
    return preserve_line_breaks(text)


def toc_entries(book):
    return [(chapter, e) for chapter in book["chapters"] for e in chapter["elements"]
            if e.get("visible", True) and e["type"] in HEADING_TYPES
            and e["heading_level"] <= book["toc"]["depth"]]


def toc_markdown(book):
    lines = ["# Table of Contents", ""]
    for chapter, e in toc_entries(book):
        title = " ".join(heading_text(e).split()).replace("[", r"\[").replace("]", r"\]")
        lines.append("  " * (e["heading_level"] - 1) + f"- [{title}]({chapter_value(chapter, 'markdown_file')}#element-{e['id']})")
    return "\n".join(lines) + "\n"


def export_markdown(book, destination):
    for chapter in book["chapters"]:
        parts = [f'<a id="element-{e["id"]}"></a>\n\n{element_markdown(e)}'
                 for e in chapter["elements"] if e.get("visible", True)]
        confined(destination, chapter_value(chapter, "markdown_file")).write_text("\n\n".join(parts) + "\n", encoding="utf-8")
    (destination / "table-of-contents.md").write_text(toc_markdown(book), encoding="utf-8")


def changes(accepted, working):
    """Compare authored values and placement, excluding legacy derived geometry."""
    common = {e["id"] for e in elements(accepted)} & {e["id"] for e in elements(working)}
    common_chapters = {c["id"] for c in accepted["chapters"]} & {c["id"] for c in working["chapters"]}
    def indexed(snapshot):
        result = {}
        chapter_position = 0
        for chapter in snapshot["chapters"]:
            position = 0
            for e in chapter["elements"]:
                data = {k: v for k, v in e.items() if k != "render_regions"}
                result[e["id"]] = (chapter["id"], position, data,
                                   chapter_position, chapter["title"], chapter["markdown_file"],
                                   chapter_value(chapter, "title"), chapter_value(chapter, "markdown_file"))
                if e["id"] in common:
                    position += 1
            if chapter["id"] in common_chapters:
                chapter_position += 1
        return result
    old, new = indexed(accepted), indexed(working)
    old_hashes = accepted.get("asset_hashes", {})
    new_hashes = working.get("asset_hashes", {})
    result = []
    if accepted["toc"]["depth"] != working["toc"]["depth"]:
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
        if before != after or any(old_hashes.get(name) != new_hashes.get(name) for name in names):
            result.append({"id": identifier, "kind": "added" if before is None else "removed" if after is None else "changed",
                           "accepted": "missing" if before is None else "visible" if before[2].get("visible", True) else "hidden",
                           "working": "missing" if after is None else "visible" if after[2].get("visible", True) else "hidden"})
    return result

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
from common.pdf_selection import selected_pages

HEADING_TYPES = {"heading", "chapter", "toc_heading", "index_heading",
                 "list_of_tables_heading", "list_of_figures_heading"}
TEST_BOOK_HIDDEN = {f"page_{page:04d}_seg_{segment:03d}"
                    for page in (7, 8, 9) for segment in (1, 2)}


def json_bytes(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":")).encode("utf-8")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def write_json(path, value):
    """Replace a single file only after its complete contents are written."""
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
    return re.sub(r"^#{1,6}[ \t]*", "", element.get("md_text", "")).strip()


def set_heading_level(element, level):
    if element["type"] not in HEADING_TYPES or type(level) is not int or not 1 <= level <= 6:
        raise ValueError("Expected a heading and an integer level from 1 to 6")
    element["heading_level"] = level
    element["md_text"] = "#" * level + " " + heading_text(element)


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
    return {"id": chapter_id, "title": title, "markdown_file": filename, "elements": elements}


def import_book(root, hide_test_toc=False):
    """Use full Stage 05 page replacements; never consult Stage 03."""
    working = root / "working"
    if (working / "book.json").exists() or (root / "accepted").exists():
        raise FileExistsError("Book already imported; edit working/book.json instead")
    workspace = root / "pdf-to-markdown"
    status = read_json(workspace / "stage_status.json")
    for stage in ("02", "04", "05", "06"):
        if status.get(stage, {}).get("status") != "success":
            raise ValueError(f"Preparation stage {stage} is not successful")
    pages = selected_pages(status["05"].get("page_ranges"))
    elements = []
    assets = working / "assets"
    assets.mkdir(parents=True, exist_ok=True)
    for entry, baseline in read_conversion(workspace, "04_filter_page_content", pages=pages):
        replacement = workspace / "05_transform_page_tables" / f"{entry['page_id']}_segments.json"
        page = read_json(replacement) if replacement.exists() else baseline
        for segment in page["segments"]:
            if segment["type"] == "thumb_index":
                continue
            element = {"id": segment["segment_id"], "type": segment["type"],
                       "visible": not (hide_test_toc and segment["segment_id"] in TEST_BOOK_HIDDEN),
                       "pdf_regions": [{"page": page["page"], "bbox": deepcopy(segment["bbox"])}]}
            for field in ("md_text", "heading_level", "table_format"):
                if segment.get(field) is not None:
                    element[field] = segment[field]
            if element["type"] in HEADING_TYPES:
                set_heading_level(element, element.get("heading_level", 1))
            if element["type"] == "graphic":
                name = f"{element['id']}.png"
                shutil.copy2(workspace / "06_extract_crops" / "assets" / name, assets / name)
                element["asset"] = f"assets/{name}"
            elements.append(element)
    destination = root / "pdf" / "pages"
    destination.mkdir(parents=True, exist_ok=True)
    for image in sorted((workspace / "02_preprocess").glob("page_*.png")):
        shutil.copy2(image, destination / image.name)
    book = {"chapters": [new_chapter(elements, "chapter-001")],
            "toc": {"enabled": True, "depth": 2}}
    write_json(working / "book.json", book)
    return book


def elements(book):
    for chapter in book["chapters"]:
        yield from chapter["elements"]


def asset_names(element):
    names = set(re.findall(r"assets/[^\s)\"'<>]+", element.get("md_text", "")))
    if element.get("asset"):
        names.add(element["asset"])
    return names


def load_input(working):
    """Read an immutable model and asset byte snapshot, with a content revision."""
    raw = (working / "book.json").read_bytes()
    book = json.loads(raw)
    ids, filenames, chapters = set(), {"table-of-contents.md"}, set()
    assets = {}
    if type(book["toc"]["enabled"]) is not bool or type(book["toc"]["depth"]) is not int or book["toc"]["depth"] not in (1, 2, 3):
        raise ValueError("TOC depth must be 1, 2 or 3")
    for chapter in book["chapters"]:
        if chapter["id"] in chapters:
            raise ValueError("Duplicate chapter ID")
        chapters.add(chapter["id"])
        filename = chapter["markdown_file"]
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
        return "#" * element["heading_level"] + " " + heading_text(element)
    text = element.get("md_text", "")
    if element["type"] in {"code_block", "preformatted_text"} and not re.match(r"^\s{0,3}(`{3,}|~{3,})", text):
        fence = "`" * max(3, max((len(run) + 1 for run in re.findall(r"`+", text)), default=3))
        return f"{fence}text\n{text}\n{fence}"
    return text


def toc_entries(book):
    if not book["toc"]["enabled"]:
        return []
    return [(chapter, e) for chapter in book["chapters"] for e in chapter["elements"]
            if e.get("visible", True) and e["type"] in HEADING_TYPES
            and e["heading_level"] <= book["toc"]["depth"]]


def toc_markdown(book):
    if not book["toc"]["enabled"]:
        return ""
    lines = ["# Table of Contents", ""]
    for chapter, e in toc_entries(book):
        title = " ".join(heading_text(e).split()).replace("[", r"\[").replace("]", r"\]")
        lines.append("  " * (e["heading_level"] - 1) + f"- [{title}]({chapter['markdown_file']}#element-{e['id']})")
    return "\n".join(lines) + "\n"


def export_markdown(book, destination):
    for chapter in book["chapters"]:
        parts = [f'<a id="element-{e["id"]}"></a>\n\n{element_markdown(e)}'
                 for e in chapter["elements"] if e.get("visible", True)]
        confined(destination, chapter["markdown_file"]).write_text("\n\n".join(parts) + "\n", encoding="utf-8")
    (destination / "table-of-contents.md").write_text(toc_markdown(book), encoding="utf-8")


def changes(accepted, working):
    """Compare authored values and placement, excluding derived raster geometry."""
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
                                   chapter_position, chapter["title"], chapter["markdown_file"])
                if e["id"] in common:
                    position += 1
            if chapter["id"] in common_chapters:
                chapter_position += 1
        return result
    old, new = indexed(accepted), indexed(working)
    old_hashes = accepted.get("asset_hashes", {})
    new_hashes = working.get("asset_hashes", {})
    result = []
    if accepted["toc"] != working["toc"]:
        result.append({"id": "generated-toc", "kind": "changed",
                       "accepted": "visible" if accepted["toc"]["enabled"] else "hidden",
                       "working": "visible" if working["toc"]["enabled"] else "hidden"})
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

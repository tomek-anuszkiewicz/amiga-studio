"""Publish an accepted book into the local Markdown reference collection."""

import json
import re
import shutil
import tempfile
from collections import Counter
from pathlib import Path
from urllib.parse import quote

from .markdown import PARSER
from .model import toc_label, valid_markdown_filename


GENERATED_ANCHOR = re.compile(r'^<a id="element-([A-Za-z0-9_-]+)"></a>\n\n', re.MULTILINE)
HEADING_BLOCK_ID = re.compile(r'^(#{1,6} [^\n]*?) \^(element-[A-Za-z0-9-]+)(?=\n|$)', re.MULTILINE)
CONTENTS_LINK = re.compile(r'\[\[([^\]|\n]+)#\^([^\]|\n]+)\|([^\]\n]*)\]\]')


def publication_markdown(markdown_files: dict[str, str]) -> dict[str, str]:
    """Use ordinary heading links in the published copy of review Markdown."""
    headings = {}
    published = {}
    for name, source in markdown_files.items():
        lines = source.splitlines(keepends=True)
        chapter_headings = []
        for token in PARSER.parse(source):
            if token.type == "heading_open":
                index = token.map[0]
                line = lines[index].rstrip('\r\n')
                block = HEADING_BLOCK_ID.fullmatch(line)
                body = block.group(1) if block else line
                title = toc_label({"md_text": body})
                chapter_headings.append((index, body, block.group(2) if block else None, title))
        original_titles = Counter(title.casefold() for _, _, _, title in chapter_headings)
        used_titles = set(original_titles)
        seen = Counter()
        for index, body, block_id, title in chapter_headings:
            key = title.casefold()
            seen[key] += 1
            if block_id is None:
                continue
            suffix = ''
            if original_titles[key] > 1 and seen[key] > 1:
                number = seen[key]
                while (title + f' ({number})').casefold() in used_titles:
                    number += 1
                suffix = f' ({number})'
                used_titles.add((title + suffix).casefold())
            headings[(name, block_id)] = (title + suffix, suffix)
            lines[index] = body + suffix + lines[index][len(lines[index].rstrip('\r\n')):]
        published[name] = GENERATED_ANCHOR.sub('', ''.join(lines))

    def replace_link(match):
        filename, block_id, label = match.groups()
        try:
            title, suffix = headings[(filename, block_id)]
        except KeyError as error:
            raise ValueError(f"Contents target is missing: {filename}#^{block_id}") from error
        label += suffix
        label = label.replace('[', r'\[').replace(']', r'\]')
        return f"[{label}]({quote(filename, safe='')}#{quote(title, safe='')})"

    for name, source in published.items():
        published[name] = CONTENTS_LINK.sub(replace_link, source)
    return published


def publish_book(book_root: Path, reference_root: Path) -> tuple[Path, int, int]:
    recipe = json.loads((book_root / "book.json").read_text(encoding="utf-8"))
    accepted = book_root / "accepted"
    accepted_recipe = json.loads((accepted / "book.json").read_text(encoding="utf-8"))
    if recipe != accepted_recipe:
        raise ValueError("The root recipe differs from Accepted; accept the intended revision first")

    markdown_files = [recipe["toc"]["markdown_file"]]
    markdown_files.extend(chapter["markdown_file"] for chapter in recipe["chapters"])
    if (len(markdown_files) != len({name.casefold() for name in markdown_files})
            or any(not valid_markdown_filename(name) for name in markdown_files)):
        raise ValueError("The accepted Markdown filenames must be unique bundle-local names")
    for name in markdown_files:
        if not (accepted / name).is_file():
            raise FileNotFoundError(accepted / name)

    descriptions = {}
    assets = set()
    for chapter in recipe["chapters"]:
        for element in chapter["elements"]:
            if (element["type"] != "graphic" or not element.get("asset")
                    or not element.get("visible", True)):
                continue
            asset = element["asset"]
            asset_path = Path(asset)
            if asset_path.parts != ("assets", asset_path.name):
                raise ValueError(f"Invalid graphic asset path: {asset}")
            if not (accepted / asset_path).is_file():
                raise FileNotFoundError(accepted / asset_path)
            assets.add(asset_path.name)
            description = element.get("description")
            if description:
                if asset_path.name in descriptions and descriptions[asset_path.name] != description:
                    raise ValueError(f"Conflicting descriptions for {asset}")
                descriptions[asset_path.name] = description

    destination = reference_root / book_root.parent.name
    if destination.exists():
        raise FileExistsError(destination)
    sources = {name: (accepted / name).read_text(encoding="utf-8") for name in markdown_files}
    published_markdown = publication_markdown(sources)
    reference_root.mkdir(parents=True, exist_ok=True)
    prepared = Path(tempfile.mkdtemp(prefix=f".{destination.name}-", dir=reference_root))
    try:
        for name, text in published_markdown.items():
            (prepared / name).write_text(text, encoding="utf-8")
        if assets:
            target_assets = prepared / "assets"
            target_assets.mkdir()
            for name in sorted(assets):
                shutil.copy2(accepted / "assets" / name, target_assets / name)
            for name, description in descriptions.items():
                (target_assets / f"{name}.txt").write_text(
                    description.rstrip("\r\n") + "\n", encoding="utf-8"
                )
        prepared.rename(destination)
    finally:
        if prepared.exists():
            shutil.rmtree(prepared)
    return destination, len(assets), len(descriptions)

"""Deterministic selected-page Markdown and original-PNG crops."""

import json
from pathlib import Path

from .pdf_page_conversion import DEFERRED_TEXT_TYPES


def crop_objects(pages):
    """Retain the physical-page and segment-array order."""
    for entry, value in pages:
        for segment in value["segments"]:
            if segment["type"] in DEFERRED_TEXT_TYPES:
                yield entry, segment


def document_markdown(pages, source):
    # The upstream source identity supplies a filename, not publication metadata.
    title = Path(source["name"]).stem
    parts = [f"---\ntitle: {json.dumps(title, ensure_ascii=False)}\n---\n\n# {title}"]
    for entry, value in pages:
        for segment in value["segments"]:
            if segment["type"] in DEFERRED_TEXT_TYPES:
                identity = segment["segment_id"]
                parts.append(f"![{segment['type'].capitalize()} {identity}](assets/{identity}.png)")
            else:
                parts.append(segment["md_text"])
    return "\n\n".join(parts) + "\n"

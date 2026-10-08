"""Deterministic selected-page Markdown and original-PNG crops."""

import json
from pathlib import Path

from .pdf_page_conversion import DEFERRED_TEXT_TYPES
from .pdf_tables import converted


def segment_markdown(segment):
    if segment["type"] in DEFERRED_TEXT_TYPES and not converted(segment):
        identity = segment["segment_id"]
        return f"![{segment['type'].capitalize()} {identity}](assets/{identity}.png)"
    return segment.get("md_text", "")


def crop_objects(pages):
    """Retain the physical-page and segment-array order."""
    for entry, value in pages:
        for segment in value["segments"]:
            if segment["type"] in DEFERRED_TEXT_TYPES and not converted(segment):
                yield entry, segment


def document_markdown(pages, source):
    # The upstream source identity supplies a filename, not publication metadata.
    title = Path(source["name"]).stem
    parts = [f"---\ntitle: {json.dumps(title, ensure_ascii=False)}\n---\n\n# {title}"]
    for entry, value in pages:
        parts.extend(content for segment in value["segments"]
                     if (content := segment_markdown(segment)))
    return "\n\n".join(parts) + "\n"

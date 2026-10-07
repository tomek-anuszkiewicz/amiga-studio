"""Deterministic selected-page Markdown and runtime bundle validation."""

import json
from pathlib import Path

from PIL import Image

from .pdf_artifacts import require
from .pdf_page_conversion import DEFERRED_TEXT_TYPES


def crop_objects(pages):
    """Retain the validated physical-page and segment-array order."""
    seen = set()
    for entry, value in pages:
        for segment in value["segments"]:
            if segment["type"] in DEFERRED_TEXT_TYPES:
                identity = segment["segment_id"]
                require(identity not in seen,
                        f"Stage 02.9 {entry['page_id']}/{identity}: duplicate asset identity")
                seen.add(identity)
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


def validate_bundle(directory, pages, source):
    """Check complete output and every generated asset link before completion."""
    require(directory.is_dir(), "Stage 02.9 bundle is missing")
    require({path.name for path in directory.iterdir()} == {"document.md", "assets"},
            "Stage 02.9 must contain exactly document.md and assets/")
    document = directory / "document.md"
    require(document.is_file() and document.read_bytes().decode("utf-8") == document_markdown(pages, source),
            "Stage 02.9 document differs from ordered Stage 02 Markdown")
    assets = directory / "assets"
    require(assets.is_dir(), "Stage 02.9 assets directory is missing")
    crops = list(crop_objects(pages))
    actual = {path.name for path in assets.iterdir()}
    expected = {f"{s['segment_id']}.png" for _, s in crops}
    require(actual == expected, "Stage 02.9 asset coverage mismatch: "
            f"missing {sorted(expected - actual)}, unexpected {sorted(actual - expected)}")
    for entry, segment in crops:
        identity = segment["segment_id"]
        try:
            with Image.open(assets / f"{identity}.png") as image:
                x0, y0, x1, y1 = segment["bbox"]
                require(image.format == "PNG" and image.size == (x1-x0, y1-y0),
                        "crop format/dimensions mismatch")
                image.verify()
        except Exception as error:
            raise ValueError(f"Stage 02.9 {entry['page_id']}/{identity}: invalid crop: {error}") from error

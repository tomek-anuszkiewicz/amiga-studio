#!/usr/bin/env python3
"""Stage 02.9: retained page Markdown with exact original-PNG raster crops."""

import argparse
from pathlib import Path
import sys
import tempfile

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion.config import load_config, PDF_STAGES
from common.pdf_artifacts import source_pdf
from common.pdf_page_conversion import read_conversion
from common.pdf_page_markdown import crop_objects, document_markdown


def emit_page_markdown(workspace, config_path=None):
    source = {"name": source_pdf(config_path or workspace / "config.yaml").name}
    pages = list(read_conversion(workspace, "02.8_filter_page_content"))
    crops = list(crop_objects(pages))
    crops_by_page = {}
    for entry, segment in crops:
        crops_by_page.setdefault(entry["page_id"], []).append(segment)
    output = workspace / "02.9_emit_page_markdown"
    with tempfile.TemporaryDirectory(prefix=".02.9-", dir=workspace) as temporary:
        temporary = Path(temporary)
        bundle = temporary / "bundle"
        assets = bundle / "assets"
        assets.mkdir(parents=True)
        # Open each original PNG once; no point conversion, padding or resizing.
        for entry, value in pages:
            page_crops = crops_by_page.get(entry["page_id"], [])
            if not page_crops:
                continue
            context = entry["page_id"]
            try:
                with Image.open(workspace / entry["png_file"]) as image:
                    image.load()
                    for segment in page_crops:
                        context = f"{entry['page_id']}/{segment['segment_id']}"
                        with image.crop(tuple(segment["bbox"])) as crop:
                            crop.save(assets / f"{segment['segment_id']}.png", format="PNG")
            except Exception as error:
                raise ValueError(f"Stage 02.9 {context}: crop write failed: {error}") from error
        (bundle / "document.md").write_text(document_markdown(pages, source), encoding="utf-8", newline="\n")
        # Keep any previous bundle until the candidate has been written. Moving
        # it into the temporary directory also removes all stale assets on success.
        previous = temporary / "previous"
        if output.exists():
            output.rename(previous)
        try:
            bundle.rename(output)
        except BaseException:
            if previous.exists():
                previous.rename(output)
            raise
    print(f"[markdown] {len(pages)} pages, {len(crops)} crops: {output / 'document.md'}")


def main():
    parser = argparse.ArgumentParser(description="Stage 02.9: page Markdown and original-PNG crops")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args()
    load_config(args.config, known_stages=PDF_STAGES, required_stages=())
    emit_page_markdown(args.workspace.resolve(), args.config)


if __name__ == "__main__":
    main()

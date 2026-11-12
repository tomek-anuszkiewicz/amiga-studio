#!/usr/bin/env python3
"""Stage 06: extract graphic segments from unchanged Stage 02 page images."""

import argparse
from pathlib import Path
import sys

from PIL import Image, __version__ as pillow_version

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from common.pdf_cache import PreparationCache, file_digest, runtime_identity
from common.pdf_config import load_config
from common.pdf_page_conversion import read_conversion
from common.pdf_selection import selected_pages

STAGE = "06_extract_crops"


def extract_crops(workspace, pages=None):
    output = workspace / STAGE / "assets"
    output.mkdir(parents=True, exist_ok=True)
    cache = PreparationCache(STAGE)
    runtime = {**runtime_identity(), "pillow": pillow_version}
    count = 0
    for entry, page in read_conversion(workspace, "05_transform_page_tables", pages=pages):
        graphics = [segment for segment in page["segments"] if segment["type"] == "graphic"]
        if not graphics:
            continue
        source = workspace / entry["png_file"]
        source_sha256 = file_digest(source)
        with Image.open(source) as image:
            for graphic in graphics:
                asset = output / f"{graphic['segment_id']}.png"
                request = {"contract": "1", "page_png_sha256": source_sha256,
                           "bbox": graphic["bbox"], "runtime": runtime}
                outputs = {"crop.png": asset}
                count += 1
                if cache.restore(request, outputs):
                    print(f"[cache] Stage 06: restored {graphic['segment_id']} PNG", flush=True)
                    continue
                with image.crop(tuple(graphic["bbox"])) as crop:
                    crop.save(asset)
                cache.save(request, outputs)
                print(f"[crop] {graphic['segment_id']}: {graphic['bbox']}", flush=True)
    print(f"[crops] saved {count} graphic PNGs to {output}", flush=True)


def main():
    parser = argparse.ArgumentParser(description="Stage 06: extract graphic crops")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    load_config(args.config)
    extract_crops(args.workspace.resolve(), pages=selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

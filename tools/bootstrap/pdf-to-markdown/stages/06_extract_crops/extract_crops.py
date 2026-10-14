#!/usr/bin/env python3
"""Stage 06: extract graphic segments from unchanged Stage 02 page images."""

import argparse
from pathlib import Path
import sys

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from common.pdf_artifacts import read_json
from common.pdf_config import load_config
from common.pdf_page_conversion import read_conversion
from common.pdf_selection import selected_pages

STAGE = "06_extract_crops"


def extract_crops(workspace, pages=None):
    output = workspace / STAGE / "assets"
    output.mkdir(parents=True, exist_ok=True)
    count = 0
    for entry, page in read_conversion(workspace, "04_filter_page_content", pages=pages):
        replacement = workspace / "05_transform_page_tables" / f"{entry['page_id']}_segments.json"
        if replacement.exists():
            page = read_json(replacement)
        graphics = [segment for segment in page["segments"] if segment["type"] == "graphic"]
        if not graphics:
            continue
        with Image.open(workspace / entry["png_file"]) as image:
            for graphic in graphics:
                asset = output / f"{graphic['segment_id']}.png"
                with image.crop(tuple(graphic["bbox"])) as crop:
                    crop.save(asset)
                count += 1
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

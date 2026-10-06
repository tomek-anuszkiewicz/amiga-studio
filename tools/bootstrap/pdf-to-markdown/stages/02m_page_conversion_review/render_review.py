#!/usr/bin/env python3
"""Stage 02m: ordered labels and model-selected boxes for every object."""

import argparse
import importlib.util
from pathlib import Path
import sys

from PIL import Image
sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from conversion.config import load_config, PDF_STAGES
from conversion.pdf_page_conversion import validate_conversion

spec = importlib.util.spec_from_file_location(
    "segmentation_review", Path(__file__).resolve().parents[1] / "02k_segmentation_review/render_review.py")
review = importlib.util.module_from_spec(spec)
spec.loader.exec_module(review)


def review_image(image, value):
    width, height = image.size
    raster = {"display_to_pixels": [1, 0, 0, 1, 0, 0],
              "pixel_width": width, "pixel_height": height}
    return review.review_image(image, {"raster": raster}, value["segments"],
                               include_continuation=True)


def render_reviews(workspace):
    pages = validate_conversion(workspace)
    output = workspace / "02m_page_conversion_review"
    output.mkdir(parents=True, exist_ok=True)
    for entry, value in pages:
        with Image.open(workspace / entry["png_file"]) as image:
            review_image(image, value).save(output / f"{entry['page_id']}_review.png")
        print(f"[review] {entry['page_id']}: {len(value['segments'])} ordered labels")


def main():
    parser = argparse.ArgumentParser(description="Stage 02m: independent page-conversion review")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args()
    load_config(args.config, known_stages=PDF_STAGES, required_stages=())
    render_reviews(args.workspace.resolve())


if __name__ == "__main__":
    main()

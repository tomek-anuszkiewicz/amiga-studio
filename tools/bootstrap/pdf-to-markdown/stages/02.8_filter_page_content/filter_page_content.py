#!/usr/bin/env python3
"""Stage 02.8: apply requested source-content exclusions to page objects."""

import argparse
from pathlib import Path
import sys

from PIL import Image, ImageChops, ImageStat

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion.config import load_config, PDF_STAGES
from common.pdf_artifacts import page_files, read_json, write_json

EXCLUDED_PAGE_TYPES = {
    "list_of_tables_heading", "list_of_tables", "list_of_figures_heading",
    "list_of_figures", "index_heading", "index",
}

NXP_TEMPLATE = Path(__file__).resolve().parent / "resources" / "nxp.png"
NXP_REFERENCE_PAGE_SIZE = (2550, 3300)
NXP_REGION = (0.0, 0.0, 0.13, 0.055)
NXP_SIZE_TOLERANCE = 0.20
NXP_ASPECT_TOLERANCE = 0.03
NXP_RGB_TOLERANCE = 0.02


def trim_nxp_margins(image):
    """Keep all pixels with any RGB channel below 245 and their containing box."""
    red, green, blue = image.split()
    darkest = ImageChops.darker(ImageChops.darker(red, green), blue)
    bounds = darkest.point(lambda channel: 255 if channel < 245 else 0).getbbox()
    return image.crop(bounds) if bounds else None


def nxp_geometry_matches(segment, page, template_size):
    if segment["type"] != "graphic":
        return False
    x0, y0, x1, y1 = segment["bbox"]
    width, height = page["image_width"], page["image_height"]
    # Canvas orientation changes page proportions, not the upright logo.
    reference_width, reference_height = NXP_REFERENCE_PAGE_SIZE
    if width > height:
        reference_width, reference_height = reference_height, reference_width
    left, top, right, bottom = NXP_REGION
    if not (left <= x0 / width < x1 / width <= right
            and top <= y0 / height < y1 / height <= bottom):
        return False
    return all(abs(actual / expected - 1) <= NXP_SIZE_TOLERANCE
               for actual, expected in zip(
                   ((x1 - x0) / width, (y1 - y0) / height),
                   (template_size[0] / reference_width,
                    template_size[1] / reference_height)))


def nxp_comparison(crop, reference):
    candidate = trim_nxp_margins(crop.convert("RGB"))
    if candidate is None:
        return None
    aspect_difference = abs((candidate.width / candidate.height)
                            / (reference.width / reference.height) - 1)
    if aspect_difference > NXP_ASPECT_TOLERANCE:
        return None
    resized = candidate.resize(reference.size, Image.Resampling.LANCZOS)
    difference = ImageChops.difference(resized, reference)
    score = sum(ImageStat.Stat(difference).mean) / (3 * 255)
    return (aspect_difference, score) if score <= NXP_RGB_TOLERANCE else None


def filter_page_content(workspace):
    with Image.open(NXP_TEMPLATE) as image:
        template_size = image.size
        reference = trim_nxp_margins(image.convert("RGB"))
    pages = [(path, read_json(path)) for path in
             page_files(workspace / "02_page_conversion", "page_*_segments.json")]
    # Inspect original objects: even an excluded page can define the boundary.
    boundary = next(((page_index, segment_index)
                     for page_index, (_, value) in enumerate(pages)
                     for segment_index, segment in enumerate(value["segments"])
                     if segment["type"] == "toc_heading"), None)
    output = workspace / "02.8_filter_page_content"
    output.mkdir(parents=True, exist_ok=True)
    input_objects = sum(len(value["segments"]) for _, value in pages)
    output_pages = output_objects = nxp_candidates = nxp_matches = 0
    for page_index, (path, value) in enumerate(pages):
        if any(segment["type"] in EXCLUDED_PAGE_TYPES for segment in value["segments"]):
            continue
        segments = [segment for segment_index, segment in enumerate(value["segments"])
                    if segment["type"] not in {"header", "footer"}
                    and (boundary is None or (page_index, segment_index) >= boundary)]
        candidates = [segment for segment in segments
                      if nxp_geometry_matches(segment, value, template_size)]
        nxp_candidates += len(candidates)
        matched_ids = set()
        if candidates:
            page_id = path.stem.removesuffix("_segments")
            with Image.open(workspace / "01_preprocess" / f"{page_id}.png") as image:
                for segment in candidates:
                    with image.crop(tuple(segment["bbox"])) as crop:
                        comparison = nxp_comparison(crop, reference)
                    if comparison is not None:
                        aspect, score = comparison
                        matched_ids.add(segment["segment_id"])
                        nxp_matches += 1
                        print(f"[nxp] {page_id}/{segment['segment_id']}: "
                              f"aspect difference={aspect:.8f}; RGB difference={score:.8f}")
            segments = [segment for segment in segments
                        if segment["segment_id"] not in matched_ids]
        if not segments:
            continue
        write_json(output / path.name, {**value, "segments": segments})
        output_pages += 1
        output_objects += len(segments)
    print(f"[filter] pages {len(pages)} -> {output_pages}; "
          f"objects {input_objects} -> {output_objects}; "
          f"removed pages: {len(pages) - output_pages}; "
          f"TOC boundary found: {boundary is not None}")
    print(f"[nxp] geometry candidates: {nxp_candidates}; matches removed: {nxp_matches}")


def main():
    parser = argparse.ArgumentParser(description="Stage 02.8: filter page content")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args()
    load_config(args.config, known_stages=PDF_STAGES, required_stages=())
    filter_page_content(args.workspace.resolve())


if __name__ == "__main__":
    main()

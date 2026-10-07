#!/usr/bin/env python3
"""Stage 02.5: type-colored annotations and source-mapped prose runs."""

import argparse
from pathlib import Path
import sys

from PIL import Image
sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion.config import load_config, PDF_STAGES
from common.pdf_page_conversion import read_conversion
from common.pdf_artifacts import write_json

from common.pdf_frames import draw_review_image

HIDDEN_TYPES = {"header", "footer", "thumb_index"}

def union_box(boxes):
    return [min(box[0] for box in boxes), min(box[1] for box in boxes),
            max(box[2] for box in boxes), max(box[3] for box in boxes)]


def prose_run_can_extend(segments, start, end):
    """Extend a source-ordered prose run only if its union has no other object."""
    boxes = [segment["bbox"] for segment in segments[start:end+1]]
    left, top, right, bottom = union_box(boxes)
    for index, segment in enumerate(segments):
        if start <= index <= end:
            continue
        x0, y0, x1, y1 = segment["bbox"]
        if left < x1 and x0 < right and top < y1 and y0 < bottom:
            return False
    return True


def review_annotations(segments):
    """Build presentation-only groups without copying or modifying source text."""
    annotations = []
    index = 0
    while index < len(segments):
        segment = segments[index]
        if segment["type"] in HIDDEN_TYPES:
            index += 1
            continue
        end = index
        if segment["type"] == "prose":
            while end+1 < len(segments) and segments[end+1]["type"] == "prose":
                if not prose_run_can_extend(segments, index, end+1):
                    break
                end += 1
        members = segments[index:end+1]
        annotations.append({
            "type": segment["type"], "heading_level": segment.get("heading_level"),
            "bbox": union_box([member["bbox"] for member in members]),
            "continuation": any(member.get("continuation") for member in members),
            "source_ordinals": list(range(index+1, end+2)),
            "source_ids": [member.get("segment_id", member.get("id")) for member in members],
        })
        index = end+1
    return annotations



def review_image(image, value, *, annotations=None):
    width, height = image.size
    raster = {"display_to_pixels": [1, 0, 0, 1, 0, 0],
              "pixel_width": width, "pixel_height": height}
    if annotations is None:
        annotations = review_annotations(value["segments"])
    canvas = draw_review_image(image, {"raster": raster}, annotations,
                               include_continuation=True)
    # Other objects' leaders must not obscure the suppressed source objects.
    for segment in value["segments"]:
        if segment["type"] in HIDDEN_TYPES:
            x0, y0, x1, y1 = segment["bbox"]
            box = (max(0, x0), max(0, y0), min(width, x1), min(height, y1))
            canvas.paste(image.crop(box).convert("RGB"), box[:2])
    return canvas


def render_reviews(workspace, pages=None):
    pages = list(read_conversion(workspace, pages=pages))
    output = workspace / "02.5_page_conversion_review"
    output.mkdir(parents=True, exist_ok=True)
    for entry, value in pages:
        annotations = review_annotations(value["segments"])
        with Image.open(workspace / entry["png_file"]) as image:
            review_image(image, value, annotations=annotations).save(output / f"{entry['page_id']}_review.png")
        write_json(output / f"{entry['page_id']}_review.json", {
            "page": value["page"], "annotations": annotations,
            "hidden_source_ordinals": [index+1 for index, segment in enumerate(value["segments"])
                                       if segment["type"] in HIDDEN_TYPES],
        })
        print(f"[review] {entry['page_id']}: {len(annotations)} labels for {len(value['segments'])} source objects")


def main():
    parser = argparse.ArgumentParser(description="Stage 02.5: independent page-conversion review")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args()
    config = load_config(args.config, known_stages=PDF_STAGES, required_stages=())
    render_reviews(args.workspace.resolve(), pages=config.get("input", {}).get("pages"))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Stage 02.5: type-colored annotations and source-mapped prose runs."""

import argparse
import math
from pathlib import Path
import sys

from PIL import Image, ImageDraw, ImageFont
sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion.config import load_config, PDF_STAGES
from common.pdf_page_conversion import read_conversion
from common.pdf_artifacts import write_json

FRAME_WIDTH = 2
HIDDEN_TYPES = {"header", "footer", "thumb_index"}
# Explicit role keys keep colors independent of page order and schema additions.
TYPE_COLORS = {
    "cover": "#775132", "header": "#646464", "footer": "#929292",
    "toc_heading": "#582d90", "toc": "#8471ba", "thumb_index": "#9e8165",
    "chapter": "#a62a43", "heading": "#73203d", "callout": "#ad5b00",
    "callout_text": "#8c7500", "prose": "#1764a0", "code_block": "#008484",
    "table": "#23783a", "graphic": "#cc4d15", "caption": "#a62d87",
    "footnote": "#5b6130", "table_legend": "#357161", "index_heading": "#3b3b88",
    "index": "#6262aa", "list_of_tables_heading": "#487317", "list_of_tables": "#73852f",
    "list_of_figures_heading": "#98442e", "list_of_figures": "#b57542",
}


def union_box(boxes):
    return [min(box[0] for box in boxes), min(box[1] for box in boxes),
            max(box[2] for box in boxes), max(box[3] for box in boxes)]


def prose_run_can_extend(segments, start, end):
    """Require one nearby downward flow and a union clear of all other objects."""
    boxes = [segment["bbox"] for segment in segments[start:end+1]]
    previous, candidate = boxes[-2:]
    gap = candidate[1] - previous[3]
    # Conservative adjacency: no vertical overlap or gaps taller than either
    # neighbor, and a common column span covering 80% of the widest run member.
    if gap < 0 or gap > min(previous[3]-previous[1], candidate[3]-candidate[1]):
        return False
    shared_width = min(box[2] for box in boxes) - max(box[0] for box in boxes)
    if shared_width < 0.8 * max(box[2]-box[0] for box in boxes):
        return False
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
            "source_ids": [member.get("id") for member in members],
        })
        index = end+1
    return annotations


def frame_box(segment, raster):
    """Map object bounds through the raster transform, rounding out."""
    box = segment.get("bbox")
    x0, y0, x1, y1 = box
    a, b, c, d, e, f = raster["display_to_pixels"]
    corners = [(a*x + c*y + e, b*x + d*y + f) for x, y in ((x0, y0), (x1, y0), (x0, y1), (x1, y1))]
    # Pillow draws the stroke inside its rectangle. Move the outer edge out
    # by the full stroke width so the source bbox stays inside or on the frame.
    left = math.floor(min(x for x, y in corners)) - FRAME_WIDTH
    top = math.floor(min(y for x, y in corners)) - FRAME_WIDTH
    right = math.ceil(max(x for x, y in corners)) + FRAME_WIDTH - 1
    bottom = math.ceil(max(y for x, y in corners)) + FRAME_WIDTH - 1
    width, height = raster["pixel_width"], raster["pixel_height"]
    return (max(0, min(width-1, left)), max(0, min(height-1, top)),
            max(0, min(width-1, right)), max(0, min(height-1, bottom)))


def draw_review_image(image, page_data, segments, *, include_continuation=False):
    width, height = image.size
    canvas = Image.new("RGB", (width*2, height), "white")
    canvas.paste(image.convert("RGB"), (0, 0))
    draw = ImageDraw.Draw(canvas)
    count = len(segments)
    margin = max(12, width // 40)
    row_height = (height - 2*margin) / max(1, count)
    font_size = max(1, min(max(14, width // 65), int(row_height * 0.6)))
    font = ImageFont.load_default(size=font_size)
    label_x = width + width//3
    leader_x = width + margin//2
    frames = [frame_box(segment, page_data["raster"]) for segment in segments]
    for index, segment in enumerate(segments):
        kind = segment.get("type")
        level = segment.get("heading_level")
        ordinals = segment.get("source_ordinals", [index+1])
        ordinal_label = str(ordinals[0]) if len(ordinals) == 1 else f"{ordinals[0]}-{ordinals[-1]}"
        label = f"{ordinal_label}. {kind}"
        if level is not None:
            label += f" | heading_level: {level}"
        if include_continuation:
            continuation = segment.get("continuation")
            if continuation:
                label += " | continuation: true"
        color = TYPE_COLORS.get(kind, "#444444")
        frame = frames[index]
        left, top, right, bottom = frame
        label_y = round(margin + (index+0.5)*row_height)
        anchor_y = (top+bottom) // 2
        # All leaders enter the panel at the same x. Straight links between
        # these ports and JSON-ordered labels expose vertical order inversions.
        draw.line([(right, anchor_y), (leader_x, anchor_y), (label_x-margin//2, label_y)],
                  fill=color, width=FRAME_WIDTH)
        draw.rectangle((left, top, right, bottom), outline=color, width=FRAME_WIDTH)
        draw.text((label_x, label_y), label, font=font, fill=color, anchor="lm")
    return canvas


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


def render_reviews(workspace):
    pages = list(read_conversion(workspace))
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
    load_config(args.config, known_stages=PDF_STAGES, required_stages=())
    render_reviews(args.workspace.resolve())


if __name__ == "__main__":
    main()

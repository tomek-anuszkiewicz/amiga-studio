#!/usr/bin/env python3
"""Stage 02.5: ordered labels and model-selected boxes for every object."""

import argparse
import colorsys
import math
from pathlib import Path
import sys

from PIL import Image, ImageDraw, ImageFont
sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion.config import load_config, PDF_STAGES
from common.pdf_page_conversion import validate_conversion
from common.pdf_artifacts import require

FRAME_WIDTH = 2


def frame_box(segment, raster):
    """Map object bounds through the raster transform, rounding out."""
    box = segment.get("bbox")
    require(isinstance(box, list) and len(box) == 4
            and all(not isinstance(v, bool) and isinstance(v, (int, float)) and math.isfinite(v) for v in box),
            "Segment bbox must contain four finite coordinates")
    x0, y0, x1, y1 = box
    require(x0 <= x1 and y0 <= y1, "Segment bbox edges are reversed")
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
        require(isinstance(kind, str) and kind, "Segment type is missing")
        require(level is None or type(level) is int and level > 0, "Invalid segment heading_level")
        label = f"{index+1}. {kind}"
        if level is not None:
            label += f" | heading_level: {level}"
        if include_continuation:
            continuation = segment.get("continuation")
            require(type(continuation) is bool, "Invalid segment continuation")
            if continuation:
                label += " | continuation: true"
        require(draw.textlength(label, font=font) <= width*2 - label_x - margin,
                "Segment label does not fit the review panel")
        color = tuple(round(channel*255) for channel in colorsys.hsv_to_rgb((index*0.61803398875) % 1, 0.8, 0.65))
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



def review_image(image, value):
    width, height = image.size
    raster = {"display_to_pixels": [1, 0, 0, 1, 0, 0],
              "pixel_width": width, "pixel_height": height}
    return draw_review_image(image, {"raster": raster}, value["segments"],
                               include_continuation=True)


def render_reviews(workspace):
    pages = validate_conversion(workspace)
    output = workspace / "02.5_page_conversion_review"
    output.mkdir(parents=True, exist_ok=True)
    for entry, value in pages:
        with Image.open(workspace / entry["png_file"]) as image:
            review_image(image, value).save(output / f"{entry['page_id']}_review.png")
        print(f"[review] {entry['page_id']}: {len(value['segments'])} ordered labels")


def main():
    parser = argparse.ArgumentParser(description="Stage 02.5: independent page-conversion review")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args()
    load_config(args.config, known_stages=PDF_STAGES, required_stages=())
    render_reviews(args.workspace.resolve())


if __name__ == "__main__":
    main()

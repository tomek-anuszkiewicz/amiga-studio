#!/usr/bin/env python3
"""Stage 02m: ordered labels, with geometry only for tables and graphics."""

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
    canvas = Image.new("RGB", (width*2, height), "white")
    canvas.paste(image.convert("RGB"), (0, 0))
    draw = review.ImageDraw.Draw(canvas)
    segments = value["segments"]
    margin = max(12, width // 40)
    row_height = (height - 2*margin) / max(1, len(segments))
    font_size = max(1, min(max(14, width // 65), int(row_height * 0.6)))
    font = review.ImageFont.load_default(size=font_size)
    label_x, leader_x = width + width//3, width + margin//2
    for index, segment in enumerate(segments):
        label = f"{index+1}. {segment['type']}"
        if segment["heading_level"] is not None:
            label += f" | heading_level: {segment['heading_level']}"
        review.require(draw.textlength(label, font=font) <= width*2-label_x-margin,
                       "Object label does not fit the review panel")
        color = tuple(round(channel*255) for channel in review.colorsys.hsv_to_rgb(
            (index*0.61803398875) % 1, 0.8, 0.65))
        label_y = round(margin + (index+0.5)*row_height)
        if segment["bbox"] is not None:
            raster = {"display_to_pixels": [1, 0, 0, 1, 0, 0],
                      "pixel_width": width, "pixel_height": height}
            left, top, right, bottom = review.frame_box(segment, raster)
            anchor_y = (top+bottom)//2
            draw.line([(right, anchor_y), (leader_x, anchor_y), (label_x-margin//2, label_y)],
                      fill=color, width=review.FRAME_WIDTH)
            draw.rectangle((left, top, right, bottom), outline=color, width=review.FRAME_WIDTH)
        draw.text((label_x, label_y), label, font=font, fill=color, anchor="lm")
    return canvas


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

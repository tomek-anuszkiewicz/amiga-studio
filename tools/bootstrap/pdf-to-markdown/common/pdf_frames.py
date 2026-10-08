"""Shared original-scale page outlines and side legends."""

import math
from PIL import Image, ImageDraw, ImageFont

FRAME_WIDTH = 2
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


def draw_review_image(image, page_data, segments, *, include_continuation=False, include_segment_ids=False):
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
        if include_segment_ids:
            label += f" | {segment['segment_id']}"
        if level is not None:
            label += f" | heading_level: {level}"
        if "rotation" in segment:
            label += f" | rotation: {segment['rotation']} deg CW"
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

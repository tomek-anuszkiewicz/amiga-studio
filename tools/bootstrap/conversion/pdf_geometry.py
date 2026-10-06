"""Displayed-page text coordinates shared by preparation and preprocessing."""

import math
import pymupdf

SCHEMA_VERSION = 1
# MuPDF stores text geometry as single-precision floats; serialized PDF text
# matrices introduce small rounding differences. This is < 0.1 pixel at 300 DPI.
GEOMETRY_TOLERANCE = 0.02
COORDINATES = {"origin": "top-left", "x_axis": "right", "y_axis": "down",
               "units": "PDF points", "bbox": "[x0, y0, x1, y1]", "frame": "displayed-page"}


def page_geometry(page):
    return {"width": page.rect.width, "height": page.rect.height,
            "rotation": page.rotation, "mediabox": list(page.mediabox),
            "cropbox": list(page.cropbox), "cropbox_position": list(page.cropbox_position),
            "extraction_to_display": list(page.rotation_matrix),
            "pdf_to_extraction": list(page.transformation_matrix)}


def valid_box(box, width, height, *, tolerance=GEOMETRY_TOLERANCE, clip=False):
    if not isinstance(box, (list, tuple)) or len(box) != 4:
        raise ValueError("Text bounding box must contain four coordinates")
    if any(isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v) for v in box):
        raise ValueError("Text bounding box must be finite")
    x0, y0, x1, y1 = box
    if x0 > x1 or y0 > y1:
        raise ValueError(f"Invalid or out-of-page text bounding box: {box}")
    if clip:
        x0, x1 = min(width, max(0, x0)), min(width, max(0, x1))
        y0, y1 = min(height, max(0, y0)), min(height, max(0, y1))
        box = [x0, y0, x1, y1]
    reversed_edges = x0 > x1 or y0 > y1
    zero_extent = x0 == x1 or y0 == y1
    if reversed_edges or zero_extent or x0 < -tolerance or y0 < -tolerance or x1 > width + tolerance or y1 > height + tolerance:
        raise ValueError(f"Invalid or out-of-page text bounding box: {box}")
    return list(box)


def valid_text(text):
    if not isinstance(text, str) or not text.strip():
        raise ValueError("Empty text in a text-bearing block")
    if "\ufffd" in text or any(ord(c) < 32 and c not in "\n\r\t" for c in text):
        raise ValueError("Unresolved defective text layer or unsupported OCR text")
    return text


def text_blocks(page):
    """Inspect actual text spans, excluding image blocks; retain extraction order."""
    blocks = []
    flags = pymupdf.TEXTFLAGS_DICT & ~pymupdf.TEXT_PRESERVE_IMAGES
    for block in page.get_text("dict", flags=flags, sort=False)["blocks"]:
        if block["type"] != 0:
            continue
        lines = ["".join(span["text"] for span in line["spans"]) for line in block["lines"]]
        text = "\n".join(lines)
        if not text.strip():
            continue
        valid_text(text)
        box = list(pymupdf.Rect(block["bbox"]) * page.rotation_matrix)
        box = valid_box(box, page.rect.width, page.rect.height, clip=True)
        blocks.append({"block_id": block["number"], "type": 0, "text": text + "\n", "bbox": box,
                       "bbox_norm": [box[0] / page.rect.width, box[1] / page.rect.height,
                                     box[2] / page.rect.width, box[3] / page.rect.height]})
    return blocks


def raster_transform(page, pixmap, dpi):
    # get_pixmap rounds the transformed page bounds outward to integer pixels.
    # Preserve this actual origin/dimensions, rather than scaling by width ratios.
    scale = dpi / 72
    matrix = pymupdf.Matrix(scale, scale)
    bounds = (page.rect * matrix).irect
    if (pixmap.x, pixmap.y, pixmap.width, pixmap.height) != (bounds.x0, bounds.y0, bounds.width, bounds.height):
        raise ValueError("Rendered page does not match its recorded transform")
    return {"pixel_width": pixmap.width, "pixel_height": pixmap.height,
            "raster_origin": [pixmap.x, pixmap.y],
            "display_to_pixels": [scale, 0, 0, scale, -pixmap.x, -pixmap.y],
            "pixel_bounds_rounding": "outward floor/ceil", "export_rounding": "none"}

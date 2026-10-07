"""Displayed-page text coordinates shared by preparation and preprocessing."""

import pymupdf

SCHEMA_VERSION = 1
COORDINATES = {"origin": "top-left", "x_axis": "right", "y_axis": "down",
               "units": "PDF points", "bbox": "[x0, y0, x1, y1]", "frame": "displayed-page"}


def page_geometry(page):
    return {"width": page.rect.width, "height": page.rect.height,
            "rotation": page.rotation, "mediabox": list(page.mediabox),
            "cropbox": list(page.cropbox), "cropbox_position": list(page.cropbox_position),
            "extraction_to_display": list(page.rotation_matrix),
            "pdf_to_extraction": list(page.transformation_matrix)}


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
        box = list(pymupdf.Rect(block["bbox"]) * page.rotation_matrix)
        box = [min(page.rect.width, max(0, box[0])), min(page.rect.height, max(0, box[1])),
               min(page.rect.width, max(0, box[2])), min(page.rect.height, max(0, box[3]))]
        blocks.append({"block_id": block["number"], "type": 0, "text": text + "\n", "bbox": box,
                       "bbox_norm": [box[0] / page.rect.width, box[1] / page.rect.height,
                                     box[2] / page.rect.width, box[3] / page.rect.height]})
    return blocks


def raster_transform(page, pixmap, dpi):
    # get_pixmap rounds the transformed page bounds outward to integer pixels.
    # Preserve this actual origin/dimensions, rather than scaling by width ratios.
    scale = dpi / 72
    return {"pixel_width": pixmap.width, "pixel_height": pixmap.height,
            "raster_origin": [pixmap.x, pixmap.y],
            "display_to_pixels": [scale, 0, 0, scale, -pixmap.x, -pixmap.y],
            "pixel_bounds_rounding": "outward floor/ceil", "export_rounding": "none"}

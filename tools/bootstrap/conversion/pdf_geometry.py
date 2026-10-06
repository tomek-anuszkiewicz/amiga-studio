"""Displayed-page text coordinates shared by preparation and preprocessing."""

import math
import re
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


def valid_box(box, width, height, *, tolerance=GEOMETRY_TOLERANCE, clip=False, allow_zero=False):
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
    if reversed_edges or (zero_extent and not allow_zero) or x0 < -tolerance or y0 < -tolerance or x1 > width + tolerance or y1 > height + tolerance:
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


def validate_ocr(response):
    from jsonschema import validate
    from .pdf_schemas import OCR
    validate(response, OCR)
    kind, blocks = response["page_type"], response["blocks"]
    if kind in ("blank", "pure_graphic"):
        if blocks:
            raise ValueError("Blank/graphic-only classification cannot contain OCR text")
    elif not blocks:
        raise ValueError("Text-bearing page has unresolved empty OCR")
    for block in blocks:
        valid_text(block["text"])
        y0, x0, y1, x1 = block["box_2d"]
        # Integer normalization can collapse a valid sub-unit OCR extent.
        valid_box([x0, y0, x1, y1], 1000, 1000, tolerance=0, allow_zero=True)
    return response


def embedded_metrics(document, xref, font, text):
    """Use serialized CID widths/descriptor metrics, not pre-serialization TTF floats."""
    descendants = document.xref_get_key(xref, "DescendantFonts")[1]
    cid = int(re.search(r"(\d+)\s+0\s+R", descendants).group(1))
    descriptor = int(document.xref_get_key(cid, "FontDescriptor")[1].split()[0])
    ascent = float(document.xref_get_key(descriptor, "Ascent")[1]) / 1000
    descent = float(document.xref_get_key(descriptor, "Descent")[1]) / 1000
    kind, widths = document.xref_get_key(cid, "W")
    if kind == "xref":
        widths = document.xref_object(int(widths.split()[0]))
    tokens = re.findall(r"-?\d+(?:\.\d+)?|\[|\]", widths)[1:-1]
    needed = {font.has_glyph(ord(c), fallback=False) for c in text}
    default = document.xref_get_key(cid, "DW")[1]
    advances = dict.fromkeys(needed, float(default) if default != "null" else 1000)
    index = 0
    while index < len(tokens):
        first = int(tokens[index])
        index += 1
        if tokens[index] == "[":
            index += 1
            glyph = first
            while tokens[index] != "]":
                if glyph in needed:
                    advances[glyph] = float(tokens[index])
                glyph += 1
                index += 1
            index += 1
        else:
            last, width = int(tokens[index]), float(tokens[index + 1])
            index += 2
            for glyph in needed:
                if first <= glyph <= last:
                    advances[glyph] = width
    length = sum(advances[font.has_glyph(ord(c), fallback=False)] for c in text) / 1000
    if length <= 0 or ascent <= descent:
        raise ValueError("Unsupported embedded font metrics")
    return length, ascent, descent


def insert_ocr(page, response):
    """Place invisible Unicode lines through explicit display->extraction maps.

    Each nonempty OCR line occupies an equal-height slice of its supplied box.
    Its font metrics determine an explicit affine fit; no text is truncated.
    Trust the PDF writer to store the requested text and placement.
    """
    fonts = [pymupdf.Font("helv"), pymupdf.Font("cjk"), pymupdf.Font(script=0)]
    for block in validate_ocr(response)["blocks"]:
        characters = set(block["text"]) - set("\r\n\t")
        choice = next((i for i, font in enumerate(fonts)
                       if all(font.has_glyph(ord(c), fallback=False) for c in characters)), None)
        if choice is None:
            raise ValueError("OCR block contains characters unsupported by the embedded Unicode fonts")
        font = fonts[choice]
        fontname = f"OCRUnicode{choice}"
        xref = page.insert_font(fontname=fontname, fontbuffer=font.buffer)
        y0, x0, y1, x1 = block["box_2d"]
        box = pymupdf.Rect(x0 * page.rect.width / 1000, y0 * page.rect.height / 1000,
                           x1 * page.rect.width / 1000, y1 * page.rect.height / 1000)
        lines = block["text"].replace("\r\n", "\n").replace("\r", "\n").split("\n")
        line_height = box.height / len(lines)
        for index, line in enumerate(lines):
            line = line.replace("\t", "    ")
            if not line.strip():
                continue
            width, ascent, descent = embedded_metrics(page.parent, xref, font, line)
            sx, sy = box.width / width, line_height / (ascent - descent)
            origin = pymupdf.Point(box.x0, box.y0 + index * line_height + ascent * sy)
            point = origin * page.derotation_matrix
            inverse = page.derotation_matrix
            linear = pymupdf.Matrix(inverse.a, inverse.b, inverse.c, inverse.d, 0, 0)
            matrix = pymupdf.Matrix(sx, sy) * linear
            # Shape morphs act in PDF's y-up frame, unlike extraction coordinates.
            matrix = pymupdf.Matrix(matrix.a, -matrix.b, -matrix.c, matrix.d, 0, 0)
            shape = page.new_shape()
            shape.insert_text(point, line, fontsize=1, fontname=fontname, render_mode=3,
                              morph=(point, matrix))
            shape.commit()


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

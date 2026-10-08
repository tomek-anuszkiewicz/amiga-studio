#!/usr/bin/env python3
"""Prepare a shared full-document PDF once, adding OCR where text is missing."""

import argparse
from pathlib import Path
import sys
import pymupdf

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from common.pdf_runtime import RENDER_DPI
from common.pdf_config import load_config
from common.pdf_artifacts import prepared_pdf_path
from common.pdf_geometry import text_blocks


_OCR_LANGUAGE = "eng"


def tesseract_settings():
    """Resolve the language and local Tesseract data directory."""
    return _OCR_LANGUAGE, pymupdf.get_tessdata()


def tesseract_ocr(pixmap, language, tessdata):
    """Keep the complete PDF emitted by Tesseract without rebuilding its text."""
    return pixmap.pdfocr_tobytes(language=language, tessdata=tessdata)


def insert_ocr_pdf(page, ocr):
    """Overlay original OCR text operators and fonts at their original scale."""
    layer = ocr[0]
    # The source page already owns its graphics. Make the OCR raster transparent
    # while leaving all text operators, glyph spacing and font resources intact.
    for image in layer.get_images():
        layer.delete_image(image[0])
    target = layer.rect * page.derotation_matrix
    page.show_pdf_page(target, ocr, 0, rotate=page.rotation)


def prepare_text_layer(pdf_path, workspace, config, page_ranges=None):
    pdf_path, workspace = Path(pdf_path).resolve(), Path(workspace).resolve()
    stage_dir = workspace / "01_text_layer"
    dpi = RENDER_DPI
    with pymupdf.open(pdf_path) as document:
        pages = list(range(1, len(document) + 1))
        stage_dir.mkdir(parents=True, exist_ok=True)
        output = prepared_pdf_path(pdf_path)
        if output.is_file():
            print(f"[skip] Stage 01: {output.name} already exists; no OCR or PDF rewrite")
            return output
        candidate = output.with_suffix(".candidate.pdf")
        candidate.unlink(missing_ok=True)
        engine = None
        try:
            for prepared_index, number in enumerate(pages):
                page = document[prepared_index]
                page_id = f"page_{number:04d}"
                if text_blocks(page):
                    print(f"[native] {page_id}: retained existing text spans")
                else:
                    pixmap = page.get_pixmap(dpi=dpi, colorspace=pymupdf.csRGB, alpha=False)
                    if engine is None:
                        engine = tesseract_settings()
                    language, tessdata = engine
                    payload = tesseract_ocr(pixmap, language, tessdata)
                    with pymupdf.open("pdf", payload) as ocr:
                        has_text = bool(ocr[0].get_text().strip())
                        if has_text:
                            insert_ocr_pdf(page, ocr)
                    provenance = "ocr" if has_text else "none"
                    page_type = "text_page" if has_text else "pure_graphic"
                    print(f"[{provenance}] {page_id}: {page_type}")
            document.save(candidate, deflate=True, garbage=4)
            candidate.replace(output)
        except BaseException:
            candidate.unlink(missing_ok=True)
            raise
    print(f"[+] Stage 01 published {output.name}; source pages: {pages}")
    return output


def main():
    parser = argparse.ArgumentParser(description="Stage 01: prepare a full-source OCR PDF unless it already exists")
    parser.add_argument("--pdf", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    config = load_config(args.config)
    prepare_text_layer(args.pdf, args.workspace, config, args.page_ranges)


if __name__ == "__main__":
    main()

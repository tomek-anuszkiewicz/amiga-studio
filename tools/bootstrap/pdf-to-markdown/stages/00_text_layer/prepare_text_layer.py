#!/usr/bin/env python3
"""Prepare a shared full-document PDF once, adding OCR where text is missing."""

import argparse
import hashlib
from pathlib import Path
import sys
import pymupdf

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion.config import load_config, PDF_STAGES
from common.pdf_artifacts import write_json, read_json, prepared_pdf_path
from common.pdf_geometry import page_geometry, text_blocks


def file_hash(path):
    """Identify compatible OCR recovery requests."""
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def tesseract_settings(config):
    """Resolve language data and fingerprint it without recording host paths."""
    settings = config.get("ocr", {})
    language = settings.get("language", "eng")
    tessdata = Path(pymupdf.get_tessdata(settings.get("tessdata")))
    files = {code: tessdata / f"{code}.traineddata" for code in language.split("+")}
    identity = {"engine": "tesseract", "language": language, "pymupdf": pymupdf.__version__,
                "traineddata": {code: file_hash(path) for code, path in files.items()}}
    return language, str(tessdata), identity


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
    stage_dir = workspace / "00_text_layer"
    source = {"name": pdf_path.name}
    dpi = config.get("render", {}).get("dpi", 300)
    with pymupdf.open(pdf_path) as document:
        pages = list(range(1, len(document) + 1))
        stage_dir.mkdir(parents=True, exist_ok=True)
        output = prepared_pdf_path(pdf_path)
        if output.is_file():
            print(f"[skip] Stage 00: {output.name} already exists; no OCR or PDF rewrite")
            return output
        candidate = output.with_suffix(".candidate.pdf")
        candidate.unlink(missing_ok=True)
        recovery = stage_dir / "recovery"
        recovery.mkdir(exist_ok=True)
        engine = None
        procedure = {"script_sha256": file_hash(__file__),
                     "settings": {key: value for key, value in config.items() if key != "llm"}}
        recovery_source = {**source, "sha256": file_hash(pdf_path)}
        try:
            for prepared_index, number in enumerate(pages):
                page = document[prepared_index]
                entry = {"page": number, "page_id": f"page_{number:04d}", "source_index": number - 1,
                         "prepared_index": prepared_index,
                         "geometry": page_geometry(page)}
                if text_blocks(page):
                    entry.update(provenance="native", page_type="text_page", classification_basis="inferred_from_native_spans")
                    print(f"[native] {entry['page_id']}: retained existing text spans")
                else:
                    image = stage_dir / ".request.png"
                    pixmap = page.get_pixmap(dpi=dpi, colorspace=pymupdf.csRGB, alpha=False)
                    pixmap.save(image)
                    if engine is None:
                        engine = tesseract_settings(config)
                    language, tessdata, engine_identity = engine
                    identity = {"source": recovery_source, "page": number, "image_sha256": file_hash(image),
                                "geometry": entry["geometry"], "format": "tesseract-pdf-v1", "procedure": procedure,
                                "engine": engine_identity}
                    recovery_file = recovery / f"{entry['page_id']}.json"
                    recovery_pdf = recovery / f"{entry['page_id']}.pdf"
                    payload = None
                    if recovery_file.is_file():
                        retained = read_json(recovery_file)
                        if retained.get("identity") == identity and recovery_pdf.is_file():
                            payload = recovery_pdf.read_bytes()
                            print(f"[recovery] {entry['page_id']}: compatible original OCR PDF")
                    fresh_ocr = payload is None
                    if payload is None:
                        payload = tesseract_ocr(pixmap, language, tessdata)
                    image.unlink()
                    with pymupdf.open("pdf", payload) as ocr:
                        has_text = bool(ocr[0].get_text().strip())
                        entry.update(page_type="text_page" if has_text else "pure_graphic", caption=None,
                                     classification_basis="inferred_from_tesseract_lines",
                                     provenance="ocr" if has_text else "none")
                        if has_text:
                            insert_ocr_pdf(page, ocr)
                    if fresh_ocr:
                        recovery_pdf.write_bytes(payload)
                        write_json(recovery_file, {"identity": identity})
                    print(f"[{entry['provenance']}] {entry['page_id']}: {entry['page_type']}")
            document.save(candidate, deflate=True, garbage=4)
            candidate.replace(output)
        except BaseException:
            candidate.unlink(missing_ok=True)
            raise
        finally:
            (stage_dir / ".request.png").unlink(missing_ok=True)
    print(f"[+] Stage 00 published {output.name}; source pages: {pages}")
    return output


def main():
    parser = argparse.ArgumentParser(description="Stage 00: prepare a full-source OCR PDF unless it already exists")
    parser.add_argument("--pdf", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    config = load_config(args.config, known_stages=PDF_STAGES, required_stages=())
    prepare_text_layer(args.pdf, args.workspace, config, args.page_ranges)


if __name__ == "__main__":
    main()

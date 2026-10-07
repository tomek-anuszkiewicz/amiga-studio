#!/usr/bin/env python3
"""Prepare a shared full-document PDF once, adding OCR where text is missing."""

import argparse
import os
from pathlib import Path
import sys
import pymupdf

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion.config import load_config, PDF_STAGES
from common.lineage import file_hash, read_state, stage_identity
from common.pdf_artifacts import write_json, read_json, require
from common.pdf_geometry import (SCHEMA_VERSION, page_geometry,
                                     text_blocks)
from common.pdf_selection import selected_pages


def tesseract_settings(config):
    """Resolve language data and fingerprint it without recording host paths."""
    settings = config.get("ocr", {})
    require(isinstance(settings, dict) and not set(settings) - {"language", "tessdata"},
            "ocr supports only language and tessdata")
    language = settings.get("language", "eng")
    require(isinstance(language, str) and language and all(
        code and all(c.isalnum() or c == "_" for c in code) for code in language.split("+")),
        "ocr.language must contain Tesseract language identifiers")
    tessdata = Path(pymupdf.get_tessdata(settings.get("tessdata")))
    files = {code: tessdata / f"{code}.traineddata" for code in language.split("+")}
    require(all(path.is_file() for path in files.values()),
            "Missing Tesseract language data; configure ocr.tessdata or TESSDATA_PREFIX")
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
    require(not pdf_path.is_relative_to(stage_dir), "Original PDF cannot live inside Stage 00 outputs")
    source_hash = file_hash(pdf_path)
    source = {"name": pdf_path.name, "sha256": source_hash, "pages": None}
    dpi = config.get("render", {}).get("dpi", 300)
    require(type(dpi) is int and dpi > 0, "render.dpi must be a positive integer")
    with pymupdf.open(pdf_path) as document:
        require(document.is_pdf and not document.needs_pass, "Only readable, unencrypted PDFs are supported")
        selection = selected_pages(page_ranges, len(document))
        pages = list(range(1, len(document) + 1))
        source_page_count = len(document)
        source["pages"] = selection if page_ranges is not None else None
        state_source = read_state(workspace).get("source")
        require(state_source is None or state_source == source, "Stage 00 source does not match workspace selection")
        stage_dir.mkdir(parents=True, exist_ok=True)
        manifest_path = stage_dir / "text_layer_manifest.json"
        manifest_path.unlink(missing_ok=True)
        output = pdf_path.with_stem(f"{pdf_path.stem}-ocr")
        procedure = stage_identity({"id": "00", "dir": "00_text_layer"}, config, source, None, Path(__file__).resolve().parents[2])
        manifest = {"schema_version": SCHEMA_VERSION, "source": source, "source_page_count": source_page_count,
                    "selected_pages": pages, "source_file": Path(os.path.relpath(pdf_path, workspace)).as_posix(),
                    "pdf_file": Path(os.path.relpath(output, workspace)).as_posix(),
                    "pages": [{"page": number, "page_id": f"page_{number:04d}",
                               "source_index": number - 1, "prepared_index": number - 1} for number in pages],
                    "ocr_procedure": procedure}
        if output.is_file():
            write_json(manifest_path, manifest)
            print(f"[skip] Stage 00: {output.name} already exists; no OCR or PDF rewrite")
            return manifest
        candidate = output.with_suffix(".candidate.pdf")
        candidate.unlink(missing_ok=True)
        recovery = stage_dir / "recovery"
        recovery.mkdir(exist_ok=True)
        engine = None
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
                    identity = {"source": source, "page": number, "image_sha256": file_hash(image),
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
            write_json(manifest_path, manifest)
        except BaseException:
            manifest_path.unlink(missing_ok=True)
            candidate.unlink(missing_ok=True)
            raise
        finally:
            (stage_dir / ".request.png").unlink(missing_ok=True)
    print(f"[+] Stage 00 published {output.name}; source pages: {pages}")
    return manifest


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

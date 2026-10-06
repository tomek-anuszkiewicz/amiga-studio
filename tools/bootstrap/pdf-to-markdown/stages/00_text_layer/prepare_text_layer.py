#!/usr/bin/env python3
"""Prepare a separate validated PDF; publish no manifest on incomplete OCR."""

import argparse
from pathlib import Path
import shutil
import sys
import pymupdf

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion.cache import digest
from conversion.config import load_config, PDF_STAGES
from common.lineage import file_hash, read_state, stage_identity
from common.pdf_artifacts import write_json, read_json, require, validate_text_layer
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


def verify_candidate(source_path, candidate, entries, dpi):
    selected = {entry["page"]: entry for entry in entries}
    with pymupdf.open(source_path) as original, pymupdf.open(candidate) as prepared:
        require(len(original) == len(prepared), "Preparation changed the source page tree")
        for number in range(len(original)):
            before, after = original[number], prepared[number]
            require(page_geometry(before) == page_geometry(after), "Preparation changed page geometry")
            old_streams = [original.xref_stream(xref) for xref in before.get_contents()]
            new_streams = [prepared.xref_stream(xref) for xref in after.get_contents()]
            # PyMuPDF adds q/Q wrappers around unwrapped source content. Require
            # byte-identical source streams in their original order, allowing
            # those wrappers and the appended invisible text streams.
            cursor = 0
            for stream in old_streams:
                match = next((i for i in range(cursor, len(new_streams)) if new_streams[i] == stream), None)
                require(match is not None, "Preparation changed source content streams")
                cursor = match + 1
            entry = selected.get(number + 1)
            if not entry or entry["provenance"] != "ocr":
                require(before.get_text("rawdict") == after.get_text("rawdict"), "Preparation changed retained native content")
                require(old_streams == new_streams, "Preparation changed a page outside OCR coverage")
            if entry:
                left, right = before.get_pixmap(dpi=dpi), after.get_pixmap(dpi=dpi)
                require((left.x, left.y, left.width, left.height, left.n, left.samples)
                        == (right.x, right.y, right.width, right.height, right.n, right.samples),
                        f"Preparation changed visible content on page {number + 1}")
                entry["text_digest"] = digest(text_blocks(after))


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
        pages = selected_pages(page_ranges, len(document))
        source["pages"] = pages if page_ranges is not None else None
        state_source = read_state(workspace).get("source")
        require(state_source is None or state_source == source, "Stage 00 source does not match workspace selection")
        stage_dir.mkdir(parents=True, exist_ok=True)
        manifest_path = stage_dir / "text_layer_manifest.json"
        manifest_path.unlink(missing_ok=True)
        output = stage_dir / f"{pdf_path.stem}-ocr.pdf"
        candidate = stage_dir / ".candidate.pdf"
        candidate.unlink(missing_ok=True)
        recovery = stage_dir / "recovery"
        recovery.mkdir(exist_ok=True)
        procedure = stage_identity({"id": "00", "dir": "00_text_layer"}, config, source, None, Path(__file__).resolve().parents[2])
        entries, engine, added_text = [], None, False
        try:
            for number in pages:
                page = document[number - 1]
                entry = {"page": number, "page_id": f"page_{number:04d}", "source_index": number - 1,
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
                        require(retained.get("digest") == digest({k: v for k, v in retained.items() if k != "digest"}),
                                "Modified per-page OCR recovery record")
                        if retained.get("identity") == identity and recovery_pdf.is_file():
                            require(file_hash(recovery_pdf) == retained.get("pdf_sha256"),
                                    "Modified per-page OCR PDF recovery artifact")
                            payload = recovery_pdf.read_bytes()
                            print(f"[recovery] {entry['page_id']}: compatible original OCR PDF")
                    if payload is None:
                        payload = tesseract_ocr(pixmap, language, tessdata)
                        # Check only that the returned artifact can be opened.
                        with pymupdf.open("pdf", payload) as recovered:
                            require(len(recovered) == 1, "Tesseract must return one OCR PDF page")
                        recovery_pdf.write_bytes(payload)
                        record = {"identity": identity, "pdf_sha256": file_hash(recovery_pdf)}
                        record["digest"] = digest(record)
                        write_json(recovery_file, record)
                    image.unlink()
                    with pymupdf.open("pdf", payload) as ocr:
                        has_text = bool(ocr[0].get_text().strip())
                        entry.update(page_type="text_page" if has_text else "pure_graphic", caption=None,
                                     classification_basis="inferred_from_tesseract_lines",
                                     provenance="ocr" if has_text else "none")
                        if has_text:
                            insert_ocr_pdf(page, ocr)
                            added_text = True
                    print(f"[{entry['provenance']}] {entry['page_id']}: {entry['page_type']}")
                entries.append(entry)
            if added_text:
                document.save(candidate, deflate=True)
            else:
                shutil.copyfile(pdf_path, candidate)
            verify_candidate(pdf_path, candidate, entries, dpi)
            require(file_hash(pdf_path) == source_hash, "Source PDF changed during preparation")
            manifest = {"schema_version": SCHEMA_VERSION, "source": source, "source_page_count": len(document),
                        "selected_pages": pages, "pdf_file": output.relative_to(workspace).as_posix(),
                        "pdf_sha256": file_hash(candidate), "pages": entries, "ocr_procedure": procedure,
                        "publication_validation": {"dpi": dpi, "selected_page_renders": "identical",
                                                   "all_page_geometry": "identical", "source_streams": "retained",
                                                   "ocr_text_insertion": "trusted_pdf_writer",
                                                   "unselected_pages": "unchanged; outside preparation coverage"}}
            candidate.replace(output)
            write_json(manifest_path, manifest)
            validate_text_layer(workspace, source)
        except BaseException:
            manifest_path.unlink(missing_ok=True)
            candidate.unlink(missing_ok=True)
            raise
        finally:
            (stage_dir / ".request.png").unlink(missing_ok=True)
    print(f"[+] Stage 00 published {output.name}; validated physical pages: {pages}")
    return manifest


def main():
    parser = argparse.ArgumentParser(description="Stage 00: prepare a validated, separate OCR PDF")
    parser.add_argument("--pdf", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    config = load_config(args.config, known_stages=PDF_STAGES, required_stages=())
    prepare_text_layer(args.pdf, args.workspace, config, args.page_ranges)


if __name__ == "__main__":
    main()

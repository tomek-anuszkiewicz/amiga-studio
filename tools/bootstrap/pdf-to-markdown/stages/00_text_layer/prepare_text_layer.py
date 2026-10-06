#!/usr/bin/env python3
"""Prepare a separate validated PDF; publish no manifest on incomplete OCR."""

import argparse
from pathlib import Path
import shutil
import sys
import pymupdf
from jsonschema import validate

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from conversion import pdf_schemas
from conversion.cache import digest
from conversion.config import load_config, PDF_STAGES
from conversion.lineage import file_hash, read_state, stage_identity
from conversion.pdf_artifacts import write_json, read_json, require, validate_text_layer
from conversion.pdf_geometry import (SCHEMA_VERSION, GEOMETRY_TOLERANCE, page_geometry,
                                     text_blocks, valid_box, validate_ocr, insert_ocr, validate_inserted)
from conversion.pdf_selection import selected_pages


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
    """Read rendered-page OCR lines in the shared normalized coordinate frame."""
    blocks = []
    with pymupdf.open("pdf", pixmap.pdfocr_tobytes(language=language, tessdata=tessdata)) as ocr:
        page = ocr[0]
        for block in page.get_text("dict")["blocks"]:
            if block["type"] != 0:
                continue
            for line in block["lines"]:
                text = "".join(span["text"] for span in line["spans"]).strip()
                if not text:
                    continue
                x0, y0, x1, y1 = valid_box(line["bbox"], page.rect.width, page.rect.height, clip=True)
                box = [y0 / page.rect.height * 1000,
                       x0 / page.rect.width * 1000, y1 / page.rect.height * 1000,
                       x1 / page.rect.width * 1000]
                blocks.append({"text": text, "box_2d": [round(value) for value in box]})
    # This is an extraction result, not visual page-type classification.
    return {"page_type": "text_page" if blocks else "pure_graphic", "caption": None, "blocks": blocks}


def text_layer_ocr(response):
    """Omit unreadable markers without joining readable fragments across them."""
    validate(response, pdf_schemas.OCR)
    if response["page_type"] in ("blank", "pure_graphic"):
        return validate_ocr(response)
    blocks = []
    for block in response["blocks"]:
        y0, x0, y1, x1 = block["box_2d"]
        y0, y1 = min(y0, y1), max(y0, y1)
        x0, x1 = min(x0, x1), max(x0, x1)
        valid_box([x0, y0, x1, y1], 1000, 1000, tolerance=0)
        text = block["text"].replace("\ufffd", " ")
        if "\ufffd" in block["text"] and not text.strip():
            continue
        blocks.append({**block, "text": text, "box_2d": [y0, x0, y1, x1]})
    return validate_ocr({**response, "blocks": blocks})


def record_ocr_read(path, source, entry, response):
    """Keep schema-shaped OCR output even when text validation rejects it."""
    record = {"schema_version": SCHEMA_VERSION, "source": source, **entry,
              "read_method": "ocr", "ocr_response": response,
              "text_validation": {"status": "pending"}}
    write_json(path, record)
    try:
        prepared = text_layer_ocr(response)
    except Exception as error:
        record["text_validation"] = {"status": "failed", "error_type": type(error).__name__,
                                     "error": str(error)}
        write_json(path, record)
        raise
    record["text_layer_response"] = prepared
    record["text_validation"] = {"status": "passed", "omitted_unreadable_markers":
                                 sum(block["text"].count("\ufffd") for block in response["blocks"]),
                                 "normalized_bounding_boxes": sum(
                                     block["box_2d"][0] > block["box_2d"][2]
                                     or block["box_2d"][1] > block["box_2d"][3]
                                     for block in response["blocks"])}
    write_json(path, record)
    return response


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
            if entry and entry["provenance"] == "ocr":
                validate_inserted(after, entry["ocr_lines"])
            else:
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
        page_reads = stage_dir / "page_reads"
        page_reads.mkdir(exist_ok=True)
        procedure = stage_identity({"id": "00", "dir": "00_text_layer"}, config, source, None, Path(__file__).resolve().parents[2])
        entries, engine, added_text = [], None, False
        try:
            for number in pages:
                page = document[number - 1]
                entry = {"page": number, "page_id": f"page_{number:04d}", "source_index": number - 1,
                         "geometry": page_geometry(page)}
                read_path = page_reads / f"{entry['page_id']}.json"
                native_blocks = text_blocks(page)
                if native_blocks:
                    entry.update(provenance="native", page_type="text_page", classification_basis="inferred_from_native_spans")
                    write_json(read_path, {"schema_version": SCHEMA_VERSION, "source": source, **entry,
                                           "read_method": "native", "native_blocks": native_blocks,
                                           "text_validation": {"status": "passed"}})
                    print(f"[native] {entry['page_id']}: retained existing text spans")
                else:
                    image = stage_dir / ".request.png"
                    pixmap = page.get_pixmap(dpi=dpi, colorspace=pymupdf.csRGB, alpha=False)
                    pixmap.save(image)
                    if engine is None:
                        engine = tesseract_settings(config)
                    language, tessdata, engine_identity = engine
                    identity = {"source": source, "page": number, "image_sha256": file_hash(image),
                                "geometry": entry["geometry"], "schema": pdf_schemas.OCR, "procedure": procedure,
                                "engine": engine_identity}
                    recovery_file = recovery / f"{entry['page_id']}.json"
                    response = None
                    if recovery_file.is_file():
                        retained = read_json(recovery_file)
                        require(retained.get("digest") == digest({k: v for k, v in retained.items() if k != "digest"}),
                                "Modified per-page OCR recovery record")
                        if retained.get("identity") == identity:
                            response = record_ocr_read(read_path, source, entry, retained["response"])
                            print(f"[recovery] {entry['page_id']}: compatible validated OCR result")
                    if response is None:
                        response = tesseract_ocr(pixmap, language, tessdata)
                        record_ocr_read(read_path, source, entry, response)
                        record = {"identity": identity, "response": response}
                        record["digest"] = digest(record)
                        write_json(recovery_file, record)
                    response = text_layer_ocr(response)
                    image.unlink()
                    entry.update(page_type=response["page_type"], caption=response["caption"],
                                 classification_basis="inferred_from_tesseract_lines", provenance="ocr" if response["blocks"] else "none")
                    if response["blocks"]:
                        entry["ocr_lines"] = insert_ocr(page, response)
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
                                                   "text_geometry_tolerance_points": GEOMETRY_TOLERANCE,
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

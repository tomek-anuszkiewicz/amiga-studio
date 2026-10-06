"""Runtime PDF handoff contracts; hashes alone do not validate page coverage."""

import json
from pathlib import Path
import pymupdf

from .cache import digest
from .lineage import file_hash, read_state
from .pdf_geometry import (SCHEMA_VERSION, COORDINATES, page_geometry, text_blocks,
                           valid_box, valid_text, raster_transform)


def write_json(path, value):
    path = Path(path)
    candidate = path.with_suffix(path.suffix + ".tmp")
    try:
        candidate.write_text(json.dumps(value, indent=2, ensure_ascii=False, allow_nan=False), encoding="utf-8")
        candidate.replace(path)
    finally:
        candidate.unlink(missing_ok=True)


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def artifact_path(workspace, relative):
    if not isinstance(relative, str) or "\\" in relative or ":" in relative:
        raise ValueError("Artifact path must be workspace-relative")
    path = workspace / relative
    if Path(relative).is_absolute() or not path.resolve().is_relative_to(workspace.resolve()):
        raise ValueError("Artifact path escapes its workspace")
    return path


def require(condition, message):
    if not condition:
        raise ValueError(message)


def coverage(manifest):
    require(type(manifest.get("schema_version")) is int and manifest["schema_version"] == SCHEMA_VERSION,
            "Unsupported PDF artifact schema")
    pages = manifest.get("selected_pages")
    count = manifest.get("source_page_count")
    require(type(count) is int and count > 0 and isinstance(pages, list) and bool(pages), "Invalid PDF page coverage")
    require(all(type(p) is int and 1 <= p <= count for p in pages) and sorted(set(pages)) == pages,
            "Invalid or duplicate physical PDF page selection")
    entries = manifest.get("pages")
    require(isinstance(entries, list) and [entry.get("page") for entry in entries] == pages,
            "PDF manifest does not cover the exact selected page set")
    require([entry.get("page_id") for entry in entries] == [f"page_{p:04d}" for p in pages],
            "PDF manifest page identities differ from physical page numbers")
    return entries


def validate_text_layer(workspace, source=None, *, require_completion=False):
    manifest_path = workspace / "00_text_layer/text_layer_manifest.json"
    if not manifest_path.is_file():
        raise ValueError("Missing validated Stage 00: restart with --pdf <source.pdf> --from-stage 00")
    manifest = read_json(manifest_path)
    entries = coverage(manifest)
    if source is not None:
        require(manifest.get("source") == source, "Stage 00 source/selection identity mismatch")
    identity = manifest.get("source", {})
    require(isinstance(identity.get("name"), str) and Path(identity["name"]).name == identity["name"]
            and isinstance(identity.get("sha256"), str) and len(identity["sha256"]) == 64,
            "Stage 00 source identity is invalid")
    procedure = manifest.get("ocr_procedure", {})
    require(procedure.get("source") == identity and procedure.get("stage") == "00_text_layer"
            and procedure.get("selection") is None and bool(procedure.get("procedure")),
            "Stage 00 OCR procedure/configuration identity is missing")
    require(identity.get("pages") is None and manifest["selected_pages"] == list(range(1, manifest["source_page_count"] + 1))
            or identity.get("pages") == manifest["selected_pages"], "Stage 00 selection differs from its source identity")
    relative = f"00_text_layer/{Path(identity['name']).stem}-ocr.pdf"
    require(manifest.get("pdf_file") == relative, "Unexpected Stage 00 prepared PDF path")
    pdf = artifact_path(workspace, relative)
    require(pdf.is_file() and file_hash(pdf) == manifest.get("pdf_sha256"), "Stage 00 prepared PDF is missing or modified")
    if require_completion:
        record = read_state(workspace).get("stages", {}).get("00", {})
        require(record.get("status") == "completed", "Stage 00 has no validated completion record; restart at 00")
        require(record.get("completion") == digest({k: v for k, v in record.items() if k != "completion"}),
                "Damaged Stage 00 completion record")
        require(record.get("identity", {}).get("source") == identity, "Stage 00 completion source mismatch")
        require(record.get("identity") == procedure, "Stage 00 manifest procedure differs from its completion record")
        for relative_name, expected_hash in record["files"].items():
            anchor, name = relative_name.split(":", 1)
            require(anchor == "workspace", "Invalid Stage 00 artifact anchor")
            path = artifact_path(workspace, name)
            require(path.is_file() and file_hash(path) == expected_hash, f"Missing/modified Stage 00 artifact: {name}")
        require(record["files"].get("workspace:" + relative) == manifest["pdf_sha256"]
                and record["files"].get("workspace:00_text_layer/text_layer_manifest.json") == file_hash(manifest_path),
                "Stage 00 completion omits required artifacts")
    with pymupdf.open(pdf) as document:
        require(document.is_pdf and not document.needs_pass and len(document) == manifest["source_page_count"],
                "Prepared PDF page tree differs from source")
        for entry in entries:
            page = document[entry["page"] - 1]
            require(entry.get("source_index") == page.number and entry.get("geometry") == page_geometry(page),
                    "Stage 00 page index/geometry mismatch")
            provenance = entry.get("provenance")
            kind = entry.get("page_type")
            require(provenance in ("native", "ocr", "none") and kind in ("text_page", "schematic", "diagram", "blank", "pure_graphic"),
                    "Unresolved Stage 00 page classification/provenance")
            require(entry.get("classification_basis") in ("inferred_from_native_spans", "inferred_from_tesseract_lines"),
                    "Missing Stage 00 classification evidence")
            blocks = text_blocks(page)
            require((provenance == "ocr" or bool(blocks) == (provenance != "none"))
                    and (kind in ("blank", "pure_graphic")) == (provenance == "none"),
                    "Inconsistent Stage 00 text/classification")
            require(entry.get("text_digest") == digest(blocks), "Stage 00 extracted text differs from its manifest")
    return manifest, pdf


def validate_preprocess(workspace, source=None, *, manifest_path=None):
    prepared, pdf = validate_text_layer(workspace, source)
    manifest = read_json(manifest_path or workspace / "pages_manifest.json")
    entries = coverage(manifest)
    require(manifest.get("pdf_file") == prepared["pdf_file"] and manifest.get("pdf_sha256") == prepared["pdf_sha256"]
            and manifest["selected_pages"] == prepared["selected_pages"]
            and manifest["source_page_count"] == prepared["source_page_count"]
            and manifest.get("source") == prepared["source"], "Stage 01 prepared-PDF/coverage identity mismatch")
    require(manifest.get("total_pages") == len(entries), "Stage 01 selected page count mismatch")
    dpi = manifest.get("dpi")
    require(type(dpi) is int and dpi > 0, "Invalid Stage 01 raster DPI")
    with pymupdf.open(pdf) as document:
        for entry, origin in zip(entries, prepared["pages"]):
            page_id = entry["page_id"]
            page = document[entry["page"] - 1]
            for field, suffix in (("png_file", ".png"), ("json_file", ".json")):
                require(entry.get(field) == f"01_preprocess/{page_id}{suffix}", "Stage 01 page pair identity mismatch")
                path = artifact_path(workspace, entry[field])
                require(path.is_file() and file_hash(path) == entry.get(field.replace("_file", "_sha256")),
                        f"Missing/modified Stage 01 page pair: {page_id}")
            data = read_json(workspace / entry["json_file"])
            geometry = page_geometry(page)
            require(type(data.get("schema_version")) is int, "Stage 01 page schema version must be an integer")
            block_ids = []
            for block in data.get("blocks", []):
                valid_text(block["text"])
                valid_box(block["bbox"], geometry["width"], geometry["height"])
                require(type(block.get("block_id")) is int and type(block.get("type")) is int and block["type"] == 0,
                        "Stage 01 text block identity/type is invalid")
                block_ids.append(block["block_id"])
            require(len(block_ids) == len(set(block_ids)), "Stage 01 duplicate text block IDs")
            for field, value in {"schema_version": SCHEMA_VERSION, "page": entry["page"], "page_id": page_id,
                                 "source_index": page.number, "pdf_sha256": prepared["pdf_sha256"],
                                 "geometry": geometry, "width": geometry["width"], "height": geometry["height"],
                                 "rotation": page.rotation, "coordinates": COORDINATES,
                                 "provenance": origin["provenance"], "page_type": origin["page_type"],
                                 "classification_basis": origin["classification_basis"]}.items():
                require(data.get(field) == value, f"Stage 01 page JSON {field} mismatch: {page_id}")
            require(data.get("blocks") == text_blocks(page), "Stage 01 text is not derived from its reopened PDF")
            image = pymupdf.Pixmap(workspace / entry["png_file"])
            transform = raster_transform(page, image, dpi)
            require(data.get("raster") == transform and entry.get("raster") == transform, "Stage 01 PNG transform mismatch")
            for field in ("width", "height", "rotation", "provenance", "page_type", "classification_basis"):
                require(entry.get(field) == data[field], f"Stage 01 manifest {field} mismatch")
            require(entry.get("pdf_sha256") == prepared["pdf_sha256"] and entry.get("block_count") == len(data["blocks"]),
                    "Stage 01 page manifest identity/block count mismatch")
    return manifest


def validate_stage_artifacts(stage, workspace, source, *, snapshot=False):
    contract = stage.get("artifact_contract")
    if contract == "pdf_text_layer":
        validate_text_layer(workspace, source)
    elif contract == "pdf_preprocess":
        path = workspace / stage["dir"] / ".manifests/pages_manifest.json" if snapshot else None
        validate_preprocess(workspace, source, manifest_path=path)

#!/usr/bin/env python3
"""Extract positioned text and matching PNGs only from validated Stage 00."""

import argparse
from pathlib import Path
import sys
import pymupdf

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion.config import load_config, PDF_STAGES
from common.lineage import file_hash
from common.pdf_artifacts import validate_text_layer, validate_preprocess, write_json, require, prepared_text_metadata
from common.pdf_geometry import SCHEMA_VERSION, COORDINATES, page_geometry, text_blocks, raster_transform
from common.pdf_selection import selected_pages


def preprocess_pdf(pdf_path, workspace_dir, dpi=300, page_ranges=None):
    workspace_dir = Path(workspace_dir).resolve()
    prepared, prepared_pdf = validate_text_layer(workspace_dir, require_completion=True)
    require(Path(pdf_path).resolve() == prepared_pdf.resolve(), "Stage 01 must read only the validated Stage 00 PDF")
    require(type(dpi) is int and dpi > 0, "render.dpi must be a positive integer")
    with pymupdf.open(prepared_pdf) as document:
        require(len(document) == prepared["source_page_count"], "Prepared PDF must contain every source page")
        pages = selected_pages(page_ranges, prepared["source_page_count"]) if page_ranges is not None else (prepared["source"]["pages"] or prepared["selected_pages"])
        require(pages == (prepared["source"]["pages"] or prepared["selected_pages"]),
                "Stage 01 page selection differs from workspace selection; restart at 00")
        directory = workspace_dir / "01_preprocess"
        directory.mkdir(parents=True, exist_ok=True)
        manifest_path = workspace_dir / "pages_manifest.json"
        manifest_path.unlink(missing_ok=True)
        (directory / ".manifests/pages_manifest.json").unlink(missing_ok=True)
        for old in directory.glob("page_*.*"):
            old.unlink()
        manifest = {"schema_version": SCHEMA_VERSION, "source": prepared["source"],
                    "source_pdf": prepared["pdf_file"], "pdf_file": prepared["pdf_file"],
                    "source_page_count": prepared["source_page_count"],
                    "selected_pages": pages, "total_pages": len(pages), "start_page": pages[0],
                    "end_page": pages[-1], "dpi": dpi, "pages": []}
        for origin in prepared["pages"]:
            number, page_id = origin["page"], origin["page_id"]
            if number not in pages:
                continue
            page = document[origin["prepared_index"]]
            geometry = page_geometry(page)
            pixmap = page.get_pixmap(dpi=dpi)
            raster = raster_transform(page, pixmap, dpi)
            png = directory / f"{page_id}.png"
            pixmap.save(png)
            blocks = text_blocks(page)
            metadata = prepared_text_metadata(blocks)
            data = {"schema_version": SCHEMA_VERSION, "page_id": page_id, "page": number,
                    "source_index": number - 1, "prepared_index": origin["prepared_index"],
                    "geometry": geometry,
                    "width": geometry["width"], "height": geometry["height"], "rotation": page.rotation,
                    "coordinates": COORDINATES, "raster": raster, "blocks": blocks, **metadata}
            json_file = directory / f"{page_id}.json"
            write_json(json_file, data)
            manifest["pages"].append({"page_id": page_id, "page": number,
                                      "png_file": png.relative_to(workspace_dir).as_posix(),
                                      "json_file": json_file.relative_to(workspace_dir).as_posix(),
                                      "png_sha256": file_hash(png), "json_sha256": file_hash(json_file),
                                      "width": geometry["width"],
                                      "height": geometry["height"], "rotation": page.rotation,
                                      "block_count": len(blocks), "raster": raster, **metadata})
            print(f"[extract] {page_id}: {len(blocks)} text blocks from {prepared_pdf.name}")
        candidate = workspace_dir / ".pages_manifest.candidate.json"
        try:
            write_json(candidate, manifest)
            validate_preprocess(workspace_dir, prepared["source"], manifest_path=candidate)
            candidate.replace(manifest_path)
        finally:
            candidate.unlink(missing_ok=True)
    print("[+] Stage 01 published validated PNG/text pairs; no OCR execution")
    return manifest


def main():
    parser = argparse.ArgumentParser(description="Stage 01: deterministic prepared-PDF preprocessing")
    parser.add_argument("--pdf", type=Path, required=True, help="Validated Stage 00 PDF")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    config = load_config(args.config, known_stages=PDF_STAGES, required_stages=())
    preprocess_pdf(args.pdf, args.workspace, config.get("render", {}).get("dpi", 300), args.page_ranges)


if __name__ == "__main__":
    main()

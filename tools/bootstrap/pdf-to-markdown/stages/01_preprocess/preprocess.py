#!/usr/bin/env python3
"""Extract positioned text and PNGs from the full-source prepared PDF."""

import argparse
from pathlib import Path
import sys
import pymupdf

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from common.pdf_runtime import RENDER_DPI
from conversion.config import load_config, PDF_STAGES
from common.pdf_artifacts import write_json, prepared_text_metadata
from common.pdf_geometry import SCHEMA_VERSION, COORDINATES, page_geometry, text_blocks, raster_transform
from common.pdf_selection import parse_page_ranges


def preprocess_pdf(pdf_path, workspace_dir, dpi=RENDER_DPI, page_ranges=None):
    directory = Path(workspace_dir) / "01_preprocess"
    directory.mkdir(parents=True, exist_ok=True)
    for old in directory.glob("page_*.*"):
        old.unlink()
    with pymupdf.open(pdf_path) as document:
        pages = parse_page_ranges(page_ranges) if page_ranges is not None else range(1, len(document) + 1)
        for number in pages:
            page_id = f"page_{number:04d}"
            page = document[number - 1]
            geometry = page_geometry(page)
            pixmap = page.get_pixmap(dpi=dpi)
            raster = raster_transform(page, pixmap, dpi)
            pixmap.save(directory / f"{page_id}.png")
            blocks = text_blocks(page)
            data = {"schema_version": SCHEMA_VERSION, "page_id": page_id, "page": number,
                    "source_index": number - 1, "prepared_index": number - 1,
                    "geometry": geometry,
                    "width": geometry["width"], "height": geometry["height"], "rotation": page.rotation,
                    "coordinates": COORDINATES, "raster": raster, "blocks": blocks,
                    **prepared_text_metadata(blocks)}
            write_json(directory / f"{page_id}.json", data)
            print(f"[extract] {page_id}: {len(blocks)} text blocks from {Path(pdf_path).name}")
    print("[+] Stage 01 published page PNGs and text JSON; no OCR execution")


def main():
    parser = argparse.ArgumentParser(description="Stage 01: deterministic prepared-PDF preprocessing")
    parser.add_argument("--pdf", type=Path, required=True, help="Full-source prepared PDF")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    config = load_config(args.config, known_stages=PDF_STAGES, required_stages=())
    preprocess_pdf(args.pdf, args.workspace, RENDER_DPI, args.page_ranges)


if __name__ == "__main__":
    main()

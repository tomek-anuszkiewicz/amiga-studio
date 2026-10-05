#!/usr/bin/env python3
"""
stages/01_preprocess/preprocess.py:
Deconstructs a physical input PDF into atomic, per-page representations:
- Single-page vector PDF (page_XXXX.pdf)
- 300 DPI high-resolution raster image (page_XXXX.png)
- Geometry text dump from fitz.get_text("blocks") (page_XXXX.json)
- Master workspace/pages_manifest.json
"""

import argparse
import json
import os
import sys
from typing import Optional, List
from pathlib import Path
import pymupdf
import yaml

try:
    from .detect_and_ocr import detect_and_ocr_pages, parse_page_ranges
except ImportError:
    from detect_and_ocr import detect_and_ocr_pages, parse_page_ranges


def preprocess_pdf(
    pdf_path: Path,
    workspace_dir: Path,
    dpi: int = 300,
    page_ranges: Optional[str] = None,
    ocr_threshold: int = 20,
    config_path: Optional[Path] = None,
) -> dict:
    if not pdf_path.exists():
        raise FileNotFoundError(f"Source PDF does not exist: {pdf_path}")

    pages_dir = workspace_dir / "01_preprocess"
    pages_dir.mkdir(parents=True, exist_ok=True)

    print(f"[*] Opening PDF: {pdf_path}")
    doc = pymupdf.open(str(pdf_path))
    doc_len = len(doc)

    if page_ranges:
        target_pages = parse_page_ranges(page_ranges)
        pages_to_process = [p for p in target_pages if 1 <= p <= doc_len]
        start_page = pages_to_process[0] if pages_to_process else 1
        end_page = pages_to_process[-1] if pages_to_process else doc_len
        print(f"[*] Processing {len(pages_to_process)} discrete pages: {pages_to_process} (of {doc_len} in doc), Rendering at {dpi} DPI")
    else:
        start_page = 1
        end_page = doc_len
        pages_to_process = list(range(1, doc_len + 1))
        print(f"[*] Processing pages 1 to {doc_len} (all {doc_len} in doc), Rendering at {dpi} DPI")

    # Clean existing stage artifacts for targeted pages to guarantee a fresh, idempotent start
    (pages_dir / ".metrics.json").unlink(missing_ok=True)
    (workspace_dir / ".stage_01_metrics.json").unlink(missing_ok=True)
    for p in pages_to_process:
        p_str = f"page_{p:04d}"
        for old_file in pages_dir.glob(f"{p_str}.*"):
            old_file.unlink(missing_ok=True)

    manifest = {
        "source_pdf": str(pdf_path),
        "total_pages": len(pages_to_process),
        "start_page": start_page,
        "end_page": end_page,
        "dpi": dpi,
        "pages": []
    }

    for page_num in pages_to_process:
        page_idx = page_num - 1
        page_str = f"page_{page_num:04d}"
        page = doc[page_idx]
        rect = page.rect

        # 1. Save single-page vector PDF
        single_doc = pymupdf.open()
        single_doc.insert_pdf(doc, from_page=page_idx, to_page=page_idx)
        pdf_out_path = pages_dir / f"{page_str}.pdf"
        single_doc.save(str(pdf_out_path))
        single_doc.close()

        # 2. Render 300 DPI PNG
        pix = page.get_pixmap(dpi=dpi)
        png_out_path = pages_dir / f"{page_str}.png"
        pix.save(str(png_out_path))

        # 3. Extract text blocks and word geometry
        blocks = page.get_text("blocks")
        # Structure each block: [x0, y0, x1, y1, text, block_no, block_type]
        block_list = []
        for b in blocks:
            block_list.append({
                "bbox": [round(coord, 2) for coord in b[:4]],
                "text": b[4],
                "block_id": b[5],
                "type": b[6],
                "bbox_norm": [
                    round(b[0] / rect.width, 4),
                    round(b[1] / rect.height, 4),
                    round(b[2] / rect.width, 4),
                    round(b[3] / rect.height, 4),
                ]
            })

        json_out_path = pages_dir / f"{page_str}.json"
        with open(json_out_path, "w", encoding="utf-8") as f:
            json.dump({
                "page": page_num,
                "width": round(rect.width, 2),
                "height": round(rect.height, 2),
                "blocks": block_list
            }, f, indent=2)

        manifest["pages"].append({
            "page": page_num,
            "pdf_file": f"01_preprocess/{page_str}.pdf",
            "png_file": f"01_preprocess/{page_str}.png",
            "json_file": f"01_preprocess/{page_str}.json",
            "width": round(rect.width, 2),
            "height": round(rect.height, 2),
            "block_count": len(block_list)
        })

        if page_num % 10 == 0 or page_num == end_page:
            print(f"    Processed page {page_num}...")

    doc.close()

    manifest_path = workspace_dir / "pages_manifest.json"
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)

    # 4. Run scan detection & Gemini Vision OCR
    detect_and_ocr_pages(
        workspace_dir=workspace_dir,
        config_path=config_path,
        page_ranges=page_ranges,
        threshold=ocr_threshold,
    )

    print(f"[+] Stage 01 complete. Manifest saved to {manifest_path}")
    return manifest


def main():
    parser = argparse.ArgumentParser(description="Stage 01: Preprocess PDF into atomic per-page assets")
    parser.add_argument("--pdf", type=str, required=True, help="Input PDF document")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")
    parser.add_argument("--page-ranges", type=str, default=None, help="Pages or ranges to process (e.g. '1-5, 7, 8, 10-15' or '1..5')")
    parser.add_argument("--ocr-threshold", type=int, default=20, help="Character threshold below which a page is considered a scan (default: 20)")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    pdf_path = Path(args.pdf)

    config_path = Path(args.config)
    if not config_path.is_file():
        raise FileNotFoundError(f"Stage 01: Config file not found: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        cfg = yaml.safe_load(f)
    if not cfg or not isinstance(cfg, dict):
        raise ValueError(f"Stage 01: Config file is empty or invalid: {config_path}")

    dpi = cfg.get("render", {}).get("dpi", 300)
    ocr_threshold = args.ocr_threshold
    ocr_cfg = cfg.get("ocr", {})
    if "threshold" in ocr_cfg:
        ocr_threshold = ocr_cfg["threshold"]

    preprocess_pdf(
        pdf_path,
        workspace_dir,
        dpi=dpi,
        page_ranges=args.page_ranges,
        ocr_threshold=ocr_threshold,
        config_path=config_path,
    )


if __name__ == "__main__":
    main()

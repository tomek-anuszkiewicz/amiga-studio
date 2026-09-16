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
from pathlib import Path
import pymupdf
import yaml


def preprocess_pdf(
    pdf_path: Path,
    workspace_dir: Path,
    dpi: int = 300,
    max_pages: int = None,
    start_page: int = 1,
    end_page: int = None
) -> dict:
    if not pdf_path.exists():
        raise FileNotFoundError(f"Source PDF does not exist: {pdf_path}")

    pages_dir = workspace_dir / "01_preprocess"
    pages_dir.mkdir(parents=True, exist_ok=True)

    print(f"[*] Opening PDF: {pdf_path}")
    doc = pymupdf.open(str(pdf_path))
    doc_len = len(doc)

    if start_page is None or start_page < 1:
        start_page = 1
    if end_page is not None:
        end_page = min(doc_len, end_page)
    elif max_pages is not None and max_pages > 0:
        end_page = min(doc_len, start_page + max_pages - 1)
    else:
        end_page = doc_len

    pages_to_process = list(range(start_page, end_page + 1))
    print(f"[*] Pages to process: {start_page}..{end_page} ({len(pages_to_process)} pages of {doc_len} in doc), Rendering at {dpi} DPI")

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

    print(f"[+] Stage 01 complete. Manifest saved to {manifest_path}")
    return manifest


def main():
    parser = argparse.ArgumentParser(description="Stage 01: Preprocess PDF into atomic per-page assets")
    parser.add_argument("--pdf", type=str, required=True, help="Input PDF document")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")
    parser.add_argument("--max-pages", type=int, default=None, help="Maximum number of pages to process")
    parser.add_argument("--page-range", type=str, default=None, help="Page range to process (e.g. 173-178 or 173..178)")
    parser.add_argument("--start-page", type=int, default=1, help="Start page number (1-indexed)")
    parser.add_argument("--end-page", type=int, default=None, help="End page number (1-indexed)")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    pdf_path = Path(args.pdf)

    dpi = 300
    config_path = Path(args.config)
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            cfg = yaml.safe_load(f) or {}
            dpi = cfg.get("render", {}).get("dpi", 300)

    start_page = args.start_page
    end_page = args.end_page
    if args.page_range:
        import re
        parts = [p.strip() for p in re.split(r"[-..:]+", args.page_range) if p.strip()]
        if len(parts) >= 2:
            start_page = int(parts[0])
            end_page = int(parts[1])
        elif len(parts) == 1:
            start_page = int(parts[0])
            end_page = int(parts[0])

    preprocess_pdf(pdf_path, workspace_dir, dpi=dpi, max_pages=args.max_pages, start_page=start_page, end_page=end_page)


if __name__ == "__main__":
    main()

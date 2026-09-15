#!/usr/bin/env python3
"""
stages/01_preprocess/preprocess.py:
Deconstructs a physical input PDF into atomic, per-page representations:
- Single-page vector PDF (page_XXXX.pdf)
- 300 DPI high-resolution raster image (page_XXXX.png)
- Geometry text dump from fitz.get_text("blocks") (page_XXXX.json)
- Master workspace/manifest.json
"""

import argparse
import json
import os
import sys
from pathlib import Path
import pymupdf
import yaml


def preprocess_pdf(pdf_path: Path, workspace_dir: Path, dpi: int = 300) -> dict:
    if not pdf_path.exists():
        raise FileNotFoundError(f"Source PDF does not exist: {pdf_path}")

    pages_dir = workspace_dir / "pages"
    pages_dir.mkdir(parents=True, exist_ok=True)

    print(f"[*] Opening PDF: {pdf_path}")
    doc = pymupdf.open(str(pdf_path))
    total_pages = len(doc)
    print(f"[*] Total pages: {total_pages}, Rendering at {dpi} DPI")

    manifest = {
        "source_pdf": str(pdf_path),
        "total_pages": total_pages,
        "dpi": dpi,
        "pages": []
    }

    for page_idx in range(total_pages):
        page_num = page_idx + 1
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
            "pdf_file": f"pages/{page_str}.pdf",
            "png_file": f"pages/{page_str}.png",
            "json_file": f"pages/{page_str}.json",
            "width": round(rect.width, 2),
            "height": round(rect.height, 2),
            "block_count": len(block_list)
        })

        if page_num % 10 == 0 or page_num == total_pages:
            print(f"    Processed {page_num}/{total_pages} pages...")

    doc.close()

    manifest_path = workspace_dir / "manifest.json"
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)

    print(f"[+] Stage 01 complete. Manifest saved to {manifest_path}")
    return manifest


def main():
    parser = argparse.ArgumentParser(description="Stage 01: Preprocess PDF into atomic per-page assets")
    parser.add_argument("--pdf", type=str, required=True, help="Input PDF document")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    pdf_path = Path(args.pdf)

    dpi = 300
    config_path = Path(args.config)
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            cfg = yaml.safe_load(f) or {}
            dpi = cfg.get("render", {}).get("dpi", 300)

    preprocess_pdf(pdf_path, workspace_dir, dpi=dpi)


if __name__ == "__main__":
    main()

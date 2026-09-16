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
    from .detect_and_ocr import detect_and_ocr_pages
except ImportError:
    from detect_and_ocr import detect_and_ocr_pages


def preprocess_pdf(
    pdf_path: Path,
    workspace_dir: Path,
    dpi: int = 300,
    max_pages: Optional[int] = None,
    start_page: int = 1,
    end_page: Optional[int] = None,
    pages_list: Optional[List[int]] = None,
    auto_ocr: bool = True,
    force_ocr: bool = False,
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

    if pages_list:
        pages_to_process = [p for p in sorted(list(set(pages_list))) if 1 <= p <= doc_len]
        start_page = pages_to_process[0] if pages_to_process else 1
        end_page = pages_to_process[-1] if pages_to_process else doc_len
        print(f"[*] Processing {len(pages_to_process)} discrete pages: {pages_to_process} (of {doc_len} in doc), Rendering at {dpi} DPI")
    else:
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

    # 4. Check for scanned or low-text pages and trigger Gemini Vision OCR
    if auto_ocr:
        scanned_pages_detected = False
        for p_info in manifest["pages"]:
            jf = workspace_dir / p_info["json_file"]
            if jf.exists():
                try:
                    with open(jf, "r", encoding="utf-8") as f:
                        p_data = json.load(f)
                    b_list = p_data.get("blocks", [])
                    total_chars = sum(len(b.get("text", "").strip()) for b in b_list if isinstance(b, dict))
                    if len(b_list) == 0 or total_chars < ocr_threshold:
                        scanned_pages_detected = True
                        break
                except Exception:
                    pass

        if scanned_pages_detected or force_ocr:
            print(f"\n[*] Scanned or low-text pages detected (threshold: {ocr_threshold} chars). Initiating Gemini Vision OCR pass...")
            ocr_success = detect_and_ocr_pages(
                workspace_dir=workspace_dir,
                config_path=config_path,
                page_range=",".join(str(p) for p in pages_to_process),
                force=force_ocr,
                threshold=ocr_threshold,
            )
            if ocr_success:
                # Refresh block_count and page_type in manifest
                for p_info in manifest["pages"]:
                    jf = workspace_dir / p_info["json_file"]
                    if jf.exists():
                        try:
                            with open(jf, "r", encoding="utf-8") as f:
                                p_data = json.load(f)
                            p_info["block_count"] = len(p_data.get("blocks", []))
                            if "page_type" in p_data:
                                p_info["page_type"] = p_data["page_type"]
                        except Exception:
                            pass
                with open(manifest_path, "w", encoding="utf-8") as f:
                    json.dump(manifest, f, indent=2)
            else:
                print("[!] Warning: OCR pass completed with errors or was skipped.", file=sys.stderr)

    print(f"[+] Stage 01 complete. Manifest saved to {manifest_path}")
    return manifest


def main():
    parser = argparse.ArgumentParser(description="Stage 01: Preprocess PDF into atomic per-page assets (with auto-OCR)")
    parser.add_argument("--pdf", type=str, required=True, help="Input PDF document")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")
    parser.add_argument("--max-pages", type=int, default=None, help="Maximum number of pages to process")
    parser.add_argument("--page-range", type=str, default=None, help="Page range or list (e.g. '16,17,18, 32,37,50,73' or '173-178')")
    parser.add_argument("--pages", dest="pages_alt", type=str, default=None, help="Alias for --page-range")
    parser.add_argument("--start-page", type=int, default=1, help="Start page number (1-indexed)")
    parser.add_argument("--end-page", type=int, default=None, help="End page number (1-indexed)")
    parser.add_argument("--no-ocr", action="store_true", help="Disable automatic scan detection and OCR")
    parser.add_argument("--force-ocr", action="store_true", help="Force OCR on all pages even if native text is present")
    parser.add_argument("--ocr-threshold", type=int, default=20, help="Character threshold below which a page is considered a scan (default: 20)")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    pdf_path = Path(args.pdf)

    dpi = 300
    config_path = Path(args.config)
    ocr_threshold = args.ocr_threshold
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            cfg = yaml.safe_load(f) or {}
            dpi = cfg.get("render", {}).get("dpi", 300)
            ocr_cfg = cfg.get("ocr", {})
            if "threshold" in ocr_cfg:
                ocr_threshold = ocr_cfg["threshold"]

    start_page = args.start_page
    end_page = args.end_page
    pages_list = None
    spec = args.page_range or args.pages_alt
    if spec:
        pages_list = []
        for segment in spec.split(","):
            segment = segment.strip()
            if not segment:
                continue
            if "-" in segment:
                s, e = segment.split("-", 1)
                pages_list.extend(range(int(s.strip()), int(e.strip()) + 1))
            elif ".." in segment:
                s, e = segment.split("..", 1)
                pages_list.extend(range(int(s.strip()), int(e.strip()) + 1))
            elif ":" in segment:
                s, e = segment.split(":", 1)
                pages_list.extend(range(int(s.strip()), int(e.strip()) + 1))
            else:
                pages_list.append(int(segment))

    preprocess_pdf(
        pdf_path,
        workspace_dir,
        dpi=dpi,
        max_pages=args.max_pages,
        start_page=start_page,
        end_page=end_page,
        pages_list=pages_list,
        auto_ocr=not args.no_ocr,
        force_ocr=args.force_ocr,
        ocr_threshold=ocr_threshold,
        config_path=config_path if config_path.exists() else None,
    )


if __name__ == "__main__":
    main()

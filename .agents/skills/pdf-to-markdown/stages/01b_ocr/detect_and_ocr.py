#!/usr/bin/env python3
"""
stages/01b_ocr/detect_and_ocr.py:
Dedicated scan detection and Gemini Vision OCR step.
Inspects the output of Stage 01 (01_preprocess).
- If pages contain native text blocks (born-digital PDF), it passes through (0 API calls).
- If pages lack text (scans or scanned book covers), it uses Gemini Vision OCR
  to extract text blocks and normalized bounding boxes into page_XXXX.json.
"""

import argparse
import json
import sys
from pathlib import Path
from typing import Optional, List
import yaml

# Import GeminiClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

try:
    from llm_client import GeminiClient
except ImportError:
    GeminiClient = None


def parse_page_range(range_str: str) -> List[int]:
    pages = set()
    for part in range_str.split(","):
        part = part.strip()
        if not part:
            continue
        if "-" in part:
            start, end = part.split("-", 1)
            pages.update(range(int(start), int(end) + 1))
        else:
            pages.add(int(part))
    return sorted(list(pages))


def parse_ocr_bounding_box(item: dict) -> tuple[float, float, float, float]:
    """
    Parses normalized bounding box coordinates [x0, y0, x1, y1] in range [0.0, 1.0].
    Supports:
    1. Gemini native integer 'box_2d': [ymin, xmin, ymax, xmax] in [0..1000]
    2. Fallback 'bbox_norm': [x0, y0, x1, y1] (floats [0.0..1.0] or ints [0..1000])
    Enforces coordinate sanity validation and ordering.
    """
    x0, y0, x1, y1 = 0.0, 0.0, 1.0, 1.0
    if "box_2d" in item and isinstance(item["box_2d"], list) and len(item["box_2d"]) == 4:
        try:
            ymin, xmin, ymax, xmax = [float(v) for v in item["box_2d"]]
            scale = 1000.0 if any(v > 1.0 for v in (ymin, xmin, ymax, xmax)) else 1.0
            x0 = xmin / scale
            y0 = ymin / scale
            x1 = xmax / scale
            y1 = ymax / scale
        except (ValueError, TypeError):
            pass
    elif "bbox_norm" in item and isinstance(item["bbox_norm"], list) and len(item["bbox_norm"]) == 4:
        try:
            c0, c1, c2, c3 = [float(v) for v in item["bbox_norm"]]
            scale = 1000.0 if any(v > 1.0 for v in (c0, c1, c2, c3)) else 1.0
            x0 = c0 / scale
            y0 = c1 / scale
            x1 = c2 / scale
            y1 = c3 / scale
        except (ValueError, TypeError):
            pass

    # Sanity validation: clamp to [0.0, 1.0]
    x0 = max(0.0, min(1.0, x0))
    y0 = max(0.0, min(1.0, y0))
    x1 = max(0.0, min(1.0, x1))
    y1 = max(0.0, min(1.0, y1))

    # Ensure valid ordering
    if x0 > x1:
        x0, x1 = x1, x0
    if y0 > y1:
        y0, y1 = y1, y0

    return x0, y0, x1, y1


def detect_and_ocr_pages(
    workspace_dir: Path,
    config_path: Path,
    page_range: Optional[str] = None,
    start_page: Optional[int] = None,
    end_page: Optional[int] = None,
    max_pages: Optional[int] = None,
    force: bool = False,
    threshold: int = 20,
) -> bool:
    if (workspace_dir / "01_preprocess").exists():
        pages_dir = workspace_dir / "01_preprocess"
    elif (workspace_dir / "01_pages").exists():
        pages_dir = workspace_dir / "01_pages"
    else:
        pages_dir = workspace_dir / "pages"

    if not pages_dir.exists():
        print(f"[!] Error: Preprocess directory not found: {pages_dir}", file=sys.stderr)
        return False

    prompt_path = Path(__file__).resolve().parent / "prompt_ocr.md"
    if not prompt_path.exists():
        print(f"[!] Error: OCR prompt not found: {prompt_path}", file=sys.stderr)
        return False
    prompt_text = prompt_path.read_text(encoding="utf-8")

    json_files = sorted(list(pages_dir.glob("page_*.json")))
    if not json_files:
        print(f"[!] Warning: No page_*.json files found in {pages_dir}")
        return True

    # Filter target pages
    target_pages = None
    if page_range:
        target_pages = parse_page_range(page_range)
    elif start_page or end_page or max_pages:
        s = start_page or 1
        e = end_page or (s + max_pages - 1 if max_pages else 999999)
        target_pages = list(range(s, e + 1))

    gemini = None
    scanned_count = 0
    skipped_count = 0
    total_ocr_blocks = 0

    print(f"[*] Inspecting {len(json_files)} pages for scan detection (threshold: {threshold} chars)...")

    if hasattr(sys.stdout, "reconfigure"):
        try:
            sys.stdout.reconfigure(encoding="utf-8")
        except Exception:
            pass

    for jf in json_files:
        try:
            with open(jf, "r", encoding="utf-8") as f:
                page_data = json.load(f)
        except Exception as err:
            print(f"[!] Error reading {jf.name}: {err}", file=sys.stderr)
            continue

        page_num = page_data.get("page", 0)
        if target_pages is not None and page_num not in target_pages:
            continue

        blocks = page_data.get("blocks", [])
        total_chars = sum(len(b.get("text", "").strip()) for b in blocks if isinstance(b, dict))

        # Check if page is considered digital or scan
        is_scanned = (len(blocks) == 0 or total_chars < threshold) or force

        if not is_scanned:
            skipped_count += 1
            print(f"  [PASS] Page {page_num:04d}: Native text present ({len(blocks)} blocks, {total_chars} chars) -> Pass-through")
            continue

        # Page is scanned or lacks text
        page_str = f"page_{page_num:04d}"
        png_path = pages_dir / f"{page_str}.png"
        if not png_path.exists():
            print(f"  [!] Page {page_num:04d}: Scanned page missing PNG at {png_path.name}, skipping OCR")
            continue

        print(f"  [SCAN] Page {page_num:04d}: Scanned page detected ({len(blocks)} blocks, {total_chars} chars). Running Gemini Vision OCR...")

        if gemini is None:
            if GeminiClient is None:
                print("[!] Error: GeminiClient unavailable. Check llm_client.py dependencies.", file=sys.stderr)
                return False
            gemini = GeminiClient()
            if not gemini.is_available():
                print("[!] Error: Gemini API key not configured or client offline.", file=sys.stderr)
                return False

        ocr_response = gemini.generate_json(prompt_text, image_path=png_path)
        page_type = "text_page"
        caption = None
        blocks_data = []

        if isinstance(ocr_response, dict):
            page_type = ocr_response.get("page_type", "text_page")
            caption = ocr_response.get("caption")
            blocks_data = ocr_response.get("blocks", [])
        elif isinstance(ocr_response, list):
            blocks_data = ocr_response
        else:
            print(f"  [!] Page {page_num:04d}: Gemini OCR did not return expected JSON, skipping")
            continue

        page_w = float(page_data.get("width", 612.0))
        page_h = float(page_data.get("height", 792.0))
        new_blocks = []

        if page_type == "blank":
            print(f"  [BLANK] Page {page_num:04d}: Classified as blank page -> 0 text blocks")
            page_data["blocks"] = []
            page_data["page_type"] = "blank"
        elif page_type == "pure_graphic":
            cap_str = f" ({caption})" if caption else ""
            print(f"  [GRAPHIC] Page {page_num:04d}: Classified as pure artwork/diagram{cap_str} -> Preserving as visual asset")
            page_data["blocks"] = []
            page_data["page_type"] = "pure_graphic"
            if caption:
                page_data["caption"] = caption
        else:
            for idx, item in enumerate(blocks_data):
                if not isinstance(item, dict):
                    continue
                text = item.get("text", "").strip()
                if not text:
                    continue

                x0, y0, x1, y1 = parse_ocr_bounding_box(item)

                bbox = [
                    round(x0 * page_w, 2),
                    round(y0 * page_h, 2),
                    round(x1 * page_w, 2),
                    round(y1 * page_h, 2),
                ]

                new_blocks.append({
                    "bbox": bbox,
                    "text": text + "\n",
                    "block_id": idx,
                    "type": 0,
                    "bbox_norm": [round(x0, 4), round(y0, 4), round(x1, 4), round(y1, 4)],
                })

            page_data["blocks"] = new_blocks
            page_data["page_type"] = "text_page"

        try:
            with open(jf, "w", encoding="utf-8") as f:
                json.dump(page_data, f, indent=2)
            scanned_count += 1
            total_ocr_blocks += len(new_blocks)
            if page_type == "text_page":
                print(f"  [+] Page {page_num:04d}: OCR populated {len(new_blocks)} text blocks -> Saved {jf.name}")
            else:
                print(f"  [+] Page {page_num:04d}: Marked as {page_type} -> Saved {jf.name}")
        except Exception as err:
            print(f"[!] Error writing {jf.name}: {err}", file=sys.stderr)

    print("\n---------------- Stage 01b Summary ----------------")
    print(f"Total Pages Inspected : {len(json_files)}")
    print(f"Digital Pass-Through  : {skipped_count}")
    print(f"Scanned Pages OCR'd   : {scanned_count}")
    print(f"Total OCR Blocks Added: {total_ocr_blocks}")
    print("---------------------------------------------------\n")
    return True


def main():
    parser = argparse.ArgumentParser(description="Stage 01b: Scan Detection & Gemini Vision OCR")
    parser.add_argument("--workspace", required=True, help="Path to pipeline workspace directory")
    parser.add_argument("--config", default="config.yaml", help="Path to config.yaml")
    parser.add_argument("--page-range", help="Page range to process (e.g. 1-10 or 1,3,5)")
    parser.add_argument("--start-page", type=int, help="Start page")
    parser.add_argument("--end-page", type=int, help="End page")
    parser.add_argument("--max-pages", type=int, help="Maximum pages to process")
    parser.add_argument("--force", action="store_true", help="Force OCR even on digital pages")
    parser.add_argument("--threshold", type=int, default=20, help="Character threshold for scan detection (default: 20)")
    args = parser.parse_args()

    workspace_dir = Path(args.workspace).resolve()
    config_path = Path(args.config).resolve()

    success = detect_and_ocr_pages(
        workspace_dir=workspace_dir,
        config_path=config_path,
        page_range=args.page_range,
        start_page=args.start_page,
        end_page=args.end_page,
        max_pages=args.max_pages,
        force=args.force,
        threshold=args.threshold,
    )
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()

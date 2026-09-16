#!/usr/bin/env python3
"""
stages/01_preprocess/detect_and_ocr.py:
Gemini Vision OCR helper module for Stage 01 (Preprocess).
Called by preprocess.py when scanned or text-deficient pages are detected:
- Scans pages lacking healthy text (total_chars < threshold or 0 text blocks).
- Uses Gemini Vision OCR to extract text blocks and normalized bounding boxes into page_XXXX.json.
"""

import json
import re
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


def parse_page_ranges(range_str: str) -> List[int]:
    if not range_str:
        return []
    pages = set()
    cleaned = re.sub(r"[,;]+", ",", range_str)
    for part in cleaned.split(","):
        part = part.strip()
        if not part:
            continue
        tokens = part.split() if not re.search(r"^\d+\s*(?:-|(?:\.\.)|:)\s*\d+$", part) else [part]
        for token in tokens:
            token = token.strip()
            if not token:
                continue
            range_match = re.match(r"^(\d+)\s*(?:-|(?:\.\.)|:)\s*(\d+)$", token)
            if range_match:
                s, e = int(range_match.group(1)), int(range_match.group(2))
                pages.update(range(min(s, e), max(s, e) + 1))
            elif token.isdigit():
                pages.add(int(token))
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
    config_path: Optional[Path] = None,
    page_ranges: Optional[str] = None,
    threshold: int = 20,
) -> bool:
    pages_dir = workspace_dir / "01_preprocess"

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
    target_pages = parse_page_ranges(page_ranges) if page_ranges else None

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
        is_scanned = len(blocks) == 0 or total_chars < threshold

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
            cfg_dict = {}
            if config_path and Path(config_path).is_file():
                try:
                    with open(config_path, "r", encoding="utf-8") as f:
                        cfg_dict = yaml.safe_load(f) or {}
                except Exception as e:
                    print(f"[!] Error loading config at {config_path}: {e}", file=sys.stderr)
            if not cfg_dict or "llm" not in cfg_dict:
                print(f"[!] Error: Valid configuration with 'llm' section required for OCR at {config_path}", file=sys.stderr)
                return False
            gemini = GeminiClient(cfg_dict)
            if not gemini.is_available():
                print("[!] Error: Gemini API key not configured or client offline.", file=sys.stderr)
                return False

        try:
            ocr_response = gemini.generate_json(prompt_text, image_path=png_path, stage="01_preprocess")
        except Exception as e:
            print(f"  [!] Page {page_num:04d}: Gemini OCR extraction failed: {e}. Preserving as visual fallback.", file=sys.stderr)
            page_data["blocks"] = []
            page_data["page_type"] = "pure_graphic"
            page_data["caption"] = f"Page {page_num} (Visual fallback)"
            with open(page_json_path, "w", encoding="utf-8") as f:
                json.dump(page_data, f, indent=2)
            continue

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

    # Refresh pages_manifest.json if any pages were modified by OCR
    if scanned_count > 0:
        manifest_path = workspace_dir / "pages_manifest.json"
        if manifest_path.exists():
            try:
                with open(manifest_path, "r", encoding="utf-8") as f:
                    manifest = json.load(f)
                for p_info in manifest.get("pages", []):
                    jf_path = workspace_dir / p_info.get("json_file", "")
                    if jf_path.exists():
                        with open(jf_path, "r", encoding="utf-8") as f:
                            p_data = json.load(f)
                        p_info["block_count"] = len(p_data.get("blocks", []))
                        if "page_type" in p_data:
                            p_info["page_type"] = p_data["page_type"]
                with open(manifest_path, "w", encoding="utf-8") as f:
                    json.dump(manifest, f, indent=2)
            except Exception:
                pass

    print("\n---------------- Stage 01 OCR Summary ----------------")
    print(f"Total Pages Inspected : {len(json_files)}")
    print(f"Digital Pass-Through  : {skipped_count}")
    print(f"Scanned Pages OCR'd   : {scanned_count}")
    print(f"Total OCR Blocks Added: {total_ocr_blocks}")
    print("------------------------------------------------------\n")
    return True

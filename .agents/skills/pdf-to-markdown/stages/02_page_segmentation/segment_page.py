#!/usr/bin/env python3
"""
stages/02_page_segmentation/segment_page.py:
Analyzes each page vertically from top to bottom.
Assigns bounding box coordinates and semantic types:
header, footer, heading, prose, code_block, table, graphic, toc, toc_header.
Emits workspace/segments/page_XXXX_segments.json.
"""

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Optional
import yaml

# Import GeminiClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

try:
    from llm_client import GeminiClient
except ImportError:
    GeminiClient = None


def classify_page_with_gemini(page_data: dict, png_path: Optional[Path], gemini: GeminiClient) -> list:
    """
    Uses Gemini Vision and semantic layout understanding to classify text blocks
    into precise semantic zones: header, footer, toc_header, toc, heading, prose, code_block, table, graphic.
    """
    page_num = page_data["page"]
    page_w = page_data.get("width", 612.0)
    page_h = page_data.get("height", 792.0)
    blocks = page_data.get("blocks", [])

    # Sort blocks top-to-bottom
    sorted_blocks = sorted(blocks, key=lambda b: (b["bbox"][1], b["bbox"][0]))

    # Prepare concise summary of blocks for LLM
    blocks_summary = []
    for i, b in enumerate(sorted_blocks):
        text = b.get("text", "").strip()
        if not text:
            continue
        bbox = b.get("bbox", [0, 0, 0, 0])
        bbox_norm = [
            round(bbox[0] / page_w, 4),
            round(bbox[1] / page_h, 4),
            round(bbox[2] / page_w, 4),
            round(bbox[3] / page_h, 4),
        ]
        blocks_summary.append({
            "idx": i,
            "bbox_norm": bbox_norm,
            "text": text[:200]
        })

    if not blocks_summary:
        # Check if this text-empty page contains a visual graphic (e.g. book cover, full-page illustration/schematic)
        if png_path and png_path.exists():
            empty_prompt_file = Path(__file__).resolve().parent / "prompt_empty_page.md"
            vision_prompt = empty_prompt_file.read_text(encoding="utf-8") if empty_prompt_file.exists() else ""
            res = gemini.generate_json(vision_prompt, image_path=png_path)
            if isinstance(res, dict) and not res.get("is_blank", False):
                g_bbox_norm = res.get("graphic_bbox_norm") or [0.0, 0.0, 1.0, 1.0]
                caption = res.get("caption") or f"Graphic on page {page_num}"
                bbox = [
                    round(g_bbox_norm[0] * page_w, 2),
                    round(g_bbox_norm[1] * page_h, 2),
                    round(g_bbox_norm[2] * page_w, 2),
                    round(g_bbox_norm[3] * page_h, 2),
                ]
                return [{
                    "segment_id": f"page_{page_num:04d}_seg_001",
                    "page": page_num,
                    "type": "graphic",
                    "bbox": bbox,
                    "bbox_norm": g_bbox_norm,
                    "heading_level": None,
                    "raw_text": ""
                }]
        return []

    prompt_file = Path(__file__).resolve().parent / "prompt.md"
    base_prompt = prompt_file.read_text(encoding="utf-8") if prompt_file.exists() else ""
    prompt = (
        f"{base_prompt}\n\n"
        f"Page {page_num} Text Blocks:\n"
        f"{json.dumps(blocks_summary, indent=2)}"
    )

    classifications = gemini.generate_json(prompt, image_path=png_path if png_path and png_path.exists() else None, stage="02_page_segmentation")
    type_map = {}
    
    # Check if Gemini flagged this entire page as a book cover or full-page illustration with overlaid text
    if isinstance(classifications, dict):
        if classifications.get("is_full_page_graphic", False):
            graphic_caption = classifications.get("graphic_caption") or "Book Cover Illustration"
            caption_text = f"Figure: {graphic_caption}" if not graphic_caption.lower().startswith("figure") else graphic_caption
            print(f"[*] Page {page_num}: Detected full-page cover graphic ('{graphic_caption}'). Suppressing individual text blocks.")
            return [{
                "segment_id": f"page_{page_num:04d}_seg_001",
                "page": page_num,
                "type": "graphic",
                "bbox": [0.0, 0.0, round(page_w, 2), round(page_h, 2)],
                "bbox_norm": [0.0, 0.0, 1.0, 1.0],
                "heading_level": None,
                "raw_text": caption_text
            }]
        items = classifications.get("segments", [])
    elif isinstance(classifications, list):
        items = classifications
    else:
        items = []

    for item in items:
        if isinstance(item, dict) and "idx" in item:
            type_map[item["idx"]] = (
                item.get("type", "prose"),
                item.get("heading_level"),
                item.get("graphic_bbox_norm")
            )

    segments = []
    seg_counter = 1
    for i, b in enumerate(sorted_blocks):
        text = b.get("text", "").strip()
        if not text:
            continue
        bbox = b.get("bbox", [0, 0, 0, 0])
        bbox_norm = b.get("bbox_norm", [
            round(bbox[0] / page_w, 4),
            round(bbox[1] / page_h, 4),
            round(bbox[2] / page_w, 4),
            round(bbox[3] / page_h, 4),
        ])

        seg_type, heading_lvl, g_bbox_norm = type_map.get(i, ("prose", None, None))

        if seg_type == "graphic" and g_bbox_norm and len(g_bbox_norm) == 4:
            # Expand bounding box to encompass the full visual diagram detected by Vision
            bbox_norm = [round(c, 4) for c in g_bbox_norm]
            bbox = [
                round(bbox_norm[0] * page_w, 2),
                round(bbox_norm[1] * page_h, 2),
                round(bbox_norm[2] * page_w, 2),
                round(bbox_norm[3] * page_h, 2),
            ]

        segments.append({
            "segment_id": f"page_{page_num:04d}_seg_{seg_counter:03d}",
            "page": page_num,
            "type": seg_type,
            "bbox": bbox,
            "bbox_norm": bbox_norm,
            "heading_level": heading_lvl,
            "raw_text": text
        })
        seg_counter += 1

    return segments


def process_segmentation(workspace_dir: Path, config: dict):
    pages_dir = workspace_dir / "01_preprocess"

    segments_dir = workspace_dir / "02_page_segmentation"
    segments_dir.mkdir(parents=True, exist_ok=True)
    for f in segments_dir.glob("*.json"):
        f.unlink()

    manifest_path = workspace_dir / "pages_manifest.json"
    if not manifest_path.exists():
        raise FileNotFoundError(f"Missing pages_manifest.json in {workspace_dir}")

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    total_pages = manifest["total_pages"]
    gemini = GeminiClient(config)

    from concurrent.futures import ThreadPoolExecutor, as_completed

    concurrency = int(config.get("llm", {}).get("concurrency", 8))
    print(f"[*] Vision LLM active ({gemini.vision_model}). Segmenting {total_pages} pages (concurrency={concurrency})...")

    def _process_page(page_entry):
        page_num = page_entry["page"]
        page_str = f"page_{page_num:04d}"
        json_file = workspace_dir / page_entry["json_file"]
        png_file = workspace_dir / page_entry.get("png_file", f"01_preprocess/{page_str}.png")

        if not json_file.exists():
            return page_num, None

        with open(json_file, "r", encoding="utf-8") as f:
            page_data = json.load(f)

        segments = classify_page_with_gemini(page_data, png_file, gemini)

        out_file = segments_dir / f"{page_str}_segments.json"
        with open(out_file, "w", encoding="utf-8") as f:
            json.dump({
                "page": page_num,
                "segments": segments
            }, f, indent=2)
        return page_num, len(segments)

    completed_count = 0
    with ThreadPoolExecutor(max_workers=concurrency) as executor:
        futures = {executor.submit(_process_page, entry): entry["page"] for entry in manifest["pages"]}
        for future in as_completed(futures):
            p_num = futures[future]
            try:
                page_num, seg_count = future.result()
                completed_count += 1
                if seg_count is not None:
                    print(f"    [+] Page {page_num:04d} segmented: {seg_count} zones ({completed_count}/{len(manifest['pages'])})")
            except Exception as e:
                print(f"[!] Error processing page {p_num}: {e}")
                raise e

    print(f"[+] Stage 02 complete. Segments written to {segments_dir}")


def main():
    parser = argparse.ArgumentParser(description="Stage 02: Page layout segmentation into semantic zones")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    if not config_path.is_file():
        raise FileNotFoundError(f"Stage 02: Config file not found: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)
    if not config or not isinstance(config, dict):
        raise ValueError(f"Stage 02: Config file is empty or invalid: {config_path}")

    process_segmentation(workspace_dir, config)


if __name__ == "__main__":
    main()

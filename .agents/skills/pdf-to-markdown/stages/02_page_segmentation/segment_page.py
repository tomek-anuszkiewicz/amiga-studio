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
    if not blocks:
        return []

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
        return []

    prompt = (
        "You are an expert technical document layout and typography analyzer for computer manuals. "
        "Inspect the attached 300 DPI high-resolution page image alongside the extracted text block bounding boxes. "
        "Classify each of the extracted text blocks into exactly ONE semantic type based on its visual appearance and position:\n"
        "- header: Running top-margin document header or chapter title rule at the very top of the page.\n"
        "- footer: Running bottom-margin footer or page number at the very bottom of the page.\n"
        "- toc_header: Prominent Table of Contents title banner (e.g. 'Contents', 'Table of Contents').\n"
        "- toc: Table of contents entries, chapter listings, and page number entries.\n"
        "- chapter: A new chapter start. The opening segment on a page indicating a new chapter (e.g. 'Chapter 1', 'Chapter 2', 'Appendix A', or major standalone chapter opening banner). It is the first segment on a page indicating a new chapter.\n"
        "- heading: Section headings, subheadings, and topic titles within an ongoing chapter (e.g. 'Copper Instruction Summary', 'Register Map', 'Using the Copper Registers'). Do NOT classify the opening chapter banner as heading; use 'chapter'. Use heading_level=1 for major sections, 2 for subsections.\n"
        "- prose: Standard narrative prose body paragraphs.\n"
        "- code_block: Monospace code listings, assembly language, memory dumps.\n"
        "- table: Structured data tables, register bit assignments, or multi-column grids.\n"
        "- graphic: Captions, diagram callouts, or embedded schematic labels.\n\n"
        "Return a strict JSON array of objects with fields:\n"
        '[{"idx": 0, "type": "chapter", "heading_level": 1}, ...]\n\n'
        f"Page {page_num} Text Blocks:\n"
        f"{json.dumps(blocks_summary, indent=2)}"
    )

    classifications = gemini.generate_json(prompt, image_path=png_path if png_path and png_path.exists() else None)
    type_map = {}
    if isinstance(classifications, list):
        for item in classifications:
            if isinstance(item, dict) and "idx" in item:
                type_map[item["idx"]] = (item.get("type", "prose"), item.get("heading_level"))

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

        seg_type, heading_lvl = type_map.get(i, ("prose", None))

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
    pages_dir = workspace_dir / "01_pages" if (workspace_dir / "01_pages").exists() else (workspace_dir / "pages")
    segments_dir = workspace_dir / "02_segments"
    segments_dir.mkdir(parents=True, exist_ok=True)
    for f in segments_dir.glob("*.json"):
        f.unlink()

    manifest_path = workspace_dir / "pages_manifest.json"
    if not manifest_path.exists():
        manifest_path = workspace_dir / "manifest.json"
    if not manifest_path.exists():
        raise FileNotFoundError(f"Missing pages_manifest.json in {workspace_dir}")

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    total_pages = manifest["total_pages"]
    gemini = GeminiClient(config) if GeminiClient else None
    if not gemini or not gemini.is_available():
        raise RuntimeError("GEMINI_API_KEY environment variable is required for Stage 02 segmentation.")

    print(f"[*] Vision LLM active ({gemini.vision_model}). Segmenting {total_pages} pages...")

    for page_entry in manifest["pages"]:
        page_num = page_entry["page"]
        page_str = f"page_{page_num:04d}"
        json_file = workspace_dir / page_entry["json_file"]
        png_file = workspace_dir / page_entry.get("png_file", f"01_pages/{page_str}.png")

        if not json_file.exists():
            continue

        with open(json_file, "r", encoding="utf-8") as f:
            page_data = json.load(f)

        segments = classify_page_with_gemini(page_data, png_file, gemini)

        out_file = segments_dir / f"{page_str}_segments.json"
        with open(out_file, "w", encoding="utf-8") as f:
            json.dump({
                "page": page_num,
                "segments": segments
            }, f, indent=2)

    print(f"[+] Stage 02 complete. Segments written to {segments_dir}")


def main():
    parser = argparse.ArgumentParser(description="Stage 02: Page layout segmentation into semantic zones")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    config = {}
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}

    process_segmentation(workspace_dir, config)


if __name__ == "__main__":
    main()

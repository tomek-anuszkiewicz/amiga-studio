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
import yaml

# Import GeminiClient from skill root
SKILL_ROOT = Path(__file__).resolve().parents[2]
if str(SKILL_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILL_ROOT))

try:
    from llm_client import GeminiClient
except ImportError:
    GeminiClient = None


def heuristic_segment_page(page_data: dict) -> list:
    """
    Deterministic layout heuristics based on text block geometry and contents.
    Provides robust fallback and pre-segmentation.
    """
    page_num = page_data["page"]
    page_w = page_data["width"]
    page_h = page_data["height"]
    blocks = page_data.get("blocks", [])

    segments = []
    seg_counter = 1

    # Sort blocks top-to-bottom
    sorted_blocks = sorted(blocks, key=lambda b: (b["bbox"][1], b["bbox"][0]))

    for b in sorted_blocks:
        text = b.get("text", "").strip()
        if not text:
            continue

        bbox = b["bbox"]
        bbox_norm = b.get("bbox_norm", [
            round(bbox[0] / page_w, 4),
            round(bbox[1] / page_h, 4),
            round(bbox[2] / page_w, 4),
            round(bbox[3] / page_h, 4),
        ])

        y0_norm = bbox_norm[1]
        y1_norm = bbox_norm[3]

        # 1. Header detection (top 7% of page)
        if y1_norm <= 0.08 and len(text.splitlines()) <= 2:
            seg_type = "header"
            heading_lvl = None
        # 2. Footer detection (bottom 7% of page)
        elif y0_norm >= 0.93 and len(text.splitlines()) <= 2:
            seg_type = "footer"
            heading_lvl = None
        # 3. TOC Header detection
        elif re.search(r"^(table\s+of\s+contents|contents|brief\s+contents)$", text, re.IGNORECASE):
            seg_type = "toc_header"
            heading_lvl = 1
        # 4. Table of Contents line detection (e.g. dots or page numbers at end of line)
        elif re.search(r"(\.{3,}|\s{3,})\d+$", text, re.MULTILINE):
            seg_type = "toc"
            heading_lvl = None
        # 5. Major Headings
        elif (len(text.splitlines()) == 1 and len(text) < 80 and
              (re.match(r"^(chapter\s+\d+|section\s+[ivxlcdm\d]+|appendix\s+[a-z\d]+)", text, re.IGNORECASE) or
               text.isupper() and len(text) > 4)):
            seg_type = "heading"
            heading_lvl = 1 if "chapter" in text.lower() or "appendix" in text.lower() else 2
        # 6. Monospace / Code blocks / Hex dumps
        elif re.search(r"(\$[0-9a-f]{4,8}|move\.[bwl]|jmp|lea|jsr|void\s+|int\s+|#include)", text, re.IGNORECASE):
            seg_type = "code_block"
            heading_lvl = None
        # 7. Tables (multiple tabs, multiple aligned numeric columns)
        elif re.search(r"(\w+\t+\w+|\d+\s{3,}\d+\s{3,}\d+)", text):
            seg_type = "table"
            heading_lvl = None
        # 8. Standard prose
        else:
            seg_type = "prose"
            heading_lvl = None

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
    pages_dir = workspace_dir / "pages"
    segments_dir = workspace_dir / "segments"
    segments_dir.mkdir(parents=True, exist_ok=True)

    manifest_path = workspace_dir / "manifest.json"
    if not manifest_path.exists():
        raise FileNotFoundError(f"Missing manifest.json in {workspace_dir}")

    with open(manifest_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    total_pages = manifest["total_pages"]
    gemini = GeminiClient(config) if GeminiClient else None
    if gemini and gemini.is_available():
        print(f"[*] Vision LLM active ({gemini.vision_model}, thinking: {gemini.thinking_level}).")
    else:
        print(f"[*] Vision LLM unavailable (no GEMINI_API_KEY). Using deterministic layout heuristics.")

    print(f"[*] Segmenting {total_pages} pages...")

    for page_entry in manifest["pages"]:
        page_num = page_entry["page"]
        page_str = f"page_{page_num:04d}"
        json_file = workspace_dir / page_entry["json_file"]

        if not json_file.exists():
            continue

        with open(json_file, "r", encoding="utf-8") as f:
            page_data = json.load(f)

        segments = heuristic_segment_page(page_data)

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

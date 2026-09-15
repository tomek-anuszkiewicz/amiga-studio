#!/usr/bin/env python3
"""
stages/03_build_raw_stream/build_stream.py:
Merges segmented pages into a global, flat, sequentially ordered raw_stream.json.
Calls extract_initial_assets to crop SVGs/PNGs and extract raw text within bounding boxes.
"""

import argparse
import json
import os
import sys
from pathlib import Path
import yaml

CURRENT_DIR = Path(__file__).resolve().parent
if str(CURRENT_DIR) not in sys.path:
    sys.path.insert(0, str(CURRENT_DIR))

from extract_initial_assets import extract_assets_for_nodes


def build_raw_stream(workspace_dir: Path, config: dict):
    segments_dir = workspace_dir / "02_segments" if (workspace_dir / "02_segments").exists() else (workspace_dir / "segments")
    if not segments_dir.exists():
        raise FileNotFoundError(f"Segments directory not found: {segments_dir}")

    segment_files = sorted(list(segments_dir.glob("page_*_segments.json")))
    if not segment_files:
        raise FileNotFoundError(f"No segment files found in {segments_dir}")

    print(f"[*] Aggregating {len(segment_files)} segmented pages into raw stream...")

    all_nodes = []
    global_node_counter = 1

    for s_file in segment_files:
        with open(s_file, "r", encoding="utf-8") as f:
            data = json.load(f)

        page_num = data.get("page", 1)
        segments = data.get("segments", [])

        # Sort segments top to bottom, then left to right
        sorted_segs = sorted(segments, key=lambda s: (s.get("bbox", [0, 0, 0, 0])[1], s.get("bbox", [0, 0, 0, 0])[0]))

        for seg in sorted_segs:
            node_id = f"node_{global_node_counter:05d}"
            node = {
                "node_id": node_id,
                "page": page_num,
                "type": seg.get("type", "prose"),
                "heading_level": seg.get("heading_level"),
                "bbox": seg.get("bbox", []),
                "bbox_norm": seg.get("bbox_norm", []),
                "raw_text": seg.get("raw_text", ""),
                "rendered_markdown": None,
                "continuation_status": None,
                "metadata": {}
            }
            all_nodes.append(node)
            global_node_counter += 1

    padding = config.get("render", {}).get("padding_margin_ratio", 0.10)
    all_nodes = extract_assets_for_nodes(workspace_dir, all_nodes, padding_ratio=padding)

    raw_dir = workspace_dir / "03_raw_stream"
    raw_dir.mkdir(parents=True, exist_ok=True)
    raw_stream_path = raw_dir / "raw_stream.json"
    with open(raw_stream_path, "w", encoding="utf-8") as f:
        json.dump(all_nodes, f, indent=2)

    print(f"[+] Stage 03 complete. {len(all_nodes)} nodes written to {raw_stream_path}")


def main():
    parser = argparse.ArgumentParser(description="Stage 03: Build raw sequential node stream and extract initial assets")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    config = {}
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            config = yaml.safe_load(f) or {}

    build_raw_stream(workspace_dir, config)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""
stages/03_build_raw_stream/build_stream.py:
Assemble Stage 02.81 objects into the downstream stream in reading order.
"""

import argparse
import json
import sys
from pathlib import Path
import yaml

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from common.pdf_page_conversion import read_conversion
from common.pdf_tables import STAGE, FIELDS, ASSOCIATED, converted

sys.path.insert(0, str(Path(__file__).resolve().parent))
from extract_initial_assets import extract_assets_for_nodes


def build_raw_stream(workspace_dir: Path, config: dict):
    pages = read_conversion(workspace_dir, STAGE)
    nodes = []
    for entry, value in pages:
        width, height = entry["width"], entry["height"]
        pixel_width, pixel_height = value["image_width"], value["image_height"]
        group_ids = {identity for table in value["segments"] if table["type"] == "table"
                     for identity in table.get("table_group_segment_ids", [])}
        for segment in value["segments"]:
            x0, y0, x1, y1 = segment["bbox"]
            normalized = [x0 / pixel_width, y0 / pixel_height,
                          x1 / pixel_width, y1 / pixel_height]
            # Legacy consumers use displayed-page points and scale them by
            # the actual PNG dimensions. Preserve the exact pixel box too.
            nodes.append({
                "node_id": f"node_{len(nodes)+1:05d}", "page": entry["page"],
                "segment_id": segment["segment_id"], "type": segment["type"],
                "heading_level": segment["heading_level"],
                "continuation": segment["continuation"],
                "md_text": segment["md_text"], "raw_text": segment["md_text"],
                "bbox_pixels": list(segment["bbox"]), "bbox_norm": normalized,
                "bbox": [normalized[0] * width, normalized[1] * height,
                         normalized[2] * width, normalized[3] * height],
                "rendered_markdown": segment["md_text"] if converted(segment) or (
                    segment["type"] in ASSOCIATED and segment["segment_id"] in group_ids) else None,
                "continuation_status": None,
                **{key: segment[key] for key in FIELDS if key in segment},
                "metadata": {key: segment[key] for key in ("source_segment_ids", "replacement_stage")
                             if key in segment},
            })
    output = workspace_dir / "03_build_raw_stream"
    assets = output / "assets"
    assets.mkdir(parents=True, exist_ok=True)
    for old_file in assets.glob("*"):
        if old_file.is_file():
            old_file.unlink()
    nodes = extract_assets_for_nodes(workspace_dir, nodes, assets_dir=assets,
                                    dpi=config.get("render", {}).get("dpi", 300))
    for node in nodes:
        if node["type"] == "cover":
            node["rendered_markdown"] = f"![Cover](assets/{Path(node['png_path']).name})"
    (output / "raw_stream.json").write_text(json.dumps(nodes, indent=2), encoding="utf-8")
    print(f"[+] Stage 03 complete: {len(nodes)} nodes from Stage 02.81 objects")


def main():
    parser = argparse.ArgumentParser(description="Stage 03: Build raw sequential node stream and extract initial assets")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)

    build_raw_stream(workspace_dir, config)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""
stages/03_build_raw_stream/build_stream.py:
Assemble validated Stage 02 objects into the downstream stream in reading order.
"""

import argparse
import json
import sys
from pathlib import Path
import yaml

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from conversion.config import UniqueLoader
from conversion.pdf_page_conversion import validate_conversion

sys.path.insert(0, str(Path(__file__).resolve().parent))
from extract_initial_assets import extract_assets_for_nodes



def build_raw_stream(workspace_dir: Path, config: dict):
    pages = validate_conversion(workspace_dir)
    nodes = []
    for entry, value in pages:
        width, height = entry["width"], entry["height"]
        pixel_width, pixel_height = value["image_width"], value["image_height"]
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
                "rendered_markdown": None, "continuation_status": None,
                "metadata": {},
            })
    output = workspace_dir / "03_build_raw_stream"
    assets = output / "assets"
    assets.mkdir(parents=True, exist_ok=True)
    for old_file in assets.glob("asset_*"):
        if old_file.is_file():
            old_file.unlink()
    nodes = extract_assets_for_nodes(workspace_dir, nodes, assets_dir=assets,
                                    dpi=config.get("render", {}).get("dpi", 300))
    (output / "raw_stream.json").write_text(json.dumps(nodes, indent=2), encoding="utf-8")
    print(f"[+] Stage 03 complete: {len(nodes)} nodes from validated Stage 02 objects")


def main():
    parser = argparse.ArgumentParser(description="Stage 03: Build raw sequential node stream and extract initial assets")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    config_path = Path(args.config)
    if not config_path.is_file():
        raise FileNotFoundError(f"Stage 03: Config file not found: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.load(f, Loader=UniqueLoader)
    if not config or not isinstance(config, dict):
        raise ValueError(f"Stage 03: Config file is empty or invalid: {config_path}")

    build_raw_stream(workspace_dir, config)


if __name__ == "__main__":
    main()

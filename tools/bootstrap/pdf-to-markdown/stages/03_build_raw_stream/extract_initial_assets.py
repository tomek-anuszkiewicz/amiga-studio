#!/usr/bin/env python3
"""
stages/03_build_raw_stream/extract_initial_assets.py:
For every table and graphic node:
1. Crops PNG assets from Stage 01 page renders with typographic padding.
2. Annotates the node with visual file paths in raw_stream.json.
Raw text remains in each node's raw_text field; no separate text dumps are written.
"""

import json
import math
from pathlib import Path
from PIL import Image


def extract_assets_for_nodes(
    workspace_dir: Path,
    nodes: list,
    assets_dir: Path = None,
    rel_prefix: str = None,
    dpi: int = 300,
) -> list:
    if assets_dir is None:
        assets_dir = workspace_dir / "03_build_raw_stream" / "assets"
    if rel_prefix is None:
        try:
            rel_prefix = assets_dir.relative_to(workspace_dir).as_posix()
        except Exception:
            rel_prefix = assets_dir.name

    pages_dir = workspace_dir / "01_preprocess"
    assets_dir.mkdir(parents=True, exist_ok=True)

    # Typographic padding derivation:
    # Standard book body text font size is 10 pt in standard 72 pt/inch PostScript space.
    # Letter height in inches = 10.0 / 72.0 inches.
    # Half-letter height padding in pixels at configured DPI:
    letter_height_inches = 10.0 / 72.0
    padding_px = round(0.5 * letter_height_inches * dpi)  # 21 pixels at 300 DPI
    padding_pt = padding_px * 72.0 / dpi                   # ~5.04 points at 300 DPI

    print(f"[*] Extracting visual assets for tables & graphics (typographic padding: {padding_pt:.2f} pt / {padding_px} px at {dpi} DPI)...")

    # Group nodes by page so each source PNG is opened once.
    nodes_by_page = {}
    for node in nodes:
        if node["type"] in ("table", "graphic", "code_block"):
            p = node["page"]
            nodes_by_page.setdefault(p, []).append(node)

    for page_num, page_nodes in nodes_by_page.items():
        page_str = f"page_{page_num:04d}"
        json_page_path = pages_dir / f"{page_str}.json"
        png_page_path = pages_dir / f"{page_str}.png"
        page_data = json.loads(json_page_path.read_text(encoding="utf-8"))
        page_width = page_data["width"]
        page_height = page_data["height"]

        with Image.open(png_page_path) as page_image:
            pixel_width, pixel_height = page_image.size
            scale_x = pixel_width / page_width
            scale_y = pixel_height / page_height
            for node in page_nodes:
                node_id = node["node_id"]
                bbox = node["bbox"]  # [x0, y0, x1, y1] in page points
                padded_bbox = [
                    max(0.0, bbox[0] - padding_pt),
                    max(0.0, bbox[1] - padding_pt),
                    min(page_width, bbox[2] + padding_pt),
                    min(page_height, bbox[3] + padding_pt),
                ]

                # Preserve the vertical padding limits imposed by neighboring nodes.
                other_nodes = [n for n in nodes if n.get("page") == page_num and n.get("node_id") != node_id and n.get("bbox")]
                for other in other_nodes:
                    ob = other["bbox"]
                    if ob[1] >= bbox[3] - 1.0:
                        padded_bbox[3] = min(padded_bbox[3], max(bbox[3], ob[1] - 1.0))
                    if ob[3] <= bbox[1] + 1.0:
                        padded_bbox[1] = max(padded_bbox[1], min(bbox[1], ob[3] + 1.0))

                # Round outward and clamp to the actual rendered pixel dimensions.
                pixel_bbox = (
                    max(0, math.floor(padded_bbox[0] * scale_x)),
                    max(0, math.floor(padded_bbox[1] * scale_y)),
                    min(pixel_width, math.ceil(padded_bbox[2] * scale_x)),
                    min(pixel_height, math.ceil(padded_bbox[3] * scale_y)),
                )
                png_filename = f"asset_{node_id}.png"
                with page_image.crop(pixel_bbox) as cropped_image:
                    cropped_image.save(assets_dir / png_filename)
                node["png_path"] = f"{rel_prefix}/{png_filename}"

    print(f"[+] Initial assets extracted to {assets_dir}")
    return nodes

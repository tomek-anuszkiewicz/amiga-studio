#!/usr/bin/env python3
"""
stages/03_build_raw_stream/extract_initial_assets.py:
For every table and graphic node:
1. Crops visual assets (SVG vector clip if available, or 300 DPI raster PNG with a 10% safety margin).
2. Extracts all underlying PDF text within the bounding box area into workspace/assets/asset_{node_id}.txt.
3. Annotates the node with file paths in raw_stream.json.
"""

import json
from pathlib import Path
import pymupdf
from PIL import Image


def extract_assets_for_nodes(workspace_dir: Path, nodes: list, padding_ratio: float = 0.10, assets_dir: Path = None, rel_prefix: str = None) -> list:
    if assets_dir is None:
        assets_dir = workspace_dir / "03_raw_stream" / "assets"
    if rel_prefix is None:
        try:
            rel_prefix = assets_dir.relative_to(workspace_dir).as_posix()
        except Exception:
            rel_prefix = assets_dir.name

    pages_dir = workspace_dir / "01_pages" if (workspace_dir / "01_pages").exists() else (workspace_dir / "pages")
    assets_dir.mkdir(parents=True, exist_ok=True)

    print(f"[*] Extracting visual and text assets for tables & graphics (padding: {int(padding_ratio*100)}%)...")

    # Group nodes by page for efficient PDF access
    nodes_by_page = {}
    for node in nodes:
        if node["type"] in ("table", "graphic"):
            p = node["page"]
            nodes_by_page.setdefault(p, []).append(node)

    for page_num, page_nodes in nodes_by_page.items():
        page_str = f"page_{page_num:04d}"
        pdf_page_path = pages_dir / f"{page_str}.pdf"
        png_page_path = pages_dir / f"{page_str}.png"

        doc = pymupdf.open(str(pdf_page_path)) if pdf_page_path.exists() else None
        page = doc[0] if doc else None
        page_pixmap = None
        if png_page_path.exists():
            try:
                page_pixmap = Image.open(png_page_path)
            except Exception:
                page_pixmap = None

        for node in page_nodes:
            node_id = node["node_id"]
            bbox = node.get("bbox", [0, 0, 100, 100]) # [x0, y0, x1, y1] in points

            # Calculate padded bounding box
            w = bbox[2] - bbox[0]
            h = bbox[3] - bbox[1]
            pad_x = w * padding_ratio
            pad_y = h * padding_ratio

            padded_bbox = [
                max(0.0, bbox[0] - pad_x),
                max(0.0, bbox[1] - pad_y),
                (page.rect.width if page else bbox[2] + pad_x),
                (page.rect.height if page else bbox[3] + pad_y),
            ]
            padded_bbox[2] = min(page.rect.width if page else 9999.0, bbox[2] + pad_x)
            padded_bbox[3] = min(page.rect.height if page else 9999.0, bbox[3] + pad_y)

            # 1. Extract Raw PDF Text Asset within the bounding box
            txt_filename = f"asset_{node_id}.txt"
            txt_path = assets_dir / txt_filename
            extracted_text = ""
            if page:
                clip_rect = pymupdf.Rect(*padded_bbox)
                extracted_text = page.get_text("text", clip=clip_rect)
            with open(txt_path, "w", encoding="utf-8") as f:
                f.write(extracted_text.strip() + "\n")
            node["raw_text_path"] = f"{rel_prefix}/{txt_filename}"

            # 2. Extract Vector SVG Clip if supported
            svg_filename = f"asset_{node_id}.svg"
            svg_path = assets_dir / svg_filename
            has_vector = False
            if page:
                try:
                    clip_rect = pymupdf.Rect(*padded_bbox)
                    svg_data = page.get_svg_image(clip=clip_rect)
                    if svg_data and len(svg_data) > 200:
                        with open(svg_path, "w", encoding="utf-8") as f:
                            f.write(svg_data)
                        node["svg_path"] = f"{rel_prefix}/{svg_filename}"
                        has_vector = True
                except Exception:
                    has_vector = False

            # 3. Extract Raster PNG Clip (at 300 DPI) with 10% safety margin
            png_filename = f"asset_{node_id}.png"
            png_path = assets_dir / png_filename
            if page:
                clip_rect = pymupdf.Rect(*padded_bbox)
                clip_pix = page.get_pixmap(dpi=300, clip=clip_rect)
                clip_pix.save(str(png_path))
                node["png_path"] = f"{rel_prefix}/{png_filename}"
            elif page_pixmap:
                # Fallback to cropping raster image
                pw, ph = page_pixmap.size
                norm = node.get("bbox_norm", [0, 0, 1, 1])
                crop_box = (
                    int(max(0.0, norm[0] - padding_ratio) * pw),
                    int(max(0.0, norm[1] - padding_ratio) * ph),
                    int(min(1.0, norm[2] + padding_ratio) * pw),
                    int(min(1.0, norm[3] + padding_ratio) * ph),
                )
                cropped_img = page_pixmap.crop(crop_box)
                cropped_img.save(png_path)
                node["png_path"] = f"{rel_prefix}/{png_filename}"

        if doc:
            doc.close()

    print(f"[+] Initial assets extracted to {assets_dir}")
    return nodes

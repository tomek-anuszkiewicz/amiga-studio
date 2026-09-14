#!/usr/bin/env python3
"""
extract_crops.py - Bounding-Box Figure Cropper, Asset Deduplicator, & Link Injector

Scans Markdown text for `<crop xmin=".." ymin=".." xmax=".." ymax=".." label=".." />`
tags, applies safety padding margins to prevent clipped borders and pin labels,
crops diagrams from page PNGs, deduplicates identical figures across chapters,
and replaces `<crop>` tags with standard Obsidian Markdown image links.
"""

import os
import sys
import re
import json
import hashlib
import argparse
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Dict, Tuple, Optional

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8', errors='backslashreplace')
if hasattr(sys.stderr, 'reconfigure'):
    sys.stderr.reconfigure(encoding='utf-8', errors='backslashreplace')

try:
    from PIL import Image
except ImportError:
    print("Error: Pillow (PIL) is required. Run 'pip install pillow'.")
    sys.exit(1)

CROP_PATTERN = re.compile(r'<crop\s+([^>]+)/>', re.IGNORECASE)

def slugify(text: str) -> str:
    """Creates a clean, lowercase, alphanumeric filename slug."""
    s = text.lower()
    s = re.sub(r'<[^>]+>', '', s)
    s = re.sub(r'[^\w\s\-]', '', s)
    s = re.sub(r'[\s\-]+', '_', s)
    return s.strip('_')

def compute_image_hash(img: Image.Image) -> str:
    """Computes SHA-256 hash of image pixel bytes to detect identical figures."""
    # Resize to standard size for content hashing to catch minor scan differences
    resized = img.convert('L').resize((64, 64), Image.Resampling.BILINEAR)
    return hashlib.sha256(resized.tobytes()).hexdigest()

def ensure_asset_sidecar(asset_path: Path, description: str) -> None:
    """Ensures a Git-tracked technical sidecar (<image>.txt) exists for the asset."""
    sidecar_path = asset_path.with_name(asset_path.name + ".txt")
    if not sidecar_path.is_file():
        content = (
            f"Technical description for {asset_path.name}:\n\n"
            f"{description.strip()}\n"
        )
        sidecar_path.write_text(content, encoding="utf-8")
        print(f"    Created sidecar: {sidecar_path.name}")

def extract_crops_from_markdown(
    md_dir: Path,
    pages_dir: Path,
    assets_dir: Path,
    padding: int = 15
) -> Tuple[int, int]:
    """
    Extracts figure crops from page PNGs based on <crop> tags in Markdown files,
    deduplicates identical images, and replaces tags with standard Markdown links.
    """
    assets_dir.mkdir(parents=True, exist_ok=True)
    
    md_files = sorted(list(md_dir.glob("*.md")))
    total_crops = 0
    deduped_crops = 0
    seen_hashes: Dict[str, str] = {} # hash -> canonical_filename

    print(f"Scanning {len(md_files)} markdown files in {md_dir} for <crop> tags...")

    for md_path in md_files:
        content = md_path.read_text(encoding="utf-8")
        if '<crop' not in content.lower():
            continue

        # Extract page number from filename (e.g. 'page_017.md' -> 17)
        m_page = re.search(r'(\d+)', md_path.stem)
        page_num = int(m_page.group(1)) if m_page else 1
        page_png_name = f"page_{page_num:03d}.png"
        page_png_path = pages_dir / page_png_name

        page_img = None
        if page_png_path.exists():
            page_img = Image.open(page_png_path)
        else:
            print(f"Warning: Page image '{page_png_path}' not found for {md_path.name}")

        def replace_crop(match):
            nonlocal total_crops, deduped_crops
            total_crops += 1

            xml_snippet = match.group(0)
            try:
                # Wrap in root element for robust XML parsing
                elem = ET.fromstring(xml_snippet)
                attrs = elem.attrib
            except Exception as e:
                # Fallback regex parsing of attributes
                attrs = dict(re.findall(r'(\w+)=["\']([^"\']+)["\']', xml_snippet))

            xmin = int(attrs.get('xmin', 0))
            ymin = int(attrs.get('ymin', 0))
            xmax = int(attrs.get('xmax', 0))
            ymax = int(attrs.get('ymax', 0))
            label = attrs.get('label', f'Figure on Page {page_num}')
            
            # Figure ID or section prefix for naming
            m_fig = re.search(r'figure\s*(\d+[\-\.]\d+)', label, re.IGNORECASE)
            fig_num = m_fig.group(1).replace('.', '-') if m_fig else f"p{page_num}_{total_crops}"
            slug = slugify(label)
            if len(slug) > 40:
                slug = slug[:40].rstrip('_')

            asset_name = f"figure_{fig_num}_{slug}.png"
            asset_path = assets_dir / asset_name

            if page_img is not None:
                # Apply safety padding margin
                w, h = page_img.size
                pad_xmin = max(0, xmin - padding)
                pad_ymin = max(0, ymin - padding)
                pad_xmax = min(w, xmax + padding)
                pad_ymax = min(h, ymax + padding)

                crop_box = (pad_xmin, pad_ymin, pad_xmax, pad_ymax)
                cropped_img = page_img.crop(crop_box)

                # Check for duplicate image
                img_hash = compute_image_hash(cropped_img)
                if img_hash in seen_hashes:
                    # Reuse canonical filename
                    canonical_name = seen_hashes[img_hash]
                    deduped_crops += 1
                    print(f"  Deduplicated: '{label}' -> reuses '{canonical_name}'")
                    ensure_asset_sidecar(assets_dir / canonical_name, label)
                    return f"![{label}](assets/{canonical_name})"
                else:
                    seen_hashes[img_hash] = asset_name
                    cropped_img.save(asset_path, "PNG")
                    ensure_asset_sidecar(asset_path, label)
                    print(f"  Cropped: {asset_name} ({cropped_img.size[0]}x{cropped_img.size[1]})")

            return f"![{label}](assets/{asset_name})"

        new_content = CROP_PATTERN.sub(replace_crop, content)
        if new_content != content:
            md_path.write_text(new_content, encoding="utf-8")

    print(f"\nCrop Extraction Complete:")
    print(f"  Total crops processed: {total_crops}")
    print(f"  Deduplicated images:   {deduped_crops}")
    print(f"  Assets saved to:       {assets_dir}")
    return total_crops, deduped_crops

def main():
    parser = argparse.ArgumentParser(description="Extract figure crops from page PNGs and replace tags.")
    parser.add_argument("--markdown-dir", "-m", type=str, required=True, help="Directory containing page markdown files")
    parser.add_argument("--pages-dir", "-p", type=str, required=True, help="Directory containing page PNG images")
    parser.add_argument("--assets-dir", "-a", type=str, required=True, help="Target assets directory")
    parser.add_argument("--padding", type=int, default=15, help="Safety padding margin in pixels (default: 15)")
    args = parser.parse_args()

    md_dir = Path(args.markdown_dir)
    pages_dir = Path(args.pages_dir)
    assets_dir = Path(args.assets_dir)

    if not md_dir.is_dir() or not pages_dir.is_dir():
        print("Error: markdown-dir and pages-dir must be valid directories.")
        sys.exit(1)

    extract_crops_from_markdown(md_dir, pages_dir, assets_dir, padding=args.padding)

if __name__ == "__main__":
    main()

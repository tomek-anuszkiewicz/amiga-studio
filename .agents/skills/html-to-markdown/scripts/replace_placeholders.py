#!/usr/bin/env python3
"""
replace_placeholders.py - Image Placeholder Resolver & Asset Link Injector

Scans Markdown documents for image placeholders:
  1. `<image placeholder src="..." alt="..." />`
  2. `<crop page="N" xmin=".." ymin=".." xmax=".." ymax=".." label=".." />`

Resolves source assets or crops from rendered page PNGs, transfers them to
a standardized assets directory, guarantees technical sidecars (.txt) exist,
and replaces placeholder tags with canonical Markdown image links:
  `![alt](assets/figure_XX_name.ext)`
"""

import os
import sys
import re
import shutil
import hashlib
import argparse
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Dict, Tuple, Optional, List

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8', errors='backslashreplace')
if hasattr(sys.stderr, 'reconfigure'):
    sys.stderr.reconfigure(encoding='utf-8', errors='backslashreplace')

try:
    from PIL import Image
except ImportError:
    Image = None

# Regex patterns for placeholders
PLACEHOLDER_IMG_PATTERN = re.compile(
    r'<(?:image|img)\s+[^>]*placeholder[^>]*\/?>',
    re.IGNORECASE
)
CROP_PATTERN = re.compile(r'<crop\s+[^>]+\/?>', re.IGNORECASE)


def make_clean_identifier(text: str) -> str:
    """Creates a clean, lowercase, alphanumeric filename identifier."""
    s = text.lower()
    s = re.sub(r'<[^>]+>', '', s)
    s = re.sub(r'[^\w\s\-]', '', s)
    s = re.sub(r'[\s\-]+', '_', s)
    return s.strip('_')


def compute_file_hash(path: Path) -> str:
    """Computes SHA-256 hash of a file."""
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def ensure_asset_sidecar(asset_path: Path, description: str):
    """Ensures a Git-tracked technical sidecar (<image>.txt) exists for the asset."""
    sidecar_path = asset_path.with_name(asset_path.name + ".txt")
    if not sidecar_path.is_file():
        content = (
            f"Asset: {asset_path.name}\n"
            f"Description: {description.strip()}\n"
            f"Source: Extracted during HTML/PDF-to-Markdown reference conversion.\n"
        )
        sidecar_path.write_text(content, encoding="utf-8")
        print(f"    Created sidecar: {sidecar_path.name}")


def parse_tag_attributes(tag_str: str) -> Dict[str, str]:
    """Extracts key-value attributes from an XML/HTML tag."""
    try:
        closed = tag_str if tag_str.rstrip().endswith("/>") else tag_str.rstrip().rstrip(">") + " />"
        elem = ET.fromstring(closed)
        return elem.attrib
    except Exception:
        attrs = {}
        for m in re.finditer(r'(\w+)=["\']([^"\']*)["\']', tag_str):
            attrs[m.group(1).lower()] = m.group(2)
        return attrs


def resolve_source_asset(src_str: str, search_dirs: List[Path]) -> Optional[Path]:
    """Finds a source asset path across candidate search directories."""
    clean_src = Path(src_str.strip().replace('\\', '/'))

    for d in search_dirs:
        cand = d / clean_src
        if cand.is_file():
            return cand
        cand_name = d / clean_src.name
        if cand_name.is_file():
            return cand_name
        if d.is_dir():
            matches = list(d.rglob(clean_src.name))
            if matches:
                return matches[0]

    return None


def process_markdown_content(
    content: str,
    md_file: Path,
    assets_dir: Path,
    source_dirs: List[Path],
    pages_dir: Optional[Path],
    padding: int = 15,
) -> Tuple[str, int]:
    """Replaces placeholders in markdown text with standard links."""
    assets_dir.mkdir(parents=True, exist_ok=True)
    replacements_count = 0
    seen_hashes: Dict[str, str] = {}

    def replace_img_placeholder(match: re.Match) -> str:
        nonlocal replacements_count
        tag_str = match.group(0)
        attrs = parse_tag_attributes(tag_str)

        src = attrs.get('src', '')
        alt = attrs.get('alt', attrs.get('label', 'Figure'))
        if not src:
            return tag_str

        found_src = resolve_source_asset(src, source_dirs)
        if not found_src:
            print(f"  Warning: Could not locate source asset for '{src}'")
            return f"![{alt}]({src})"

        file_hash = compute_file_hash(found_src)
        ext = found_src.suffix.lower()

        ident = make_clean_identifier(alt)
        if len(ident) > 40:
            ident = ident[:40].rstrip('_')
        if not ident:
            ident = found_src.stem

        if file_hash in seen_hashes:
            canonical_name = seen_hashes[file_hash]
            print(f"  Deduplicated asset: '{alt}' -> reuses '{canonical_name}'")
            replacements_count += 1
            return f"![{alt}](assets/{canonical_name})"

        asset_name = f"{ident}{ext}"
        target_path = assets_dir / asset_name
        shutil.copy2(found_src, target_path)
        seen_hashes[file_hash] = asset_name

        ensure_asset_sidecar(target_path, alt)

        replacements_count += 1
        print(f"  Resolved placeholder: '{alt}' -> 'assets/{asset_name}'")
        return f"![{alt}](assets/{asset_name})"

    def replace_crop_placeholder(match: re.Match) -> str:
        nonlocal replacements_count
        if Image is None:
            print("  Warning: Pillow not installed, skipping crop extraction.")
            return match.group(0)

        tag_str = match.group(0)
        attrs = parse_tag_attributes(tag_str)

        label = attrs.get('label', attrs.get('alt', 'Figure'))
        page_str = attrs.get('page', '')
        if not page_str:
            m_p = re.search(r'(\d+)', md_file.stem)
            page_num = int(m_p.group(1)) if m_p else 1
        else:
            page_num = int(page_str)

        xmin = int(attrs.get('xmin', 0))
        ymin = int(attrs.get('ymin', 0))
        xmax = int(attrs.get('xmax', 0))
        ymax = int(attrs.get('ymax', 0))

        if not pages_dir or not pages_dir.is_dir():
            print("  Warning: pages-dir not specified, cannot crop region.")
            return match.group(0)

        page_png = pages_dir / f"page_{page_num:03d}.png"
        if not page_png.is_file():
            print(f"  Warning: Page PNG '{page_png.name}' not found for crop.")
            return match.group(0)

        with Image.open(page_png) as img:
            w, h = img.size
            box = (
                max(0, xmin - padding),
                max(0, ymin - padding),
                min(w, xmax + padding),
                min(h, ymax + padding),
            )
            cropped = img.crop(box)

            ident = make_clean_identifier(label)
            if len(ident) > 40:
                ident = ident[:40].rstrip('_')
            asset_name = f"crop_p{page_num:03d}_{ident}.png"
            target_path = assets_dir / asset_name
            cropped.save(target_path, "PNG")

        ensure_asset_sidecar(target_path, label)
        replacements_count += 1
        print(f"  Cropped region: '{label}' -> 'assets/{asset_name}'")
        return f"![{label}](assets/{asset_name})"

    new_content = PLACEHOLDER_IMG_PATTERN.sub(replace_img_placeholder, content)
    new_content = CROP_PATTERN.sub(replace_crop_placeholder, new_content)

    return new_content, replacements_count


def main():
    parser = argparse.ArgumentParser(
        description="Resolve image placeholders in Markdown and update asset links."
    )
    parser.add_argument("--markdown", "-m", type=str, required=True, help="Path to Markdown file or directory")
    parser.add_argument("--assets-dir", "-a", type=str, default="assets", help="Target assets directory (default: assets)")
    parser.add_argument("--source-assets-dir", "-s", type=str, default=None, help="Directory containing source HTML assets")
    parser.add_argument("--pages-dir", "-p", type=str, default=None, help="Directory containing rendered page PNGs")
    parser.add_argument("--padding", type=int, default=15, help="Padding margin for crops in pixels")

    args = parser.parse_args()
    md_target = Path(args.markdown)
    assets_dir = Path(args.assets_dir)
    pages_dir = Path(args.pages_dir) if args.pages_dir else None

    if md_target.is_file():
        md_files = [md_target]
        base_dir = md_target.parent
    elif md_target.is_dir():
        md_files = sorted(list(md_target.glob("*.md")))
        base_dir = md_target
    else:
        print(f"Error: Target '{md_target}' does not exist.")
        sys.exit(1)

    search_dirs = [base_dir]
    if args.source_assets_dir:
        search_dirs.append(Path(args.source_assets_dir))

    total_replaced = 0
    for f in md_files:
        print(f"Processing: {f.name}")
        content = f.read_text(encoding="utf-8")
        updated, count = process_markdown_content(
            content=content,
            md_file=f,
            assets_dir=assets_dir,
            source_dirs=search_dirs,
            pages_dir=pages_dir,
            padding=args.padding,
        )
        if count > 0 and updated != content:
            f.write_text(updated, encoding="utf-8")
            total_replaced += count

    print(f"\nPlaceholder Replacement Complete.")
    print(f"  Files processed:       {len(md_files)}")
    print(f"  Placeholders resolved: {total_replaced}")
    print(f"  Assets location:       {assets_dir}")

    if assets_dir.exists() and not any(assets_dir.iterdir()):
        try:
            assets_dir.rmdir()
        except Exception:
            pass


if __name__ == "__main__":
    main()

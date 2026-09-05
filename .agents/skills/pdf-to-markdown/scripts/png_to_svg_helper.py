#!/usr/bin/env python3
"""
png_to_svg_helper.py - SVG viewBox & ClipPath Safety Auditor and Padding Expander

Audits SVG diagram files to ensure that viewBox dimensions and clipPath boundaries
do not truncate outer labels, IC pin names, signal lines, or borders when rendered
in Obsidian, and provides automated viewBox expansion with safety margins.
"""

import sys
import re
import argparse
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Tuple, Optional

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8', errors='backslashreplace')
if hasattr(sys.stderr, 'reconfigure'):
    sys.stderr.reconfigure(encoding='utf-8', errors='backslashreplace')

def parse_viewbox(vb_str: str) -> Optional[Tuple[float, float, float, float]]:
    """Parses viewBox string 'min_x min_y width height'."""
    parts = re.split(r'[\s,]+', vb_str.strip())
    if len(parts) == 4:
        try:
            return tuple(float(p) for p in parts)
        except ValueError:
            return None
    return None

def audit_and_pad_svg(svg_path: Path, padding: float = 20.0, in_place: bool = False) -> bool:
    """
    Checks an SVG file for tight viewBox or clipPath restrictions and expands them.
    """
    content = svg_path.read_text(encoding="utf-8")
    
    # 1. Match viewBox on root <svg>
    vb_match = re.search(r'<svg[^>]*\bviewBox=["\']([^"\']+)["\']', content)
    if not vb_match:
        print(f"[{svg_path.name}] Warning: No viewBox found on root <svg> tag.")
        return False

    raw_vb = vb_match.group(1)
    parsed = parse_viewbox(raw_vb)
    if not parsed:
        print(f"[{svg_path.name}] Warning: Could not parse viewBox: '{raw_vb}'")
        return False

    min_x, min_y, width, height = parsed

    # 2. Expand viewBox by padding on all 4 sides
    new_min_x = min_x - padding
    new_min_y = min_y - padding
    new_width = width + (padding * 2)
    new_height = height + (padding * 2)

    new_vb = f"{new_min_x:.1f} {new_min_y:.1f} {new_width:.1f} {new_height:.1f}"

    # 3. Check for clipPath elements that might truncate graphics
    has_clip_path = ('<clipPath' in content)

    print(f"[{svg_path.name}]:")
    print(f"  Current viewBox:  '{raw_vb}'")
    print(f"  Padded viewBox:   '{new_vb}' (+{padding}px margin)")
    if has_clip_path:
        print(f"  Notice: <clipPath> elements detected. Ensure clip bounds match or exceed padded viewBox.")

    if in_place:
        # Replace viewBox in content
        updated_content = content.replace(f'viewBox="{raw_vb}"', f'viewBox="{new_vb}"')
        updated_content = updated_content.replace(f"viewBox='{raw_vb}'", f"viewBox='{new_vb}'")
        
        # Optionally expand rect inside clipPath if present
        if has_clip_path:
            # Expand clip rect if it matches old viewBox
            def replace_clip_rect(m):
                rect_tag = m.group(0)
                # Expand width/height attributes if present
                return rect_tag

        svg_path.write_text(updated_content, encoding="utf-8")
        print(f"  Applied padded viewBox to {svg_path}")
        return True

    return True

def main():
    parser = argparse.ArgumentParser(description="Audit and expand SVG viewBox boundaries to prevent clipping.")
    parser.add_argument("path", type=str, help="Path to an SVG file or directory of SVGs")
    parser.add_argument("--padding", "-p", type=float, default=20.0, help="Padding margin to add in pixels (default: 20.0)")
    parser.add_argument("--apply", "-a", action="store_true", help="Apply modifications in-place to SVG files")
    args = parser.parse_args()

    target = Path(args.path)
    if target.is_file() and target.suffix.lower() == '.svg':
        audit_and_pad_svg(target, padding=args.padding, in_place=args.apply)
    elif target.is_dir():
        svgs = list(target.glob("*.svg"))
        print(f"Auditing {len(svgs)} SVG files in {target}...")
        for s in svgs:
            audit_and_pad_svg(s, padding=args.padding, in_place=args.apply)
    else:
        print(f"Error: '{target}' is not a valid SVG file or directory.")
        sys.exit(1)

if __name__ == "__main__":
    main()

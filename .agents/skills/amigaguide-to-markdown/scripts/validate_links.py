#!/usr/bin/env python3
"""
validate_links.py - Automated Obsidian Markdown Link & Anchor Validator

Simulates the exact heading normalization and link resolution engine from
Obsidian to verify that 100% of internal links and anchored subpaths in a
folder of Markdown documents resolve without errors.
"""

import sys
import re
import argparse
import urllib.parse
from pathlib import Path
from typing import Dict, List, Tuple

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8', errors='backslashreplace')
if hasattr(sys.stderr, 'reconfigure'):
    sys.stderr.reconfigure(encoding='utf-8', errors='backslashreplace')

# Exact regex used inside Obsidian's core JavaScript engine:
# var SD = /[!"#$%&()*+,.:;<=>?@^`{|}~\/\[\]\\\r\n]/g;
SD_PATTERN = re.compile(r'[!"#$%&()*+,.:;<=>?@^`{|}~/\\[\]\\\r\n]')

def obsidian_td(text: str) -> str:
    """
    Obsidian's internal TD(e) heading normalization algorithm:
    function TD(e) {
        return e.replace(SD, " ").replace(/\\s+/g, " ").trim().toLowerCase();
    }
    """
    cleaned = SD_PATTERN.sub(' ', text)
    return re.sub(r'\s+', ' ', cleaned).strip().lower()

def js_decode_uri(uri: str) -> str:
    """
    Simulates JavaScript's window.decodeURI(uri) behavior.
    Unlike urllib.parse.unquote (which decodes %2C, %3A, etc.),
    JS decodeURI leaves URI-reserved characters encoded!
    Specifically: %20 -> space, %28 -> '(', %29 -> ')',
    while %2C remains '%2C', %3A remains '%3A', %2F remains '%2F', etc.
    """
    # Replace sequences that decodeURI decodes:
    # Common ones in links: %20, %21, %27, %28, %29, %2D, %2E, %5F, %7E
    # Reserved ones that decodeURI DOES NOT decode:
    # %23, %24, %26, %2B, %2C, %2F, %3A, %3B, %3D, %3F, %40
    def repl(match):
        code = match.group(1).upper()
        # Reserved in decodeURI:
        # 23 (#), 24 ($), 26 (&), 2B (+), 2C (,), 2F (/), 3A (:), 3B (;), 3D (=), 3F (?), 40 (@)
        reserved = {'23', '24', '26', '2B', '2C', '2F', '3A', '3B', '3D', '3F', '40'}
        if code in reserved:
            return match.group(0) # leave %XX intact
        try:
            return chr(int(code, 16))
        except ValueError:
            return match.group(0)

    return re.sub(r'%([0-9a-fA-F]{2})', repl, uri)

def index_headings(directory: Path) -> Dict[str, List[str]]:
    """
    Maps filename -> list of exact heading strings found in that file.
    """
    headings_map = {}
    for md_file in directory.glob("*.md"):
        headings = []
        try:
            content = md_file.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            content = md_file.read_text(encoding="latin-1", errors="replace")

        for line in content.splitlines():
            # Check for Markdown headings # ... ######
            m = re.match(r'^#{1,6}\s+(.*)', line)
            if m:
                htitle = m.group(1).strip()
                headings.append(htitle)

        headings_map[md_file.name] = headings
    return headings_map

def validate_links(directory: Path, strict_same_doc: bool = True) -> int:
    """
    Validates all markdown links in directory. Returns number of failures.
    """
    headings_map = index_headings(directory)
    existing_files = set(headings_map.keys())
    
    total_links = 0
    total_anchored = 0
    failures = []
    warnings = []

    link_pattern = re.compile(r'\[([^\]]*)\]\(([^)]+)\)')

    for md_file in sorted(directory.glob("*.md")):
        try:
            content = md_file.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            content = md_file.read_text(encoding="latin-1", errors="replace")

        for line_num, line in enumerate(content.splitlines(), start=1):
            # Ignore code blocks or markdown fences if necessary, but regex scan lines
            for match in link_pattern.finditer(line):
                label = match.group(1)
                target = match.group(2).strip()

                # Skip external web URLs or relative image assets
                if target.startswith(('http://', 'https://', 'mailto:', 'assets/', '../assets/')):
                    continue
                if any(target.lower().endswith(ext) for ext in ('.png', '.jpg', '.jpeg', '.gif', '.svg', '.iff')):
                    continue

                total_links += 1

                # Parse target: [file.md][#anchor]
                if '#' in target:
                    enc_file, raw_anchor = target.split('#', 1)
                    total_anchored += 1
                else:
                    enc_file, raw_anchor = target, None

                target_filename = urllib.parse.unquote(enc_file) if enc_file else md_file.name

                # Check if target is same-doc
                is_same_doc = (target_filename == md_file.name)

                # Warning for Obsidian: Same-document link includes file name
                if enc_file and is_same_doc and strict_same_doc:
                    warnings.append(
                        f"[{md_file.name}:{line_num}] Same-document link includes file name: '{target}' "
                        f"(Recommend: '#{raw_anchor}' to ensure proper scrolling in Obsidian)"
                    )

                # Check file existence
                if target_filename not in existing_files:
                    failures.append(
                        f"[{md_file.name}:{line_num}] Target file does not exist: '{target_filename}' (Link: '{target}')"
                    )
                    continue

                # Check anchor resolution if present
                if raw_anchor is not None:
                    # Obsidian decodes the anchor via JS decodeURI
                    decoded_anchor = js_decode_uri(raw_anchor)
                    normalized_anchor = obsidian_td(decoded_anchor)

                    available_headings = headings_map[target_filename]
                    matched = False
                    for h in available_headings:
                        if obsidian_td(h) == normalized_anchor:
                            matched = True
                            break

                    if not matched:
                        # Check if it failed because %2C was used instead of literal comma
                        detail = ""
                        if '%2C' in raw_anchor.upper() or '%3A' in raw_anchor.upper():
                            detail = " (LIKELY CAUSE: Encoded %2C or %3A in anchor is not decoded by Obsidian's decodeURI!)"
                        failures.append(
                            f"[{md_file.name}:{line_num}] Broken anchor in '{target_filename}': "
                            f"#{raw_anchor} (Obsidian TD: '{normalized_anchor}'){detail}"
                        )

    # Report
    print("=" * 70)
    print(f"OBSIDIAN LINK VALIDATION REPORT for {directory}")
    print("=" * 70)
    print(f"Files scanned:         {len(existing_files)}")
    print(f"Total links checked:   {total_links}")
    print(f"Anchored links:        {total_anchored}")
    print(f"Warnings:              {len(warnings)}")
    print(f"Errors / Failures:     {len(failures)}")
    print("-" * 70)

    if warnings:
        print("\nWARNINGS:")
        for w in warnings[:20]:
            print(f"  ⚠ {w}")
        if len(warnings) > 20:
            print(f"  ... and {len(warnings) - 20} more warnings.")

    if failures:
        print("\nFAILURES:")
        for f in failures[:30]:
            print(f"  ❌ {f}")
        if len(failures) > 30:
            print(f"  ... and {len(failures) - 30} more failures.")
        print("\nRESULT: FAILED")
        return len(failures)
    else:
        print("\nRESULT: 100% PASS - ALL LINKS & ANCHORS VALIDATED IN OBSIDIAN!")
        return 0

def main():
    parser = argparse.ArgumentParser(description="Validate Obsidian markdown links and anchors.")
    parser.add_argument("directory", type=str, help="Directory containing markdown files")
    parser.add_argument("--no-strict-same-doc", action="store_true", help="Don't warn if same-doc link includes file name")
    args = parser.parse_args()

    target_dir = Path(args.directory)
    if not target_dir.is_dir():
        print(f"Error: {target_dir} is not a valid directory.")
        sys.exit(1)

    code = validate_links(target_dir, strict_same_doc=not args.no_strict_same_doc)
    sys.exit(min(code, 255))

if __name__ == "__main__":
    main()

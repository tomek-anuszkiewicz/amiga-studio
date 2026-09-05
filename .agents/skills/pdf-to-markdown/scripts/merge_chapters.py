#!/usr/bin/env python3
"""
merge_chapters.py - Page-to-Chapter Compiler, Header/Footer Stripper, & Navigation Injector

Compiles page-level Markdown documents into cohesive chapter files based on
the book manifest, strips repeating page headers and running footers,
seamlessly stitches multi-page split tables, injects Prev | TOC | Next
navigation bars at the beginning and end of each chapter, and generates the
top-level Table of Contents document.
"""

import os
import sys
import re
import json
import argparse
import urllib.parse
from pathlib import Path
from typing import Dict, List, Tuple, Any, Optional

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8', errors='backslashreplace')
if hasattr(sys.stderr, 'reconfigure'):
    sys.stderr.reconfigure(encoding='utf-8', errors='backslashreplace')

def clean_title(title: str) -> str:
    """Sanitizes chapter title for use in filenames."""
    t = re.sub(r'[<>:"/\\|?*]', '', title)
    t = re.sub(r'\s+', ' ', t).strip()
    return t

def strip_page_headers_and_footers(text: str) -> str:
    """
    Removes repeating print page artifacts: running top headers, bottom page
    numbers (e.g. '6-15', 'Page 42'), and repeating copyright lines.
    """
    lines = text.splitlines()
    filtered = []
    
    # Common print header/footer patterns
    header_patterns = [
        re.compile(r'^\s*(?:MOTOROLA|COMMODORE|AMIGA|USER[\'’]S MANUAL|REFERENCE MANUAL)\b.*$', re.IGNORECASE),
        re.compile(r'^\s*SECTION\s+\d+\s*[-–—]?\s*.*$', re.IGNORECASE),
        re.compile(r'^\s*CHAPTER\s+\d+\s*[-–—]?\s*.*$', re.IGNORECASE),
    ]
    
    footer_patterns = [
        re.compile(r'^\s*\d+[-–—]\d+\s*$'), # e.g. '6-15'
        re.compile(r'^\s*Page\s+[\d\-–—.]+\s*$', re.IGNORECASE), # e.g. 'Page 1-1', 'Page 42'
        re.compile(r'^\s*-\s*[\d\-–—.]+\s*-\s*$'), # e.g. '- 42 -', '- 1-1 -'
        re.compile(r'^\s*(?:©|Copyright|\(C\))\s*\d{4}\b.*$', re.IGNORECASE),
    ]

    for idx, line in enumerate(lines):
        line_s = line.strip()
        if not line_s:
            filtered.append(line)
            continue

        # Top 4 lines: check running header
        if idx < 4 and any(p.match(line_s) for p in header_patterns):
            continue

        # Bottom 4 lines: check running footer / page number
        if idx >= len(lines) - 4 and any(p.match(line_s) for p in footer_patterns):
            continue

        filtered.append(line)

    return '\n'.join(filtered)

def stitch_split_tables(text: str) -> str:
    """
    Detects when a Markdown table was split across consecutive page boundaries
    and stitches it into a single cohesive table, stripping redundant headers.
    """
    lines = text.splitlines()
    out_lines = []
    i = 0
    n = len(lines)

    while i < n:
        line = lines[i]
        
        # Check if current line is inside a table
        if line.strip().startswith('|') and line.strip().endswith('|'):
            out_lines.append(line)
            # Look ahead across empty lines to see if a continuation table starts
            j = i + 1
            while j < n and not lines[j].strip():
                j += 1
            
            if j < n and lines[j].strip().startswith('|') and lines[j].strip().endswith('|'):
                # Check if lines[j] is a repeated header row followed by delimiter |---|
                if j + 1 < n:
                    sep_line = lines[j + 1].strip()
                    chars = set(sep_line.replace('|', '').strip())
                    if sep_line.startswith('|') and chars and chars <= {'-', ':', ' '}:
                        # lines[j] is repeated header, lines[j+1] is delimiter. Skip both!
                        i = j + 2
                        continue
            i += 1
        else:
            out_lines.append(line)
            i += 1

    return '\n'.join(out_lines)

def build_navigation_bar(
    prev_chapter: Optional[Dict[str, Any]],
    toc_filename: str,
    next_chapter: Optional[Dict[str, Any]]
) -> str:
    """
    Constructs standardized Prev | TOC | Next navigation links.
    """
    parts = []
    enc_toc = urllib.parse.quote(toc_filename)

    if prev_chapter:
        enc_prev = urllib.parse.quote(prev_chapter['filename'])
        parts.append(f"[⬅ Previous: {prev_chapter['clean_title']}]({enc_prev})")
    
    parts.append(f"[📑 Table of Contents]({enc_toc})")

    if next_chapter:
        enc_next = urllib.parse.quote(next_chapter['filename'])
        parts.append(f"[Next: {next_chapter['clean_title']} ➡]({enc_next})")

    return " | ".join(parts)

def compile_chapters(
    manifest_path: Path,
    pages_md_dir: Path,
    output_dir: Path
) -> None:
    """
    Compiles page-level Markdown files into full chapter documents.
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    
    chapters = manifest.get('chapters', [])
    valid_chapters = [ch for ch in chapters if not ch.get('exclude', False)]
    
    # 1. Assign standardized filenames
    for idx, ch in enumerate(valid_chapters, start=1):
        clean_t = clean_title(ch['title'])
        ch['clean_title'] = clean_t
        ch['filename'] = f"{idx:02d} - {clean_t}.md"

    toc_filename = "00 - Table of Contents and Front Matter.md"

    # 2. Compile each chapter
    print(f"Compiling {len(valid_chapters)} chapters into {output_dir}...")

    for idx, ch in enumerate(valid_chapters):
        prev_ch = valid_chapters[idx - 1] if idx > 0 else None
        next_ch = valid_chapters[idx + 1] if idx + 1 < len(valid_chapters) else None

        nav_bar = build_navigation_bar(prev_ch, toc_filename, next_ch)

        # Collect page markdown chunks
        page_chunks = []
        for p in range(ch['start_page'], ch['end_page'] + 1):
            page_md = pages_md_dir / f"page_{p:03d}.md"
            if page_md.exists():
                c = page_md.read_text(encoding="utf-8").strip()
                if c:
                    c_clean = strip_page_headers_and_footers(c)
                    page_chunks.append(c_clean)
            else:
                # Check alternative naming: e.g. A500... 015.md
                alt_files = list(pages_md_dir.glob(f"*{p:03d}.md")) or list(pages_md_dir.glob(f"*{p}.md"))
                if alt_files:
                    c = alt_files[0].read_text(encoding="utf-8").strip()
                    c_clean = strip_page_headers_and_footers(c)
                    page_chunks.append(c_clean)

        raw_chapter_text = '\n\n'.join(page_chunks)
        stitched_text = stitch_split_tables(raw_chapter_text)

        # Ensure chapter starts with H1 title
        lines = stitched_text.splitlines()
        first_h1 = f"# {ch['clean_title']}"
        if lines and lines[0].startswith('# '):
            lines[0] = first_h1
            body_start = 1
        else:
            lines.insert(0, first_h1)
            body_start = 1

        # Assemble full document with top & bottom navigation bars
        full_doc = [
            lines[0], # H1 Title
            "",
            nav_bar,  # Top Navigation Bar
            "---",
            ""
        ]
        full_doc.extend(lines[body_start:])
        full_doc.extend([
            "",
            "---",
            nav_bar,  # Bottom Navigation Bar
            ""
        ])

        out_file = output_dir / ch['filename']
        out_file.write_text('\n'.join(full_doc) + '\n', encoding="utf-8")
        print(f"  Created: {ch['filename']} ({len(page_chunks)} pages, {len(stitched_text)} bytes)")

    # 3. Generate Table of Contents
    print(f"Generating Table of Contents: {toc_filename}...")
    toc_lines = [
        f"# {manifest.get('source_pdf', 'Reference Manual')}: Table of Contents\n",
        "> [!NOTE] Document Overview",
        f"> **Source PDF:** {manifest.get('source_pdf', 'Unknown')}",
        f"> **Total Pages:** {manifest.get('total_pages', 'N/A')}",
        f"> **Sections Compiled:** {len(valid_chapters)}",
        "",
        "## Table of Contents\n"
    ]

    for ch in valid_chapters:
        enc_target = urllib.parse.quote(ch['filename'])
        indent = "  " * max(0, ch.get('level', 1) - 1)
        toc_lines.append(f"{indent}- [{ch['clean_title']}]({enc_target})")

    toc_lines.append("\n---\n")
    (output_dir / toc_filename).write_text('\n'.join(toc_lines) + '\n', encoding="utf-8")
    print(f"Compilation finished successfully. All chapters saved to {output_dir}")

def main():
    parser = argparse.ArgumentParser(description="Compile page markdown files into chapters with navigation.")
    parser.add_argument("--manifest", "-m", type=str, required=True, help="Path to manifest.json")
    parser.add_argument("--pages-md-dir", "-p", type=str, required=True, help="Directory containing page markdown files")
    parser.add_argument("--output-dir", "-o", type=str, required=True, help="Target chapter output directory")
    args = parser.parse_args()

    manifest_path = Path(args.manifest)
    pages_md_dir = Path(args.pages_md_dir)
    output_dir = Path(args.output_dir)

    if not manifest_path.exists():
        print(f"Error: Manifest '{manifest_path}' not found.")
        sys.exit(1)

    compile_chapters(manifest_path, pages_md_dir, output_dir)

if __name__ == "__main__":
    main()

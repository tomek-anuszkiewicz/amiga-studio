#!/usr/bin/env python3
"""
pdf_to_pages.py - PDF Chapter Mapping & High-Resolution Page-to-PNG Renderer

Uses PyMuPDF to extract bookmark outlines, map chapter page ranges, exclude
obsolete print front/back matter (Index, List of Tables, List of Figures),
and render each page into high-resolution PNGs for LLM Vision transcription.
"""

import os
import sys
import json
import argparse
from pathlib import Path
from typing import Dict, List, Any

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8', errors='backslashreplace')
if hasattr(sys.stderr, 'reconfigure'):
    sys.stderr.reconfigure(encoding='utf-8', errors='backslashreplace')

try:
    import pymupdf as fitz
except ImportError:
    try:
        import fitz
    except ImportError:
        print("Error: PyMuPDF is required. Run 'pip install pymupdf'.")
        sys.exit(1)

def extract_outline_and_map_chapters(doc: fitz.Document, total_pages: int) -> List[Dict[str, Any]]:
    """
    Extracts PDF outline/bookmarks and maps each section to its starting and ending page.
    Filters out obsolete print matter such as Index, List of Tables, and List of Figures.
    """
    toc = doc.get_toc() # List of [lvl, title, page, ...]
    if not toc:
        # Fallback: single chapter spanning all pages
        return [{
            'title': 'Document Content',
            'start_page': 1,
            'end_page': total_pages,
            'level': 1,
            'exclude': False
        }]

    # Filter out empty or invalid items
    valid_toc = [item for item in toc if len(item) >= 3 and isinstance(item[2], int) and item[2] > 0]
    
    # Sort by starting page
    valid_toc.sort(key=lambda x: x[2])

    chapters = []
    for i, item in enumerate(valid_toc):
        lvl, title, start_p = item[0], item[1].strip(), item[2]
        
        # Calculate end page
        if i + 1 < len(valid_toc):
            next_start = valid_toc[i + 1][2]
            end_p = max(start_p, next_start - 1)
        else:
            end_p = total_pages

        title_lower = title.lower()
        
        # Check for obsolete print matter to exclude
        is_obsolete = any(keyword in title_lower for keyword in [
            'list of tables', 'list of figures', 'list of illustrations',
            'index', 'alphabetical index', 'subject index'
        ])

        chapters.append({
            'title': title,
            'start_page': start_p,
            'end_page': end_p,
            'level': lvl,
            'exclude': is_obsolete
        })

    return chapters

def render_pages_to_png(doc: fitz.Document, out_dir: Path, dpi: int = 200, start_page: int = 1, end_page: int = None) -> List[str]:
    """
    Renders pages of the PDF into zero-padded high-resolution PNGs.
    """
    out_dir.mkdir(parents=True, exist_ok=True)
    rendered_files = []
    total_pages = len(doc)
    start_p = max(1, start_page)
    end_p = min(total_pages, end_page) if end_page else total_pages
    
    # Matrix calculation for DPI (72 is standard base PDF DPI)
    zoom = dpi / 72.0
    mat = fitz.Matrix(zoom, zoom)

    print(f"Rendering pages {start_p} to {end_p} (of {total_pages}) at {dpi} DPI to {out_dir}...")

    for page_num in range(start_p, end_p + 1):
        page = doc.load_page(page_num - 1)
        pix = page.get_pixmap(matrix=mat, alpha=False)
        
        filename = f"page_{page_num:03d}.png"
        file_path = out_dir / filename
        pix.save(str(file_path))
        rendered_files.append(filename)

        if page_num % 25 == 0 or page_num == end_p:
            print(f"  Rendered [{page_num:03d}/{end_p:03d}] pages...")

    return rendered_files

def main():
    parser = argparse.ArgumentParser(description="Split PDF into chapter page mappings and render high-res PNGs.")
    parser.add_argument("pdf_path", type=str, help="Path to input PDF file")
    parser.add_argument("--output-dir", "-o", type=str, required=True, help="Output workspace directory")
    parser.add_argument("--dpi", type=int, default=200, help="Rendering resolution in DPI (default: 200)")
    parser.add_argument("--no-render", action="store_true", help="Only extract outline and manifest, skip PNG rendering")
    parser.add_argument("--start-page", type=int, default=1, help="First page to process (1-based, default: 1)")
    parser.add_argument("--end-page", type=int, default=None, help="Last page to process (1-based, default: all)")
    parser.add_argument("--custom-manifest", "-m", type=str, default=None, help="Path to custom chapters manifest JSON")
    args = parser.parse_args()

    pdf_path = Path(args.pdf_path)
    if not pdf_path.exists():
        print(f"Error: File '{pdf_path}' does not exist.")
        sys.exit(1)

    out_dir = Path(args.output_dir)
    pages_dir = out_dir / "pages"
    pages_dir.mkdir(parents=True, exist_ok=True)

    doc = fitz.open(str(pdf_path))
    total_pages = len(doc)
    start_page = max(1, args.start_page)
    end_page = min(total_pages, args.end_page) if args.end_page else total_pages
    print(f"Opened PDF '{pdf_path.name}': {total_pages} total pages (processing {start_page}..{end_page}).")

    # 1. Extract outline and map chapters
    if args.custom_manifest:
        custom_manifest_path = Path(args.custom_manifest)
        if not custom_manifest_path.exists():
            print(f"Error: Custom manifest '{custom_manifest_path}' does not exist.")
            sys.exit(1)
        manifest_data = json.loads(custom_manifest_path.read_text(encoding="utf-8"))
        chapters = manifest_data.get('chapters', [])
        print(f"Loaded {len(chapters)} chapters from custom manifest.")
    else:
        chapters = extract_outline_and_map_chapters(doc, total_pages)
    
    # 2. Render pages to PNG if requested
    if not args.no_render:
        rendered_files = render_pages_to_png(doc, pages_dir, dpi=args.dpi, start_page=start_page, end_page=end_page)
    else:
        rendered_files = [f"page_{p:03d}.png" for p in range(start_page, end_page + 1)]

    # 3. Save manifest
    manifest = {
        'source_pdf': pdf_path.name,
        'total_pages': total_pages,
        'processed_range': [start_page, end_page],
        'dpi': args.dpi,
        'chapters': chapters,
        'pages': rendered_files
    }

    manifest_path = out_dir / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    print(f"Manifest saved to {manifest_path} with {len(chapters)} mapped sections.")
    
    print("\nSummary of Mapped Chapters:")
    for ch in chapters:
        status = " (EXCLUDED - Obsolete Print Matter)" if ch.get('exclude', False) else ""
        print(f"  - [{ch['start_page']:03d}-{ch['end_page']:03d}] {ch['title']}{status}")

if __name__ == "__main__":
    main()

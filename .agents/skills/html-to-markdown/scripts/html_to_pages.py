#!/usr/bin/env python3
"""
html_to_pages.py - Headless Browser HTML-to-Page-PNG Renderer

Converts arbitrary HTML documents into high-resolution page PNGs
by printing the document to PDF via headless Chrome and rasterizing each
page using PyMuPDF.
"""

import os
import sys
import json
import shutil
import argparse
import subprocess
from pathlib import Path
from typing import List, Dict, Optional

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8', errors='backslashreplace')
if hasattr(sys.stderr, 'reconfigure'):
    sys.stderr.reconfigure(encoding='utf-8', errors='backslashreplace')

try:
    import pymupdf  # fitz
except ImportError:
    try:
        import fitz as pymupdf
    except ImportError:
        print("Error: PyMuPDF is required. Install via 'pip install pymupdf'.")
        sys.exit(1)


def find_chrome_executable(user_path: Optional[str] = None) -> Optional[str]:
    """Locates a Google Chrome, Chromium, or Edge executable."""
    if user_path and Path(user_path).is_file():
        return user_path

    # Common search candidates across platforms
    candidates = [
        # Windows
        os.environ.get("CHROME_BIN", ""),
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        os.path.expandvars(r"%LOCALAPPDATA%\Google\Chrome\Application\chrome.exe"),
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        # Linux
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/chromium-browser",
        "/usr/bin/chromium",
        # macOS
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    ]

    for c in candidates:
        if c and Path(c).is_file():
            return c

    # Check system PATH
    for name in ["chrome", "google-chrome", "chromium", "msedge"]:
        found = shutil.which(name)
        if found:
            return found

    return None


def html_to_pages(
    html_path: Path,
    output_dir: Path,
    resolution_dpi: int = 200,
    chrome_bin: Optional[str] = None,
    keep_pdf: bool = False,
) -> List[Path]:
    """Prints HTML to PDF via headless Chrome and exports each page as PNG."""
    chrome_exe = find_chrome_executable(chrome_bin)
    if not chrome_exe:
        raise RuntimeError(
            "Google Chrome, Chromium, or Edge not found. "
            "Specify path via --chrome-bin or set CHROME_BIN."
        )

    output_dir = output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    temp_pdf = output_dir / "_temp_printed.pdf"

    abs_html = html_path.resolve()
    file_url = abs_html.as_uri()

    print("Printing HTML to PDF via headless browser...")
    print(f"  Source:     {abs_html.name}")
    print(f"  Browser:    {Path(chrome_exe).name}")
    print(f"  Target PDF: {temp_pdf.name}")

    cmd = [
        chrome_exe,
        "--headless",
        "--disable-gpu",
        "--no-pdf-header-footer",
        f"--print-to-pdf={str(temp_pdf)}",
        file_url,
    ]

    result = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode != 0 or not temp_pdf.is_file():
        raise RuntimeError(
            f"Headless Chrome print-to-pdf failed with exit code {result.returncode}:\n"
            f"{result.stderr.decode('utf-8', errors='replace')}"
        )

    doc = pymupdf.open(temp_pdf)
    total_pages = len(doc)
    print(f"Rasterizing {total_pages} pages to PNG at {resolution_dpi} resolution...")

    page_paths: List[Path] = []
    manifest_pages: List[Dict] = []

    for i, page in enumerate(doc):
        page_num = i + 1
        page_file = output_dir / f"page_{page_num:03d}.png"
        pix = page.get_pixmap(dpi=resolution_dpi)
        pix.save(str(page_file))
        page_paths.append(page_file)

        manifest_pages.append({
            "page_number": page_num,
            "filename": page_file.name,
            "width": pix.width,
            "height": pix.height,
            "resolution": resolution_dpi,
        })
        print(f"  Rendered page {page_num:03d}/{total_pages:03d} ({pix.width}x{pix.height})")

    doc.close()

    manifest = {
        "source_html": abs_html.name,
        "total_pages": total_pages,
        "resolution": resolution_dpi,
        "pages": manifest_pages,
    }
    manifest_path = output_dir / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    print(f"Manifest written to: {manifest_path.name}")

    if not keep_pdf and temp_pdf.is_file():
        temp_pdf.unlink()

    return page_paths


def main():
    parser = argparse.ArgumentParser(
        description="Render HTML to high-resolution page PNGs using headless Chrome and PyMuPDF."
    )
    parser.add_argument("--input", "-i", type=str, required=True, help="Path to input HTML file")
    parser.add_argument("--output-dir", "-o", type=str, default="pages", help="Directory for rendered PNG pages")
    parser.add_argument("--resolution", type=int, default=200, help="Rasterization resolution (default: 200)")
    parser.add_argument("--chrome-bin", type=str, default=None, help="Path to Chrome/Edge executable")
    parser.add_argument("--keep-pdf", action="store_true", help="Retain intermediate printed PDF file")

    args = parser.parse_args()
    html_path = Path(args.input)

    if not html_path.is_file():
        print(f"Error: HTML input file '{html_path}' does not exist.")
        sys.exit(1)

    try:
        pages = html_to_pages(
            html_path=html_path,
            output_dir=Path(args.output_dir),
            resolution_dpi=args.resolution,
            chrome_bin=args.chrome_bin,
            keep_pdf=args.keep_pdf,
        )
        print(f"\nSuccessfully rendered {len(pages)} pages to '{args.output_dir}'.")
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()

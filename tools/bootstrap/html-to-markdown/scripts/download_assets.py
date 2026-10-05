#!/usr/bin/env python3
"""
download_assets.py - HTML Image Asset Extractor & Sidecar Generator

Scans an HTML document for local and remote image assets (<img> tags),
copies local files or downloads remote URLs into an assets/ directory,
and creates Git-tracked technical sidecars (.txt) per asset description rules.
"""

import os
import sys
import re
import shutil
import hashlib
import argparse
import urllib.request
from pathlib import Path
from typing import List, Dict, Set, Optional

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8', errors='backslashreplace')
if hasattr(sys.stderr, 'reconfigure'):
    sys.stderr.reconfigure(encoding='utf-8', errors='backslashreplace')

try:
    from bs4 import BeautifulSoup
except ImportError:
    BeautifulSoup = None


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


def extract_images_from_html(html_text: str) -> List[Dict[str, str]]:
    """Extracts src and alt attributes from all img tags in HTML."""
    results = []
    if BeautifulSoup is not None:
        soup = BeautifulSoup(html_text, "html.parser")
        for img in soup.find_all("img"):
            src = img.get("src")
            if src:
                alt = img.get("alt") or img.get("title") or Path(src).stem
                results.append({"src": src.strip(), "alt": alt.strip()})
    else:
        # Fallback regex
        for m in re.finditer(r'<img\s+[^>]*src=["\']([^"\']+)["\'][^>]*>', html_text, re.IGNORECASE):
            src = m.group(1).strip()
            alt_m = re.search(r'alt=["\']([^"\']*)["\']', m.group(0), re.IGNORECASE)
            alt = alt_m.group(1).strip() if alt_m else Path(src).stem
            results.append({"src": src, "alt": alt})

    return results


def download_or_copy_assets(
    html_path: Path,
    assets_dir: Path,
    base_url: Optional[str] = None,
) -> int:
    """Extracts, downloads, or copies all image assets from an HTML file."""
    assets_dir.mkdir(parents=True, exist_ok=True)
    html_dir = html_path.parent
    html_text = html_path.read_text(encoding="utf-8", errors="replace")

    images = extract_images_from_html(html_text)
    print(f"Found {len(images)} image reference(s) in {html_path.name}")

    copied_count = 0
    seen_hashes: Dict[str, str] = {}

    for idx, img in enumerate(images):
        src = img["src"]
        alt = img["alt"] or f"figure_{idx+1}"

        print(f"[{idx+1}/{len(images)}] Processing: {src}")

        # Check if remote URL
        is_remote = src.startswith(("http://", "https://"))
        if is_remote:
            url = src
        elif base_url:
            url = f"{base_url.rstrip('/')}/{src.lstrip('/')}"
            is_remote = True
        else:
            url = None

        ident = make_clean_identifier(alt)
        if len(ident) > 40:
            ident = ident[:40].rstrip('_')
        if not ident:
            ident = f"figure_{idx+1}"

        temp_dest = None
        ext = Path(src.split('?')[0]).suffix.lower() or ".png"
        target_name = f"{ident}{ext}"
        target_path = assets_dir / target_name

        if is_remote:
            try:
                headers = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"}
                req = urllib.request.Request(url, headers=headers)
                print(f"  Downloading remote: {url}")
                with urllib.request.urlopen(req, timeout=15) as resp:
                    data = resp.read()
                temp_dest = assets_dir / f"_temp_{target_name}"
                temp_dest.write_bytes(data)
                source_file = temp_dest
            except Exception as e:
                print(f"  Warning: Failed to download {url}: {e}")
                continue
        else:
            # Local file resolution
            local_cand = html_dir / src
            if not local_cand.is_file():
                # Try finding in adjacent folders
                matches = list(html_dir.rglob(Path(src).name))
                if matches:
                    local_cand = matches[0]

            if not local_cand.is_file():
                print(f"  Warning: Local asset not found: {src}")
                continue

            source_file = local_cand

        file_hash = compute_file_hash(source_file)
        if file_hash in seen_hashes:
            canonical = seen_hashes[file_hash]
            print(f"  Deduplicated: '{src}' matches '{canonical}'")
            if temp_dest and temp_dest.is_file():
                temp_dest.unlink()
            continue

        if temp_dest:
            temp_dest.replace(target_path)
        else:
            shutil.copy2(source_file, target_path)

        seen_hashes[file_hash] = target_name
        copied_count += 1
        print(f"  Saved to: {target_path.name}")

    print(f"\nAsset Extraction Complete.")
    print(f"  Total processed: {len(images)}")
    print(f"  Saved assets:    {copied_count}")
    print(f"  Destination:     {assets_dir}")

    if assets_dir.exists() and not any(assets_dir.iterdir()):
        try:
            assets_dir.rmdir()
        except Exception:
            pass

    return copied_count


def main():
    parser = argparse.ArgumentParser(
        description="Extract and download all image assets from HTML to assets/ directory."
    )
    parser.add_argument("--html", "-i", type=str, required=True, help="Path to input HTML file")
    parser.add_argument("--assets-dir", "-a", type=str, default="assets", help="Target assets directory (default: assets)")
    parser.add_argument("--base-url", "-u", type=str, default=None, help="Optional base URL for relative remote links")

    args = parser.parse_args()
    html_path = Path(args.html)

    if not html_path.is_file():
        print(f"Error: HTML file '{html_path}' does not exist.")
        sys.exit(1)

    download_or_copy_assets(
        html_path=html_path,
        assets_dir=Path(args.assets_dir),
        base_url=args.base_url,
    )


if __name__ == "__main__":
    main()

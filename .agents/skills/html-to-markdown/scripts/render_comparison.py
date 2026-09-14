#!/usr/bin/env python3
"""Visual Comparison Renderer for HTML to Markdown Conversions.

Renders both the original HTML and converted Markdown into images using headless
browser automation and stitches them into a side-by-side composite image for
visual verification.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Optional, Tuple
from PIL import Image, ImageDraw, ImageFont

try:
    from markdown_it import MarkdownIt
except ImportError:
    MarkdownIt = None


def find_browser_executable() -> Optional[str]:
    """Find Google Chrome or Microsoft Edge executable on the host."""
    candidates = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        shutil.which("chrome"),
        shutil.which("msedge"),
        shutil.which("google-chrome"),
        shutil.which("chromium"),
    ]
    for c in candidates:
        if c and os.path.exists(c):
            return c
    return None


def render_html_to_image(
    browser_exe: str,
    html_path: Path,
    output_png: Path,
    window_size: Tuple[int, int] = (1000, 1200),
) -> bool:
    """Render an HTML file to PNG using a headless browser."""
    file_url = f"file:///{html_path.resolve().as_posix()}"
    output_png.parent.mkdir(parents=True, exist_ok=True)

    cmd = [
        browser_exe,
        "--headless",
        "--disable-gpu",
        "--hide-scrollbars",
        f"--window-size={window_size[0]},{window_size[1]}",
        f"--screenshot={output_png.resolve()}",
        file_url,
    ]

    try:
        subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return output_png.exists() and output_png.stat().st_size > 0
    except Exception as err:
        print(f"Error rendering HTML '{html_path}': {err}", file=sys.stderr)
        return False


def convert_markdown_to_styled_html(md_path: Path, temp_html_path: Path) -> bool:
    """Convert Markdown to a standalone HTML page with clean GitHub-like styling."""
    if MarkdownIt is None:
        print("Error: markdown-it-py is not installed.", file=sys.stderr)
        return False

    with open(md_path, "r", encoding="utf-8", errors="replace") as f:
        md_text = f.read()

    # Strip YAML frontmatter for rendering
    if md_text.startswith("---"):
        parts = md_text.split("---", 2)
        if len(parts) >= 3:
            md_text = parts[2]

    parser = MarkdownIt("commonmark").enable("table")
    html_body = parser.render(md_text)

    styled_html = f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
body {{
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
    font-size: 14px;
    line-height: 1.6;
    color: #24292e;
    background-color: #ffffff;
    padding: 30px;
    margin: 0 auto;
    max-width: 900px;
}}
h1, h2, h3, h4 {{
    margin-top: 24px;
    margin-bottom: 16px;
    font-weight: 600;
    line-height: 1.25;
    border-bottom: 1px solid #eaecef;
    padding-bottom: .3em;
}}
p {{
    margin-top: 0;
    margin-bottom: 16px;
}}
pre {{
    padding: 16px;
    overflow: auto;
    font-size: 85%;
    line-height: 1.45;
    background-color: #f6f8fa;
    border-radius: 6px;
    border: 1px solid #e1e4e8;
    margin-bottom: 16px;
}}
code {{
    font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
    font-size: 85%;
    padding: .2em .4em;
    margin: 0;
    background-color: rgba(27,31,35,.05);
    border-radius: 3px;
}}
pre code {{
    background-color: transparent;
    padding: 0;
}}
table {{
    border-spacing: 0;
    border-collapse: collapse;
    margin-top: 0;
    margin-bottom: 16px;
    width: 100%;
}}
table th, table td {{
    padding: 6px 13px;
    border: 1px solid #dfe2e5;
}}
table tr:nth-child(2n) {{
    background-color: #f6f8fa;
}}
img {{
    max-width: 100%;
    box-sizing: content-box;
}}
</style>
</head>
<body>
{html_body}
</body>
</html>
"""

    temp_html_path.parent.mkdir(parents=True, exist_ok=True)
    with open(temp_html_path, "w", encoding="utf-8") as f:
        f.write(styled_html)
    return True


def create_side_by_side_composite(
    html_img_path: Path,
    md_img_path: Path,
    output_composite_path: Path,
    header_height: int = 50,
) -> bool:
    """Stitch HTML and Markdown screenshots side by side with titles."""
    try:
        img_html = Image.open(html_img_path)
        img_md = Image.open(md_img_path)

        # Equalize heights if needed
        height = max(img_html.height, img_md.height)
        width_total = img_html.width + img_md.width + 10  # 10px divider

        composite = Image.new("RGB", (width_total, height + header_height), color=(240, 242, 245))
        draw = ImageDraw.Draw(composite)

        # Draw header banner
        draw.rectangle([(0, 0), (width_total, header_height)], fill=(30, 35, 45))

        # Add titles
        draw.text((30, 15), "ORIGINAL HTML SOURCE", fill=(255, 255, 255))
        draw.text((img_html.width + 40, 15), "CONVERTED MARKDOWN (RENDERED)", fill=(100, 220, 140))

        # Paste images
        composite.paste(img_html, (0, header_height))
        composite.paste(img_md, (img_html.width + 10, header_height))

        # Draw dividing line
        draw.line(
            [(img_html.width + 5, 0), (img_html.width + 5, height + header_height)],
            fill=(180, 185, 195),
            width=2,
        )

        output_composite_path.parent.mkdir(parents=True, exist_ok=True)
        composite.save(output_composite_path)
        print(f"Saved side-by-side visual comparison to '{output_composite_path}'")
        return True
    except Exception as err:
        print(f"Error creating composite image: {err}", file=sys.stderr)
        return False


def main() -> int:
    parser = argparse.ArgumentParser(description="Render side-by-side visual comparison of HTML and Markdown.")
    parser.add_argument("--html", "-H", required=True, help="Path to original HTML file")
    parser.add_argument("--markdown", "-M", required=True, help="Path to converted Markdown file")
    parser.add_argument("--output-dir", "-o", default="Obsidian/Amiga/Reference/temp/html-sandbox", help="Output directory")
    parser.add_argument("--width", type=int, default=1000, help="Viewport width for each panel")
    parser.add_argument("--height", type=int, default=1200, help="Viewport height for each panel")

    args = parser.parse_args()
    html_path = Path(args.html)
    md_path = Path(args.markdown)
    out_dir = Path(args.output_dir)

    if not html_path.exists():
        print(f"Error: HTML file '{html_path}' does not exist.", file=sys.stderr)
        return 1
    if not md_path.exists():
        print(f"Error: Markdown file '{md_path}' does not exist.", file=sys.stderr)
        return 1

    browser_exe = find_browser_executable()
    if not browser_exe:
        print("Error: Neither Google Chrome nor Microsoft Edge was found on this system.", file=sys.stderr)
        return 1

    print(f"Using browser executable: {browser_exe}")

    html_img = out_dir / "html_render.png"
    md_temp_html = out_dir / "_md_rendered.html"
    md_img = out_dir / "markdown_render.png"
    composite_img = out_dir / "visual_comparison.png"

    print("Rendering original HTML to image...")
    if not render_html_to_image(browser_exe, html_path, html_img, (args.width, args.height)):
        return 1

    print("Rendering Markdown to styled HTML...")
    if not convert_markdown_to_styled_html(md_path, md_temp_html):
        return 1

    print("Rendering Markdown HTML to image...")
    if not render_html_to_image(browser_exe, md_temp_html, md_img, (args.width, args.height)):
        return 1

    print("Generating side-by-side visual comparison...")
    if not create_side_by_side_composite(html_img, md_img, composite_img):
        return 1

    print("[SUCCESS] Visual comparison generated successfully.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

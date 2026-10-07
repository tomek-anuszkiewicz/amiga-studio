#!/usr/bin/env python3
"""Deterministic browser-rendered comparisons; no inference or export dependency."""

import argparse
import base64
from html import escape
from io import BytesIO
from pathlib import Path
import sys
import tempfile

from bs4 import BeautifulSoup
from markdown_it import MarkdownIt
from PIL import Image
from playwright.sync_api import sync_playwright

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion.config import load_config, PDF_STAGES
from common.pdf_page_conversion import read_conversion
from common.pdf_page_markdown import segment_markdown
from common.pdf_tables import STAGE, ordered_markdown

CSS = """
body { margin: 24px; color: #17202a; background: white; font: 18px Arial; }
h1 { font-size: 22px; } h2 { font-size: 20px; }
.comparison { display: flex; align-items: flex-start; gap: 32px; width: max-content; }
.panel { width: max-content; min-width: 700px; }
.converted { max-width: none; }
.source { display: block; max-width: 1000px; height: auto; }
table { border-collapse: collapse; } th, td { border: 1px solid #aaa; padding: 8px; }
pre { white-space: pre; background: #f3f5f7; padding: 16px; width: max-content; min-width: 650px; }
details { margin: 16px 0; } summary { cursor: pointer; } img { max-width: 1000px; height: auto; }
"""


def png_uri(image):
    with BytesIO() as data:
        image.save(data, format="PNG")
        return "data:image/png;base64," + base64.b64encode(data.getvalue()).decode("ascii")


def comparison_html(workspace, entry, page, table):
    by_id = {s["segment_id"]: s for s in page["segments"]}
    group = [by_id[s] for s in table["table_group_segment_ids"]]
    # Use the persistent crop for the table; source context remains Stage 01 pixels.
    with Image.open(workspace / STAGE / table["table_source_asset"]) as crop:
        table_uri = png_uri(crop)
    with Image.open(workspace / entry["png_file"]) as image:
        boxes = [s["bbox"] for s in group]
        box = (min(b[0] for b in boxes), min(b[1] for b in boxes),
               max(b[2] for b in boxes), max(b[3] for b in boxes))
        with image.crop(box) as context:
            # Paste the saved table crop at its source position without rescaling.
            with Image.open(workspace / STAGE / table["table_source_asset"]) as crop:
                context.paste(crop, (table["bbox"][0] - box[0], table["bbox"][1] - box[1]))
            source_uri = png_uri(context)
    markdown = ordered_markdown(group, segment_markdown, lambda _: table_uri)
    renderer = MarkdownIt("commonmark", {"html": True}).enable("table")
    dom = BeautifulSoup(renderer.render(markdown), "html.parser")
    for img in dom.find_all("img"):
        if str(img.get("src", "")).startswith("assets/"):
            img["src"] = table_uri
    for details in dom.find_all("details"):
        summary = details.find("summary")
        if summary and summary.get_text() == "Table text for RAG":
            details["open"] = ""
    label = escape(f"Physical page {page['page']} | {table['segment_id']} | "
                   + {"html": "HTML", "markdown": "Markdown", "unconverted": "Unconverted"}[table["table_format"]])
    return (f"<!doctype html><html><head><meta charset='utf-8'><style>{CSS}</style></head>"
            f"<body><h1>{label}</h1><div class='comparison'>"
            f"<section class='panel'><h2>Source</h2><img class='source' src='{source_uri}'></section>"
            f"<section class='panel converted'><h2>Converted</h2>{dom}</section>"
            "</div></body></html>")


def render_reviews(workspace, config):
    output = workspace / "02.82_table_conversion_review"
    output.mkdir(parents=True, exist_ok=True)
    counts = dict.fromkeys(("markdown", "html", "unconverted"), 0)
    channel = config.get("table_review", {}).get("browser_channel", "msedge")
    with tempfile.TemporaryDirectory(prefix=".02.82-", dir=workspace) as temporary, sync_playwright() as backend:
        browser = backend.chromium.launch(channel=channel, headless=True)
        try:
            browser_page = browser.new_page(viewport={"width": 1800, "height": 1000},
                                            device_scale_factor=1, java_script_enabled=False)
            # Review only local artifacts; generated HTML cannot fetch remote content.
            browser_page.route("http://**/*", lambda route: route.abort())
            browser_page.route("https://**/*", lambda route: route.abort())
            for entry, page in read_conversion(workspace, STAGE):
                for table in page["segments"]:
                    if table["type"] != "table":
                        continue
                    html = Path(temporary) / "comparison.html"
                    html.write_text(comparison_html(workspace, entry, page, table), encoding="utf-8")
                    browser_page.goto(html.as_uri(), wait_until="load")
                    browser_page.evaluate("document.fonts.ready")
                    browser_page.evaluate("Promise.all(Array.from(document.images).map(i => i.decode()))")
                    width = browser_page.evaluate("Math.ceil(document.documentElement.scrollWidth)")
                    browser_page.set_viewport_size({"width": width, "height": 1000})
                    target = output / f"{table['segment_id']}_review.png"
                    browser_page.screenshot(path=str(target), full_page=True)
                    counts[table["table_format"]] += 1
                    print(f"[review] {target}")
        finally:
            browser.close()
    print(f"[review] {counts}; PNGs: {output}")


def main():
    parser = argparse.ArgumentParser(description="Stage 02.82: table source/conversion review")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args()
    render_reviews(args.workspace.resolve(), load_config(args.config, known_stages=PDF_STAGES))


if __name__ == "__main__":
    main()

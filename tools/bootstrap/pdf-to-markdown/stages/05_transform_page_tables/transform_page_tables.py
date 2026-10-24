#!/usr/bin/env python3
"""Replace table content with Markdown or HTML transcribed from temporary crops."""

import argparse
from copy import deepcopy
import html
from pathlib import Path
import re
import sys
import tempfile
from time import perf_counter
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion import CodexClient
from conversion.parallel import convert_parallel
from common.pdf_config import load_config
from common.pdf_artifacts import read_json, write_json
from common.pdf_page_conversion import read_conversion
from common.pdf_tables import STAGE, converted
from common.pdf_selection import selected_pages


ANGLE_NOTATION = re.compile(r"<[^<>\r\n]+>")


def blocks_in_crop(page, crop):
    """Select extracted PDF blocks that overlap this table's pixel crop."""
    if not page['blocks']:
        return []
    a, b, c, d, e, f = page['raster']['display_to_pixels']
    selected = []
    for block in page['blocks']:
        x0, y0, x1, y1 = block['bbox']
        corners = [(a*x + c*y + e, b*x + d*y + f) for x, y in ((x0, y0), (x1, y1))]
        left, right = sorted(point[0] for point in corners)
        top, bottom = sorted(point[1] for point in corners)
        if max(left, crop[0]) < min(right, crop[2]) and max(top, crop[1]) < min(bottom, crop[3]):
            selected.append(block)
    return selected


def escape_source_angle_notation(markup, blocks):
    """Keep angle-bracket text extracted from the PDF out of the HTML parser."""
    source = '\n'.join(block['text'] for block in blocks)
    for literal in sorted(set(ANGLE_NOTATION.findall(source)), key=len, reverse=True):
        markup = markup.replace(literal, html.escape(literal, quote=False))
    return markup


def transform_tables(workspace, config, pages=None):
    directory = Path(__file__).parent
    prompt = (directory / "prompt.md").read_text(encoding="utf-8")
    input_dir = "04_filter_page_content"
    output = workspace / STAGE
    output.mkdir(parents=True, exist_ok=True)
    counts = dict.fromkeys(("markdown", "html", "unconverted"), 0)
    saved_pages = 0
    with tempfile.TemporaryDirectory(prefix=".05-", dir=workspace) as temporary:
        jobs = []
        pending_pages = {}
        for entry, page in read_conversion(workspace, input_dir,
                                           pages=pages):
            tables = [s for s in page["segments"] if s["type"] == "table"]
            if tables:
                print(f"[tables] {entry['page_id']}: {len(tables)} tables", flush=True)
                pending_pages[entry['page_id']] = {
                    "page": page, "original": deepcopy(page), "remaining": len(tables),
                    "source_page": read_json(workspace / '02_preprocess' /
                                             f"{entry['page_id']}.json")}
                with Image.open(workspace / entry["png_file"]) as image:
                    for index, table in enumerate(tables, 1):
                        label = f"{table['segment_id']} ({index}/{len(tables)})"
                        crop_path = Path(temporary) / f"{table['segment_id']}.png"
                        with image.crop(tuple(table["bbox"])) as crop:
                            crop.save(crop_path)
                        jobs.append((entry['page_id'], table, label, crop_path))
            else:
                print(f"[tables] {entry['page_id']}: no tables; skipped", flush=True)

        def convert(codex, job):
            _, _, label, crop_path = job
            print(f"[table] {label}: transcribing...", flush=True)
            started = perf_counter()
            try:
                result = codex.generate_json(prompt, image_path=crop_path)
            except Exception as error:
                error.add_note(f"Table conversion failed: {label}")
                raise
            print(f"[table] {label}: {result['format']} in "
                  f"{perf_counter() - started:.2f}s", flush=True)
            return result

        def accept(job, result):
            nonlocal saved_pages
            page_id, table, _, _ = job
            for key in ("table_rag_text", "table_group_segment_ids", "table_source_asset"):
                table.pop(key, None)
            table["table_format"] = result["format"]
            if converted(table):
                table["md_text"] = escape_source_angle_notation(
                    result["md_text"], blocks_in_crop(pending_pages[page_id]["source_page"], table['bbox']))
            counts[result["format"]] += 1
            pending = pending_pages[page_id]
            pending["remaining"] -= 1
            if pending["remaining"] == 0:
                if pending["page"] != pending["original"]:
                    write_json(output / f"{page_id}_segments.json", pending["page"])
                    saved_pages += 1
                    print(f"[tables] {page_id}: changed page saved", flush=True)
                else:
                    print(f"[tables] {page_id}: unchanged; no file saved", flush=True)
                del pending_pages[page_id]

        convert_parallel(config, stage=STAGE, operation="pdf_table_to_markup",
                         jobs=jobs, convert=convert, accept=accept, client_factory=CodexClient)
    print(f"[tables] {counts}; changed pages saved: {saved_pages}; JSON: {output}", flush=True)


def main():
    parser = argparse.ArgumentParser(description="Stage 05: transcribe page tables")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    transform_tables(args.workspace.resolve(), load_config(args.config),
                     pages=selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

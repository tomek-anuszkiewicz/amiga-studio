#!/usr/bin/env python3
"""Replace table content with Markdown or HTML transcribed from temporary crops."""

import argparse
from copy import deepcopy
from pathlib import Path
import sys
import tempfile
from time import perf_counter
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion import CodexClient
from common.pdf_config import load_config
from common.pdf_artifacts import write_json
from common.pdf_page_conversion import read_conversion
from common.pdf_schemas import object_schema, STRING
from common.pdf_tables import STAGE, converted
from common.pdf_selection import selected_pages

RESPONSE = object_schema({"format": {"type": "string", "enum": ["markdown", "html", "unconverted"]},
                          "md_text": STRING})


def transform_tables(workspace, config, pages=None):
    directory = Path(__file__).parent
    prompt = (directory / "prompt.md").read_text(encoding="utf-8")
    input_dir = "04_filter_page_content"
    output = workspace / STAGE
    output.mkdir(parents=True, exist_ok=True)
    counts = dict.fromkeys(("markdown", "html", "unconverted"), 0)
    saved_pages = 0
    with tempfile.TemporaryDirectory(prefix=".05-", dir=workspace) as temporary, \
            CodexClient(config, stage=STAGE, operation="pdf_table_to_markup") as codex:
        for entry, page in read_conversion(workspace, input_dir,
                                           pages=pages):
            tables = [s for s in page["segments"] if s["type"] == "table"]
            if tables:
                print(f"[tables] {entry['page_id']}: {len(tables)} tables", flush=True)
                original = deepcopy(page)
                with Image.open(workspace / entry["png_file"]) as image:
                    for index, table in enumerate(tables, 1):
                        label = f"{table['segment_id']} ({index}/{len(tables)})"
                        print(f"[table] {label}: transcribing...", flush=True)
                        started = perf_counter()
                        crop_path = Path(temporary) / f"{table['segment_id']}.png"
                        with image.crop(tuple(table["bbox"])) as crop:
                            crop.save(crop_path)
                        result = codex.generate_json(prompt, schema=RESPONSE, image_path=crop_path)
                        for key in ("table_rag_text", "table_group_segment_ids", "table_source_asset"):
                            table.pop(key, None)
                        table["table_format"] = result["format"]
                        if converted(table):
                            table["md_text"] = result["md_text"]
                        counts[result["format"]] += 1
                        print(f"[table] {label}: {result['format']} in "
                              f"{perf_counter() - started:.2f}s", flush=True)
                if page != original:
                    write_json(output / f"{entry['page_id']}_segments.json", page)
                    saved_pages += 1
                    print(f"[tables] {entry['page_id']}: changed page saved", flush=True)
                else:
                    print(f"[tables] {entry['page_id']}: unchanged; no file saved", flush=True)
            else:
                print(f"[tables] {entry['page_id']}: no tables; skipped", flush=True)
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

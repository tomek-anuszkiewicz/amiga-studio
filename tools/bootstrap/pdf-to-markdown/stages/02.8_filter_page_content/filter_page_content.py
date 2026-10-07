#!/usr/bin/env python3
"""Stage 02.8: apply requested source-content exclusions to page objects."""

import argparse
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion.config import load_config, PDF_STAGES
from common.pdf_artifacts import page_files, read_json, write_json

EXCLUDED_PAGE_TYPES = {
    "list_of_tables_heading", "list_of_tables", "list_of_figures_heading",
    "list_of_figures", "index_heading", "index",
}


def filter_page_content(workspace):
    pages = [(path, read_json(path)) for path in
             page_files(workspace / "02_page_conversion", "page_*_segments.json")]
    # Inspect original objects: even an excluded page can define the boundary.
    boundary = next(((page_index, segment_index)
                     for page_index, (_, value) in enumerate(pages)
                     for segment_index, segment in enumerate(value["segments"])
                     if segment["type"] == "toc_heading"), None)
    output = workspace / "02.8_filter_page_content"
    output.mkdir(parents=True, exist_ok=True)
    input_objects = sum(len(value["segments"]) for _, value in pages)
    output_pages = output_objects = 0
    for page_index, (path, value) in enumerate(pages):
        if any(segment["type"] in EXCLUDED_PAGE_TYPES for segment in value["segments"]):
            continue
        segments = [segment for segment_index, segment in enumerate(value["segments"])
                    if segment["type"] not in {"header", "footer"}
                    and (boundary is None or (page_index, segment_index) >= boundary)]
        if not segments:
            continue
        write_json(output / path.name, {**value, "segments": segments})
        output_pages += 1
        output_objects += len(segments)
    print(f"[filter] pages {len(pages)} -> {output_pages}; "
          f"objects {input_objects} -> {output_objects}; "
          f"removed pages: {len(pages) - output_pages}; "
          f"TOC boundary found: {boundary is not None}")


def main():
    parser = argparse.ArgumentParser(description="Stage 02.8: filter page content")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args()
    load_config(args.config, known_stages=PDF_STAGES, required_stages=())
    filter_page_content(args.workspace.resolve())


if __name__ == "__main__":
    main()

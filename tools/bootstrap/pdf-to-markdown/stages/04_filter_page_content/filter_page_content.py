#!/usr/bin/env python3
"""Stage 04: apply requested source-content exclusions to page objects."""

import argparse
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from common.pdf_config import load_config
from common.pdf_artifacts import page_files
from common.pdf_selection import selected_pages
from common.pdf_artifacts import read_json, write_json


def filter_page_content(workspace, pages=None):
    pages = [(path, read_json(path)) for path in
             page_files(workspace / "03_page_conversion", "page_*_segments.json", pages)]
    output = workspace / "04_filter_page_content"
    output.mkdir(parents=True, exist_ok=True)
    input_objects = sum(len(value["segments"]) for _, value in pages)
    output_pages = output_objects = 0
    for path, value in pages:
        segments = [segment for segment in value["segments"]
                    if segment["type"] not in {"header", "footer"}]
        if not segments:
            continue
        write_json(output / path.name, {**value, "segments": segments})
        output_pages += 1
        output_objects += len(segments)
    print(f"[filter] pages {len(pages)} -> {output_pages}; "
          f"objects {input_objects} -> {output_objects}; "
          f"removed empty pages: {len(pages) - output_pages}")


def main():
    parser = argparse.ArgumentParser(description="Stage 04: filter page content")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    load_config(args.config)
    filter_page_content(args.workspace.resolve(), pages=selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

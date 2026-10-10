#!/usr/bin/env python3
"""Stage 03: independent logical objects and per-object Markdown."""

import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion import CodexClient
from common.pdf_config import load_config
from common.pdf_artifacts import read_json, write_json, page_files
from common.pdf_selection import selected_pages


def convert_pages(workspace, config, pages=None):
    prompt = Path(__file__).with_name("prompt.md").read_text(encoding="utf-8")
    output = workspace / "03_page_conversion"
    output.mkdir(parents=True, exist_ok=True)
    with CodexClient(config, stage="03_page_conversion", operation="pdf_page_to_objects") as codex:
        for path in page_files(workspace / "02_preprocess", "page_*.json",
                               pages):
            data = read_json(path)
            # Character rectangles belong to deterministic book import, not the model request.
            for block in data["blocks"]:
                block.pop("char_bboxes", None)
            page_id = path.stem
            request = (prompt + "\n\nSource data (complete extracted-text JSON):\n"
                       + json.dumps(data, ensure_ascii=False))
            value = codex.generate_json(request, image_path=path.with_suffix(".png"))
            for ordinal, segment in enumerate(value["segments"], 1):
                segment["segment_id"] = f"{page_id}_seg_{ordinal:03d}"
            write_json(output / f"{page_id}_segments.json", value)
            print(f"[conversion] {page_id}: {len(value['segments'])} objects")


def main():
    parser = argparse.ArgumentParser(description="Stage 03: independent page conversion")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    config = load_config(args.config)
    convert_pages(args.workspace.resolve(), config, pages=selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

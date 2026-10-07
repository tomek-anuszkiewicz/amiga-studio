#!/usr/bin/env python3
"""Stage 02: independent logical objects and per-object Markdown."""

import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion import CodexClient
from conversion.config import load_config, PDF_STAGES
from common.pdf_artifacts import read_json, write_json, page_files
from common.pdf_page_conversion import RESPONSE


def convert_pages(workspace, config):
    prompt = Path(__file__).with_name("prompt.md").read_text(encoding="utf-8")
    output = workspace / "02_page_conversion"
    output.mkdir(parents=True, exist_ok=True)
    with CodexClient(config, stage="02_page_conversion", images=True) as codex:
        for path in page_files(workspace / "01_preprocess", "page_*.json",
                               config.get("input", {}).get("pages")):
            data = read_json(path)
            entry = data
            page_id = path.stem
            request = (prompt + "\n\nSource data (complete extracted-text JSON):\n"
                       + json.dumps(data, ensure_ascii=False)
                       + "\nOutput page identity: " + json.dumps({
                           "page": entry["page"], "image_width": entry["raster"]["pixel_width"],
                           "image_height": entry["raster"]["pixel_height"]}))
            value = codex.generate_json(request, schema=RESPONSE,
                                        image_path=path.with_suffix(".png"))
            for ordinal, segment in enumerate(value["segments"], 1):
                segment["segment_id"] = f"{page_id}_seg_{ordinal:03d}"
            write_json(output / f"{page_id}_segments.json", value)
            print(f"[conversion] {page_id}: {len(value['segments'])} objects")


def main():
    parser = argparse.ArgumentParser(description="Stage 02: independent page conversion")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args()
    config = load_config(args.config, known_stages=PDF_STAGES, required_stages={"02_page_conversion"})
    convert_pages(args.workspace.resolve(), config)


if __name__ == "__main__":
    main()

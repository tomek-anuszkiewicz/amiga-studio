#!/usr/bin/env python3
"""Stage 02d: independent logical objects and per-object Markdown."""

import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
from conversion import CodexClient
from conversion.config import load_config, PDF_STAGES
from conversion.pdf_artifacts import read_json, write_json, validate_preprocess
from conversion.pdf_page_conversion import RESPONSE, validate_page


def convert_pages(workspace, config):
    manifest = validate_preprocess(workspace)
    prompt = Path(__file__).with_name("prompt.md").read_text(encoding="utf-8")
    output = workspace / "02d_page_conversion"
    output.mkdir(parents=True, exist_ok=True)
    with CodexClient(config, stage="02d_page_conversion", images=True) as codex:
        for entry in manifest["pages"]:
            data = read_json(workspace / entry["json_file"])
            request = (prompt + "\n\nSource data (complete extracted-text JSON):\n"
                       + json.dumps(data, ensure_ascii=False)
                       + "\nOutput page identity: " + json.dumps({
                           "page": entry["page"], "image_width": entry["raster"]["pixel_width"],
                           "image_height": entry["raster"]["pixel_height"]}))
            value = codex.generate_json(request, schema=RESPONSE,
                                        image_path=workspace / entry["png_file"],
                                        validator=lambda value: validate_page(value, entry))
            for ordinal, segment in enumerate(value["segments"], 1):
                segment["segment_id"] = f"{entry['page_id']}_seg_{ordinal:03d}"
            validate_page(value, entry, stored=True)
            write_json(output / f"{entry['page_id']}_segments.json", value)
            print(f"[conversion] {entry['page_id']}: {len(value['segments'])} objects")


def main():
    parser = argparse.ArgumentParser(description="Stage 02d: independent page conversion")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args()
    config = load_config(args.config, known_stages=PDF_STAGES, required_stages={"02d_page_conversion"})
    convert_pages(args.workspace.resolve(), config)


if __name__ == "__main__":
    main()

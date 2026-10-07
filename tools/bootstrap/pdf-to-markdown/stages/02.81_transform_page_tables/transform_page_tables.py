#!/usr/bin/env python3
"""Transcribe persistent original-resolution table crops with Codex."""

import argparse
import json
from pathlib import Path
import sys
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion import CodexClient
from conversion.config import load_config, PDF_STAGES
from common.pdf_artifacts import write_json
from common.pdf_page_conversion import read_conversion
from common.pdf_schemas import object_schema, STRING
from common.pdf_tables import STAGE, collect_groups, simple_html

RESPONSE = object_schema({"format": {"type": "string", "enum": ["markdown", "html", "unconverted"]},
                          "md_text": STRING})
COMPANION = object_schema({"rag_text": {"type": "string", "minLength": 1}})


def transform_tables(workspace, config):
    directory = Path(__file__).parent
    prompt = (directory / "prompt.md").read_text(encoding="utf-8")
    group_prompt = (directory / "group_prompt.md").read_text(encoding="utf-8")
    predecessor = config.get("table_conversion", {}).get("predecessor", "02.8")
    input_dir = {"02.8": "02.8_filter_page_content", "02": "02_page_conversion"}[predecessor]
    output = workspace / STAGE
    assets = output / "assets"
    assets.mkdir(parents=True, exist_ok=True)
    counts = dict.fromkeys(("markdown", "html", "unconverted"), 0)
    with CodexClient(config, stage=STAGE, images=True) as codex:
        for entry, page in read_conversion(workspace, input_dir):
            groups = collect_groups(page["segments"], page["page"])
            tables = [s for s in page["segments"] if s["type"] == "table"]
            if tables:
                with Image.open(workspace / entry["png_file"]) as image:
                    for table in tables:
                        crop_path = assets / f"{table['segment_id']}.png"
                        with image.crop(tuple(table["bbox"])) as crop:
                            crop.save(crop_path)
                        table["table_source_asset"] = crop_path.relative_to(output).as_posix()
                        result = codex.generate_json(prompt, schema=RESPONSE, image_path=crop_path)
                        table["table_format"] = result["format"]
                        if result["format"] != "unconverted":
                            table["md_text"] = result["md_text"]
                        if result["format"] == "html":
                            group = [{"segment_id": s["segment_id"], "type": s["type"],
                                      "md_text": s["md_text"]} for s in groups[table["segment_id"]]]
                            request = group_prompt + "\n\nSource-ordered group:\n" + json.dumps({
                                "table_segment_id": table["segment_id"],
                                "table_position": next(i for i, s in enumerate(group)
                                                       if s["segment_id"] == table["segment_id"]),
                                "segments": group}, ensure_ascii=False)
                            companion = codex.generate_json(request, schema=COMPANION, image_path=crop_path)
                            if not companion["rag_text"].strip():
                                raise ValueError(f"Empty HTML companion for {table['segment_id']}")
                            table["table_rag_text"] = companion["rag_text"]
                            if simple_html(table["md_text"]):
                                print(f"WARNING: {table['segment_id']} (physical page {page['page']}): "
                                      "HTML table has no merged cells and simple cell content; "
                                      "Markdown should be sufficient. Review format choice.")
                        counts[result["format"]] += 1
            write_json(output / f"{entry['page_id']}_segments.json", page)
    print(f"[tables] {counts}; JSON/assets: {output}")


def main():
    parser = argparse.ArgumentParser(description="Stage 02.81: transcribe page tables")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args()
    transform_tables(args.workspace.resolve(), load_config(args.config, known_stages=PDF_STAGES))


if __name__ == "__main__":
    main()

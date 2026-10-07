#!/usr/bin/env python3
"""Stage 02.41: identify independent tables inside existing table objects."""

import argparse
import json
from pathlib import Path
import sys
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion import CodexClient
from conversion.config import load_config, PDF_STAGES
from common.pdf_artifacts import read_json, write_json
from common.pdf_page_conversion import resolved_page_files, TABLE_SPLIT_STAGE, SEGMENT
from common.pdf_schemas import object_schema, STRING
from common.pdf_selection import selected_pages

TABLE = object_schema({**SEGMENT["properties"],
                       "type": {"type": "string", "enum": ["table"]},
                       "continuation": {"type": "boolean", "enum": [False]},
                       "segment_id": STRING})
RESPONSE = object_schema({"tables": {"type": "array", "items": TABLE, "minItems": 1}})


def table_replacements(source, tables, reserved_ids):
    """Keep singleton responses unchanged; assign split IDs in model reading order."""
    if not tables:
        raise ValueError("Table split response contains no tables")
    if len(tables) == 1:
        return [source]
    replacements = []
    for index, table in enumerate(tables):
        identity = source["segment_id"]
        if index:
            suffix = 2
            while f"{identity}_table_{suffix}" in reserved_ids:
                suffix += 1
            identity = f"{identity}_table_{suffix}"
        reserved_ids.add(identity)
        replacements.append({**source, **table, "segment_id": identity,
                             "continuation": False,
                             "source_segment_ids": [source["segment_id"]],
                             "replacement_stage": TABLE_SPLIT_STAGE})
    return replacements


def split_tables(workspace, config, pages=None):
    paths = list(resolved_page_files(workspace, pages, before_stage="02.41"))
    output = workspace / TABLE_SPLIT_STAGE
    output.mkdir(parents=True, exist_ok=True)
    prompt = Path(__file__).with_name("prompt.md").read_text(encoding="utf-8")
    requested = changed_pages = split_objects = total_before = total_after = 0
    codex = None
    try:
        for path in paths:
            page = read_json(path)
            objects = page["segments"]
            candidates = [obj for obj in objects if obj["type"] == "table"]
            page_id = path.stem.removesuffix("_segments")
            replacements = {}
            reserved_ids = {obj["segment_id"] for obj in objects}
            if candidates:
                source = workspace / "01_preprocess" / f"{page_id}.png"
                with Image.open(source) as image:
                    image.load()
                    width, height = image.size
                context = {key: value for key, value in page.items() if key != "segments"}
                context.update(image_width=width, image_height=height, objects=objects)
                if codex is None:
                    codex = CodexClient(config, stage=TABLE_SPLIT_STAGE, images=True)
                for target in candidates:
                    request = prompt + "\n\nFrozen source page and target table:\n" + json.dumps(
                        {**context, "target_table": target}, ensure_ascii=False)
                    response = codex.generate_json(request, schema=RESPONSE, image_path=source)
                    tables = table_replacements(target, response["tables"], reserved_ids)
                    replacements[target["segment_id"]] = tables
                    requested += 1
                    split_objects += len(tables) > 1
                    print(f"[table-split] {page_id}/{target['segment_id']}: tables=1 -> {len(tables)}")
            segments = [child for obj in objects
                        for child in replacements.get(obj["segment_id"], [obj])]
            changed = segments != objects
            if changed:
                write_json(output / path.name, {**page, "segments": segments})
                changed_pages += 1
            before = len(candidates)
            after = sum(obj["type"] == "table" for obj in segments)
            total_before += before
            total_after += after
            print(f"[table-split] {page_id}: tables={before} -> {after}; changed={changed}")
        print(f"[table-split] input pages={len(paths)}; requested tables={requested}; "
              f"split objects={split_objects}; changed pages={changed_pages}; "
              f"tables={total_before} -> {total_after}")
    finally:
        if codex is not None:
            codex.close()


def main():
    parser = argparse.ArgumentParser(description="Stage 02.41: split grouped tables with original-page vision")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    split_tables(args.workspace.resolve(), load_config(args.config, known_stages=PDF_STAGES),
                 pages=selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Stage 02.42: review table representations against original-page pixels."""

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
from common.pdf_page_conversion import resolved_page_files, TABLE_RECLASSIFY_STAGE, SEGMENT
from common.pdf_schemas import object_schema
from common.pdf_selection import selected_pages

RESPONSE = object_schema({
    "action": {"type": "string", "enum": ["retain", "replace"]},
    "replacements": {"type": "array", "items": SEGMENT},
})


def table_replacements(source, response, reserved_ids):
    """Preserve retained objects and assign stable child identities in source order."""
    if response["action"] == "retain":
        return [source]
    children = response["replacements"]
    if not children:
        raise ValueError("Table reclassification replacement contains no objects")
    replacements = []
    for index, child in enumerate(children):
        identity = source["segment_id"]
        if index:
            suffix = 2
            while f"{identity}_part_{suffix}" in reserved_ids:
                suffix += 1
            identity = f"{identity}_part_{suffix}"
        reserved_ids.add(identity)
        replacement = {**source, **child, "segment_id": identity,
                       "source_segment_ids": [source["segment_id"]],
                       "replacement_stage": TABLE_RECLASSIFY_STAGE}
        if len(children) == 1:
            replacement.update(bbox=list(source["bbox"]), continuation=source["continuation"])
        if child["type"] == "graphic":
            replacement["image_only"] = True
        replacements.append(replacement)
    return replacements


def reclassify_tables(workspace, config, pages=None):
    paths = list(resolved_page_files(workspace, pages, before_stage="02.42"))
    output = workspace / TABLE_RECLASSIFY_STAGE
    output.mkdir(parents=True, exist_ok=True)
    prompt = Path(__file__).with_name("prompt.md").read_text(encoding="utf-8")
    requested = changed_pages = 0
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
                    width, height = image.size
                context = {key: value for key, value in page.items() if key != "segments"}
                context.update(image_width=width, image_height=height, objects=objects)
                if codex is None:
                    codex = CodexClient(config, stage=TABLE_RECLASSIFY_STAGE, images=True)
                for target in candidates:
                    request = prompt + "\n\nFrozen source page and target table:\n" + json.dumps(
                        {**context, "target_table": target}, ensure_ascii=False)
                    response = codex.generate_json(request, schema=RESPONSE, image_path=source)
                    children = table_replacements(target, response, reserved_ids)
                    replacements[target["segment_id"]] = children
                    requested += 1
                    print(f"[table-reclassify] {page_id}/{target['segment_id']}: "
                          f"{response['action']} -> {[child['type'] for child in children]}", flush=True)
            segments = [child for obj in objects
                        for child in replacements.get(obj["segment_id"], [obj])]
            if segments != objects:
                write_json(output / path.name, {**page, "segments": segments})
                changed_pages += 1
            print(f"[table-reclassify] {page_id}: changed={segments != objects}", flush=True)
        print(f"[table-reclassify] input pages={len(paths)}; requested tables={requested}; "
              f"changed pages={changed_pages}")
    finally:
        if codex is not None:
            codex.close()


def main():
    parser = argparse.ArgumentParser(description="Stage 02.42: reclassify tables using original-page vision")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    reclassify_tables(args.workspace.resolve(), load_config(args.config, known_stages=PDF_STAGES),
                      pages=selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

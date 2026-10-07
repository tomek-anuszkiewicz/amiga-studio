#!/usr/bin/env python3
"""Stage 02.44: record clockwise image corrections without changing pixels."""

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
from common.pdf_page_conversion import resolved_page_files, IMAGE_ROTATION_STAGE
from common.pdf_schemas import object_schema
from common.pdf_selection import selected_pages

RESPONSE = object_schema({"rotation": {"type": "number", "minimum": 0, "exclusiveMaximum": 360}})


def detect_image_rotation(workspace, config, pages=None):
    paths = list(resolved_page_files(workspace, pages, before_stage="02.44"))
    output = workspace / IMAGE_ROTATION_STAGE
    output.mkdir(parents=True, exist_ok=True)
    prompt = Path(__file__).with_name("prompt.md").read_text(encoding="utf-8")
    requested = changed_pages = 0
    codex = None
    try:
        for path in paths:
            page = read_json(path)
            objects = page["segments"]
            candidates = [obj for obj in objects if obj["type"] == "graphic"]
            page_id = path.stem.removesuffix("_segments")
            replacements = {}
            if candidates:
                source = workspace / "01_preprocess" / f"{page_id}.png"
                with Image.open(source) as image:
                    width, height = image.size
                context = {key: value for key, value in page.items() if key != "segments"}
                context.update(image_width=width, image_height=height, objects=objects)
                if codex is None:
                    codex = CodexClient(config, stage=IMAGE_ROTATION_STAGE, images=True)
                for target in candidates:
                    request = prompt + "\n\nFrozen source page and target image:\n" + json.dumps(
                        {**context, "target_image": target}, ensure_ascii=False)
                    response = codex.generate_json(request, schema=RESPONSE, image_path=source)
                    rotation = response["rotation"]
                    if "rotation" not in target or target["rotation"] != rotation:
                        replacements[target["segment_id"]] = {
                            **target, "rotation": rotation,
                            "source_segment_ids": target.get("source_segment_ids", [target["segment_id"]]),
                            "replacement_stage": IMAGE_ROTATION_STAGE,
                        }
                    requested += 1
                    print(f"[image-rotation] {page_id}/{target['segment_id']}: {rotation} deg CW", flush=True)
            segments = [replacements.get(obj["segment_id"], obj) for obj in objects]
            if segments != objects:
                write_json(output / path.name, {**page, "segments": segments})
                changed_pages += 1
            print(f"[image-rotation] {page_id}: graphics={len(candidates)}; changed={segments != objects}", flush=True)
        print(f"[image-rotation] input pages={len(paths)}; requested graphics={requested}; "
              f"changed pages={changed_pages}; requests={codex.call_count if codex else 0}; "
              f"cache hits={codex.cached_call_count if codex else 0}")
    finally:
        if codex is not None:
            codex.close()


def main():
    parser = argparse.ArgumentParser(description="Stage 02.44: record clockwise image rotation")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    detect_image_rotation(args.workspace.resolve(), load_config(args.config, known_stages=PDF_STAGES),
                          pages=selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

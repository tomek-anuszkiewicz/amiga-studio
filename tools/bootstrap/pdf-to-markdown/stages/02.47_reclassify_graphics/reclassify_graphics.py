#!/usr/bin/env python3
"""Stage 02.47: reconsider existing graphics using original-page vision."""

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
from common.pdf_page_conversion import resolved_page_files, GRAPHIC_RECLASSIFY_STAGE
from common.pdf_schemas import object_schema, STRING
from common.pdf_selection import selected_pages

RESPONSE = object_schema({
    "action": {"type": "string", "enum": ["retain", "prose", "table", "code_block"]},
    "md_text": STRING,
})


def reclassify_graphics(workspace, config, pages=None):
    paths = list(resolved_page_files(workspace, pages, before_stage="02.47"))
    output = workspace / GRAPHIC_RECLASSIFY_STAGE
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
                    codex = CodexClient(config, stage=GRAPHIC_RECLASSIFY_STAGE, images=True)
                for target in candidates:
                    request = prompt + "\n\nFrozen source page and target graphic:\n" + json.dumps(
                        {**context, "target_graphic": target}, ensure_ascii=False)
                    response = codex.generate_json(request, schema=RESPONSE, image_path=source)
                    action = response["action"]
                    if action != "retain":
                        replacements[target["segment_id"]] = {
                            **target, "type": action,
                            "md_text": response["md_text"] if action in {"prose", "code_block"} else "",
                            "heading_level": None,
                        }
                        replacements[target["segment_id"]].pop("rotation", None)
                    requested += 1
                    print(f"[graphic-reclassify] {page_id}/{target['segment_id']}: {action}", flush=True)
            segments = [replacements.get(obj["segment_id"], obj) for obj in objects]
            if segments != objects:
                write_json(output / path.name, {**page, "segments": segments})
                changed_pages += 1
            print(f"[graphic-reclassify] {page_id}: changed={segments != objects}", flush=True)
        print(f"[graphic-reclassify] input pages={len(paths)}; requested graphics={requested}; "
              f"changed pages={changed_pages}")
    finally:
        if codex is not None:
            codex.close()


def main():
    parser = argparse.ArgumentParser(description="Stage 02.47: reclassify graphics using original-page vision")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    reclassify_graphics(args.workspace.resolve(), load_config(args.config, known_stages=PDF_STAGES),
                           pages=selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Stage 02.45: recover source code misclassified as prose."""

import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion import CodexClient
from conversion.config import load_config, PDF_STAGES
from common.pdf_artifacts import read_json, write_json
from common.pdf_page_conversion import resolved_page_files, PROSE_CODE_STAGE
from common.pdf_schemas import object_schema, STRING
from common.pdf_selection import selected_pages

RESPONSE = object_schema({"replacements": {"type": "array", "items": object_schema({
    "segment_id": STRING, "md_text": STRING,
})}})


def recover_code_from_prose(workspace, config, pages=None):
    paths = list(resolved_page_files(workspace, pages, before_stage="02.45"))
    output = workspace / PROSE_CODE_STAGE
    output.mkdir(parents=True, exist_ok=True)
    prompt = Path(__file__).with_name("prompt.md").read_text(encoding="utf-8")
    requested = changed_pages = 0
    codex = None
    try:
        for path in paths:
            page = read_json(path)
            objects = page["segments"]
            candidates = {obj["segment_id"]: obj for obj in objects if obj["type"] == "prose"}
            page_id = path.stem.removesuffix("_segments")
            replacements = {}
            if candidates:
                source = workspace / "01_preprocess" / f"{page_id}.png"
                if codex is None:
                    codex = CodexClient(config, stage=PROSE_CODE_STAGE, images=True)
                request = prompt + "\n\nFrozen resolved page JSON and eligible prose IDs:\n" + json.dumps(
                    {"page_json": page, "eligible_prose_ids": list(candidates)}, ensure_ascii=False)
                response = codex.generate_json(request, schema=RESPONSE, image_path=source)
                for replacement in response["replacements"]:
                    identity = replacement["segment_id"]
                    if identity not in candidates:
                        print(f"[prose-code] {page_id}/{identity}: ignored non-eligible ID", flush=True)
                        continue
                    target = candidates[identity]
                    replacements[identity] = {
                        **target, "type": "code_block", "md_text": replacement["md_text"],
                        "source_segment_ids": target.get("source_segment_ids", [identity]),
                        "replacement_stage": PROSE_CODE_STAGE,
                    }
                requested += 1
            segments = [replacements.get(obj["segment_id"], obj) for obj in objects]
            if segments != objects:
                write_json(output / path.name, {**page, "segments": segments})
                changed_pages += 1
            print(f"[prose-code] {page_id}: prose={len(candidates)}; "
                  f"changed IDs={list(replacements)}", flush=True)
        print(f"[prose-code] input pages={len(paths)}; requested pages={requested}; "
              f"changed pages={changed_pages}; requests={codex.call_count if codex else 0}; "
              f"cache hits={codex.cached_call_count if codex else 0}")
    finally:
        if codex is not None:
            codex.close()


def main():
    parser = argparse.ArgumentParser(description="Stage 02.45: recover source code classified as prose")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    recover_code_from_prose(args.workspace.resolve(), load_config(args.config, known_stages=PDF_STAGES),
                            pages=selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

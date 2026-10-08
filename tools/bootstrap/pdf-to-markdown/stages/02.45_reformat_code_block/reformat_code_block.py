#!/usr/bin/env python3
"""Stage 02.45: reformat existing code blocks using resolved page JSON."""

import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion import CodexClient
from conversion.config import load_config, PDF_STAGES
from common.pdf_artifacts import read_json, write_json
from common.pdf_page_conversion import resolved_page_files, CODE_REFORMAT_STAGE
from common.pdf_schemas import object_schema, STRING
from common.pdf_selection import selected_pages

RESPONSE = object_schema({"md_text": STRING})


def reformat_code_block(workspace, config, pages=None):
    paths = list(resolved_page_files(workspace, pages, before_stage="02.45"))
    output = workspace / CODE_REFORMAT_STAGE
    output.mkdir(parents=True, exist_ok=True)
    prompt = Path(__file__).with_name("prompt.md").read_text(encoding="utf-8")
    requested = changed_pages = 0
    codex = None
    try:
        for path in paths:
            page = read_json(path)
            objects = page["segments"]
            candidates = [obj for obj in objects if obj["type"] == "code_block"]
            page_id = path.stem.removesuffix("_segments")
            replacements = {}
            if candidates:
                if codex is None:
                    codex = CodexClient(config, stage=CODE_REFORMAT_STAGE)
                for target in candidates:
                    identity = target["segment_id"]
                    request = prompt + "\n\nFrozen resolved page JSON and target code block:\n" + json.dumps(
                        {"page_json": page, "target_code_block": target}, ensure_ascii=False)
                    response = codex.generate_json(request, schema=RESPONSE)
                    replacements[identity] = {
                        **target, "md_text": response["md_text"],
                        "source_segment_ids": target.get("source_segment_ids", [identity]),
                        "replacement_stage": CODE_REFORMAT_STAGE,
                    }
                    requested += 1
                    print(f"[code-reformat] {page_id}/{identity}: "
                          f"text changed={response['md_text'] != target['md_text']}", flush=True)
            segments = [replacements.get(obj["segment_id"], obj) for obj in objects]
            if segments != objects:
                write_json(output / path.name, {**page, "segments": segments})
                changed_pages += 1
            print(f"[code-reformat] {page_id}: code blocks={len(candidates)}; "
                  f"formatted IDs={list(replacements)}", flush=True)
        print(f"[code-reformat] input pages={len(paths)}; requested code blocks={requested}; "
              f"changed pages={changed_pages}; requests={codex.call_count if codex else 0}; "
              f"cache hits={codex.cached_call_count if codex else 0}")
    finally:
        if codex is not None:
            codex.close()


def main():
    parser = argparse.ArgumentParser(description="Stage 02.45: reformat existing code blocks")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    reformat_code_block(args.workspace.resolve(), load_config(args.config, known_stages=PDF_STAGES),
                            pages=selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Stage 02.4: vision-assisted advisory recovery with sparse page overrides."""

import argparse
import json
from pathlib import Path
import sys
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion import CodexClient
from conversion.config import load_config, PDF_STAGES
from common.pdf_artifacts import read_json, write_json, page_files
from common.pdf_page_conversion import CALLOUT_STAGE
from common.pdf_callouts import RESPONSE, advisory_keywords, request_objects, apply_replacements


def reclassify_callouts(workspace, config):
    paths = page_files(workspace / "02_page_conversion", "page_*_segments.json")
    pages = [read_json(path) for path in paths]
    keywords = advisory_keywords(pages)
    output = workspace / CALLOUT_STAGE
    output.mkdir(parents=True, exist_ok=True)
    prompt = Path(__file__).with_name("prompt.md").read_text(encoding="utf-8")
    hits = changed = requested = 0
    # Open transport lazily: a fragment with no candidates needs no model call.
    codex = None
    try:
        for path, page in zip(paths, pages):
            objects = request_objects(page, keywords)
            candidates = [{"segment_id": obj["segment_id"], "keywords": obj["candidate_keywords"]}
                          for obj in objects if obj["candidate_keywords"]]
            page_id = path.stem.removesuffix("_segments")
            page_changed = False
            if candidates:
                source = workspace / "01_preprocess" / f"{page_id}.png"
                # Missing or unreadable imagery fails instead of using text alone.
                with Image.open(source) as image:
                    image.load()
                    width, height = image.size
                context = {key: value for key, value in page.items() if key != "segments"}
                context.update(image_width=width, image_height=height,
                               objects=objects, candidates=candidates)
                request = prompt + "\n\nFrozen source page:\n" + json.dumps(context, ensure_ascii=False)
                if codex is None:
                    codex = CodexClient(config, stage=CALLOUT_STAGE, images=True)
                response = codex.generate_json(request, schema=RESPONSE, image_path=source)
                corrected, edit_reports = apply_replacements(page, objects, response)
                for report in edit_reports:
                    if report["status"] == "retained":
                        print(f"[callouts] {page_id}: retained range: {report['reason']}")
                page_changed = corrected != page
                if page_changed:
                    write_json(output / path.name, corrected)
                    changed += 1
                requested += 1
                hits += len(candidates)
            print(f"[callouts] {page_id}: candidates={len(candidates)}; changed={page_changed}")
        print(f"[callouts] input pages={len(pages)}; candidate objects={hits}; "
              f"requested pages={requested}; changed pages={changed}")
    finally:
        if codex is not None:
            codex.close()


def main():
    parser = argparse.ArgumentParser(description="Stage 02.4: recover complete advisory ranges with original-page vision")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args()
    reclassify_callouts(args.workspace.resolve(), load_config(args.config, known_stages=PDF_STAGES))


if __name__ == "__main__":
    main()

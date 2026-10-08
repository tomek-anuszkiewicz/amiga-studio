#!/usr/bin/env python3
"""Stage 02.46: review suspicious side-by-side order on complete pages."""

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
from common.pdf_frames import draw_review_image
from common.pdf_page_conversion import resolved_page_files, READING_ORDER_STAGE
from common.pdf_schemas import object_schema, STRING
from common.pdf_selection import selected_pages

RESPONSE = object_schema({"ordered_segment_ids": {"type": "array", "items": STRING}})


def suspicious_pairs(objects):
    """Flag right-before-left pairs with overlapping vertical intervals."""
    pairs = []
    for index, earlier in enumerate(objects):
        left, top, _, bottom = earlier["bbox"]
        for later_index in range(index + 1, len(objects)):
            later = objects[later_index]
            _, later_top, later_right, later_bottom = later["bbox"]
            if left >= later_right and top < later_bottom and later_top < bottom:
                pairs.append({"earlier_ordinal": index + 1,
                              "earlier_segment_id": earlier["segment_id"],
                              "later_ordinal": later_index + 1,
                              "later_segment_id": later["segment_id"]})
    return pairs


def reordered_objects(objects, ordered_ids):
    """Apply only a complete permutation, moving the original objects."""
    by_id = {obj["segment_id"]: obj for obj in objects}
    if (not isinstance(ordered_ids, list) or any(not isinstance(identity, str) for identity in ordered_ids)
            or len(by_id) != len(objects) or len(ordered_ids) != len(objects)
            or len(set(ordered_ids)) != len(ordered_ids) or set(ordered_ids) != set(by_id)):
        raise ValueError("Reading-order response must be a permutation of current segment IDs")
    return [by_id[identity] for identity in ordered_ids]


def review_reading_order(workspace, config, pages=None):
    paths = list(resolved_page_files(workspace, pages, before_stage="02.46"))
    output = workspace / READING_ORDER_STAGE
    diagnostics = output / "diagnostics"
    diagnostics.mkdir(parents=True, exist_ok=True)
    prompt = Path(__file__).with_name("prompt.md").read_text(encoding="utf-8")
    requested = changed_pages = 0
    codex = None
    try:
        for path in paths:
            page = read_json(path)
            objects = page["segments"]
            before = [obj["segment_id"] for obj in objects]
            pairs = suspicious_pairs(objects)
            page_id = path.stem.removesuffix("_segments")
            report = {"page": page["page"], "suspicious_pairs": pairs,
                      "before_segment_ids": before, "after_segment_ids": before,
                      "requested": bool(pairs), "status": "not_suspicious"}
            if pairs:
                source = workspace / "01_preprocess" / f"{page_id}.png"
                annotated = diagnostics / f"{page_id}_input.png"
                # Fresh labels correspond one-to-one to the frozen resolved array.
                annotations = [{**obj, "source_ordinals": [index + 1]}
                               for index, obj in enumerate(objects)]
                with Image.open(source) as image:
                    width, height = image.size
                    raster = {"display_to_pixels": [1, 0, 0, 1, 0, 0],
                              "pixel_width": width, "pixel_height": height}
                    draw_review_image(image, {"raster": raster}, annotations,
                                      include_segment_ids=True).save(annotated)
                report["status"] = "requesting"
                write_json(diagnostics / f"{page_id}_order.json", report)
                if codex is None:
                    codex = CodexClient(config, stage=READING_ORDER_STAGE, images=True)
                request = prompt + "\n\nFrozen resolved page and order evidence:\n" + json.dumps(
                    {"page_json": page, "current_order": [
                        {"ordinal": index + 1, "segment_id": obj["segment_id"], "bbox": obj["bbox"]}
                        for index, obj in enumerate(objects)], "suspicious_pairs": pairs}, ensure_ascii=False)
                response = codex.generate_json(request, schema=RESPONSE,
                                               image_path=[source, annotated])
                segments = reordered_objects(objects, response["ordered_segment_ids"])
                requested += 1
                report.update(after_segment_ids=response["ordered_segment_ids"], status="success")
                if segments != objects:
                    write_json(output / path.name, {**page, "segments": segments})
                    changed_pages += 1
            write_json(diagnostics / f"{page_id}_order.json", report)
            print(f"[reading-order] {page_id}: pairs={pairs}; "
                  f"before={before}; after={report['after_segment_ids']}", flush=True)
        print(f"[reading-order] input pages={len(paths)}; reviewed pages={requested}; "
              f"changed pages={changed_pages}; requests={codex.call_count if codex else 0}; "
              f"cache hits={codex.cached_call_count if codex else 0}")
    finally:
        if codex is not None:
            codex.close()


def main():
    parser = argparse.ArgumentParser(description="Stage 02.46: review suspicious page reading order")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    review_reading_order(args.workspace.resolve(), load_config(args.config, known_stages=PDF_STAGES),
                         pages=selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

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
from common.pdf_callouts import RESPONSE, advisory_keywords, request_frames, apply_replacements
from common.pdf_frames import draw_review_image

FRAME_COLORS = ("#1764a0", "#ad5b00", "#23783a", "#a62d87", "#008484", "#73203d")


def framed_request(image, frames):
    """One frame per exact request object; colors convey identity, never role."""
    width, height = image.size
    raster = {"display_to_pixels": [1, 0, 0, 1, 0, 0],
              "pixel_width": width, "pixel_height": height}
    labels = [f"{frame['frame_id']} = {frame['segment_id']}" +
              (" [read-only]" if frame["read_only"] else "") for frame in frames]
    return draw_review_image(image, {"raster": raster}, frames, labels=labels,
                             colors=[FRAME_COLORS[i % len(FRAME_COLORS)] for i in range(len(frames))],
                             leaders=False)


def reclassify_callouts(workspace, config):
    paths = page_files(workspace / "02_page_conversion", "page_*_segments.json")
    pages = [read_json(path) for path in paths]
    keywords = advisory_keywords(pages)
    output = workspace / CALLOUT_STAGE
    diagnostics = output / "diagnostics"
    diagnostics.mkdir(parents=True, exist_ok=True)
    prompt = Path(__file__).with_name("prompt.md").read_text(encoding="utf-8")
    hits = changed = requested = 0
    # Open transport lazily: a fragment with no candidates needs no model call.
    codex = None
    try:
        for path, page in zip(paths, pages):
            frames = request_frames(page, keywords)
            candidates = [frame["frame_id"] for frame in frames if frame["candidate_keywords"]]
            page_id = path.stem.removesuffix("_segments")
            report = {"page": page["page"], "keywords": keywords, "frames": frames,
                      "candidate_frame_ids": candidates, "decisions": [], "replacements": [],
                      "edit_reports": [], "changed": False}
            if candidates:
                source = workspace / "01_preprocess" / f"{page_id}.png"
                companion = diagnostics / f"{page_id}_frames.png"
                # Missing or unreadable imagery fails instead of using text alone.
                with Image.open(source) as image:
                    framed_request(image, frames).save(companion)
                context = {key: value for key, value in page.items() if key != "segments"}
                context.update(frames=frames, candidate_frame_ids=candidates, keywords=keywords)
                request = prompt + "\n\nFrozen source page and frame map:\n" + json.dumps(context, ensure_ascii=False)
                if codex is None:
                    codex = CodexClient(config, stage=CALLOUT_STAGE, images=True)
                response = codex.generate_json(request, schema=RESPONSE, image_path=[source, companion])
                write_json(diagnostics / f"{page_id}_response.json", response)
                corrected, edit_reports = apply_replacements(page, frames, response)
                report.update(decisions=response.get("decisions", []), replacements=response.get("replacements", []),
                              edit_reports=edit_reports, source_image=f"01_preprocess/{source.name}",
                              framed_image=f"{CALLOUT_STAGE}/diagnostics/{companion.name}")
                report["changed"] = corrected != page
                if report["changed"]:
                    write_json(output / path.name, corrected)
                    changed += 1
                requested += 1
                hits += len(candidates)
            write_json(diagnostics / f"{page_id}_decisions.json", report)
            print(f"[callouts] {page_id}: candidates={len(candidates)}; changed={report['changed']}")
        write_json(output / "decision_report.json", {
            "input_pages": len(pages), "candidate_objects": hits, "requested_pages": requested,
            "changed_pages": changed, "keywords": keywords})
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

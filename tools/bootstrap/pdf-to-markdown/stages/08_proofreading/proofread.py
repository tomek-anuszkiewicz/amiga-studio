#!/usr/bin/env python3
"""Stage 08: proofread retained Stage 07 page text without changing page structure."""

import argparse
from copy import deepcopy
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from conversion import CodexClient
from conversion.parallel import convert_parallel
from common.pdf_artifacts import write_json
from common.pdf_config import load_config
from common.pdf_page_conversion import read_conversion
from common.pdf_selection import selected_pages

STAGE = "08_proofreading"
CONTEXT_CHARS = 600
FORMAT_FIELDS = ("table_format", "text_format", "heading_level", "continuation")


def project(segment):
    """Send only text and formatting metadata needed to proofread an object."""
    result = {key: segment.get(key) for key in ("segment_id", "type", "role", "md_text")}
    result.update({key: segment[key] for key in FORMAT_FIELDS if key in segment})
    return result


def boundary(page, *, tail):
    parts = [segment for segment in page["segments"] if isinstance(segment.get("md_text"), str)
             and segment["md_text"]]
    sequence = reversed(parts) if tail else iter(parts)
    excerpt = []
    remaining = CONTEXT_CHARS
    for segment in sequence:
        content = segment["md_text"]
        if remaining <= 0:
            break
        excerpt.append({"segment_id": segment["segment_id"],
                        "md_text": content[-remaining:] if tail else content[:remaining]})
        remaining -= len(excerpt[-1]["md_text"])
    if tail:
        excerpt.reverse()
    return {"page": page["page"], "segments": excerpt}


def request(prompt, page, previous=None, following=None, unresolved=None):
    payload = {"target_page": {"page": page["page"],
                               "segments": [project(segment) for segment in page["segments"]]},
               "previous_tail": boundary(previous, tail=True) if previous else None,
               "following_head": boundary(following, tail=False) if following else None}
    if unresolved is not None:
        segments = {segment["segment_id"]: segment for segment in page["segments"]}
        payload["initial_unresolved"] = [
            {**item, "bbox": segments[item["segment_id"]].get("bbox")}
            for item in unresolved
        ]
    return prompt + "\n\nPAGE INPUT JSON:\n" + json.dumps(payload, ensure_ascii=False)


def apply_changes(page, response):
    """Validate the complete response before updating any target text."""
    if not isinstance(response, dict) or set(response) != {"changes", "unresolved"}:
        raise ValueError("Proofreading response needs changes and unresolved lists")
    changes, unresolved = response["changes"], response["unresolved"]
    if not isinstance(changes, list) or not isinstance(unresolved, list):
        raise ValueError("Proofreading response lists are invalid")
    targets = {segment["segment_id"]: segment for segment in page["segments"]}
    if len(targets) != len(page["segments"]):
        raise ValueError("Target page has duplicate segment IDs")
    replacements = {}
    for change in changes:
        if not isinstance(change, dict) or set(change) != {"segment_id", "md_text"}:
            raise ValueError("Proofreading change has invalid fields")
        identifier, text = change["segment_id"], change["md_text"]
        if identifier not in targets or identifier in replacements:
            raise ValueError("Proofreading change targets an unknown or duplicate segment")
        if not isinstance(text, str) or not text.strip() or not isinstance(targets[identifier].get("md_text"), str):
            raise ValueError("Proofreading change must replace existing text with nonempty text")
        replacements[identifier] = text
    unresolved_ids = set()
    for item in unresolved:
        if (not isinstance(item, dict) or set(item) != {"segment_id", "reason"}
                or item["segment_id"] not in targets
                or item["segment_id"] in unresolved_ids
                or not isinstance(item["reason"], str) or not item["reason"].strip()):
            raise ValueError("Proofreading unresolved item is invalid")
        unresolved_ids.add(item["segment_id"])
    for identifier, text in replacements.items():
        targets[identifier]["md_text"] = text
    return len(replacements), unresolved


def apply_image_response(page, response, pending_ids):
    """Apply supported corrections and keep every other flagged segment unchanged."""
    if not isinstance(response, dict) or set(response) != {"changes", "unchanged"}:
        raise ValueError("Image proofreading response needs changes and unchanged lists")
    unchanged = response["unchanged"]
    if (not isinstance(unchanged, list) or any(
            not isinstance(item, dict) or set(item) != {"segment_id", "reason"}
            or not isinstance(item["segment_id"], str)
            or not isinstance(item["reason"], str) or not item["reason"].strip()
            for item in unchanged)):
        raise ValueError("Image proofreading unchanged items are invalid")
    candidate = deepcopy(page)
    apply_changes(candidate, {"changes": response["changes"], "unresolved": []})
    handled = ([item["segment_id"] for item in response["changes"]]
               + [item["segment_id"] for item in unchanged])
    if len(handled) != len(set(handled)) or set(handled) != pending_ids:
        raise ValueError("Image proofreading must address every initial unresolved segment once")
    page["segments"] = candidate["segments"]
    return unchanged


def proofread(workspace, config, pages=None, client_factory=CodexClient):
    prompt = (Path(__file__).parent / "prompt.md").read_text(encoding="utf-8")
    image_prompt = (Path(__file__).parent / "prompt_image.md").read_text(encoding="utf-8")
    retained = list(read_conversion(workspace, "07_repair_callouts"))
    selected = set(pages) if pages is not None else None
    jobs = [(entry, page, retained[index - 1][1] if index else None,
             retained[index + 1][1] if index + 1 < len(retained) else None)
            for index, (entry, page) in enumerate(retained)
            if selected is None or page["page"] in selected]
    output = workspace / STAGE
    output.mkdir(parents=True, exist_ok=True)
    (output / "unresolved_summary.md").unlink(missing_ok=True)
    (output / "proofreading_summary.md").unlink(missing_ok=True)
    for audit_path in output.glob("page_*_proofreading.json"):
        audit_path.unlink()

    def convert(client, job):
        entry, page, previous, following = job
        print(f"[proofreading] {entry['page_id']}: requesting page correction", flush=True)
        calls, cached = client.call_count, client.cached_call_count
        try:
            response = client.generate_json(request(prompt, page, previous, following))
        except Exception as error:
            error.add_note(f"Proofreading failed: {entry['page_id']}")
            raise
        print(f"[proofreading] {entry['page_id']}: response received; "
              f"calls={client.call_count - calls}, cache_hits={client.cached_call_count - cached}", flush=True)
        candidate = deepcopy(page)
        _, initial_unresolved = apply_changes(candidate, response)
        if initial_unresolved:
            image_path = workspace / "02_preprocess" / f"{entry['page_id']}.png"
            if not image_path.is_file():
                raise FileNotFoundError(f"Missing Stage 02 page image for {entry['page_id']}: {image_path}")
            print(f"[proofreading] {entry['page_id']}: requesting page correction with image; "
                  f"image={image_path.name}", flush=True)
            calls, cached = client.call_count, client.cached_call_count
            try:
                image_response = client.generate_json(
                    request(image_prompt, candidate, previous, following, initial_unresolved),
                    image_path=image_path)
            except Exception as error:
                error.add_note(f"Image proofreading failed: {entry['page_id']}")
                raise
            print(f"[proofreading] {entry['page_id']}: image response received; "
                  f"calls={client.call_count - calls}, cache_hits={client.cached_call_count - cached}", flush=True)
            apply_image_response(candidate, image_response,
                                 {item["segment_id"] for item in initial_unresolved})
        return candidate

    def accept(job, result):
        entry, _, _, _ = job
        write_json(output / f"{entry['page_id']}_segments.json", result)
        print(f"[proofreading] {entry['page_id']}: done", flush=True)

    convert_parallel(config, stage=STAGE, operation="pdf_page_proofreading", jobs=jobs,
                     convert=convert, accept=accept, client_factory=client_factory)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    proofread(args.workspace.resolve(), load_config(args.config), selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

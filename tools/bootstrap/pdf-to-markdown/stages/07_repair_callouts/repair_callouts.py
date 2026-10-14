#!/usr/bin/env python3
"""Stage 07: assemble source advisory labels and bodies into Markdown callouts."""

import argparse
from copy import deepcopy
from pathlib import Path
import re
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from common.pdf_artifacts import read_json, write_json
from common.pdf_config import load_config
from common.pdf_page_conversion import read_conversion
from common.pdf_selection import selected_pages

STAGE = "07_repair_callouts"


def repair_page(page):
    """Keep page order and source regions; warn instead of guessing a pairing."""
    segments = page["segments"]
    retained = []
    warnings = []
    fixed = index = 0
    while index < len(segments):
        element = segments[index]
        retained.append(element)
        index += 1
        text = element.get("md_text", "")
        if element["type"] == "callout_text":
            warnings.append(f"{element['segment_id']}: callout text has no preceding callout label")
            continue
        if element["type"] != "callout" or re.match(r"^\s*>\s*\[!", text):
            continue
        if index == len(segments) or segments[index]["type"] != "callout_text":
            warnings.append(f"{element['segment_id']}: callout has no immediately following body on this page")
            continue
        body = segments[index]
        if element['role'] != body['role']:
            warnings.append(f"{element['segment_id']}: role differs from body {body['segment_id']}; left unchanged")
            continue
        label_text = re.sub(r"^(\*\*|__|\*|_)(.*)\1$", r"\2", text.strip())
        label = re.fullmatch(r"([A-Za-z][A-Za-z0-9_-]*):?", label_text)
        body_text = body.get("md_text", "")
        if not label or not body_text.strip():
            warnings.append(f"{element['segment_id']}: invalid label or empty body {body['segment_id']}")
            continue
        element["md_text"] = "> [!" + label[1].upper() + "] " + text.strip() + "\n>\n" + "".join(
            "> " + line for line in body_text.splitlines(keepends=True))
        regions = []
        for source in (element, body):
            for region in source.get("pdf_regions", [{"page": page["page"], "bbox": source["bbox"]}]):
                if region not in regions:
                    regions.append(deepcopy(region))
        element["pdf_regions"] = regions
        first, second = element["bbox"], body["bbox"]
        element["bbox"] = [min(first[0], second[0]), min(first[1], second[1]),
                           max(first[2], second[2]), max(first[3], second[3])]
        index += 1
        fixed += 1
    page["segments"] = retained
    return fixed, warnings


def repair_callouts(workspace, pages=None):
    output = workspace / STAGE
    output.mkdir(parents=True, exist_ok=True)
    fixed = saved = warning_count = 0
    for entry, page in read_conversion(workspace, "04_filter_page_content", pages=pages):
        replacement = workspace / "05_transform_page_tables" / f"{entry['page_id']}_segments.json"
        if replacement.exists():
            page = read_json(replacement)
        count, warnings = repair_page(page)
        fixed += count
        warning_count += len(warnings)
        for warning in warnings:
            print(f"[callout warning] {warning}", flush=True)
        write_json(output / f"{entry['page_id']}_segments.json", page)
        saved += 1
    print(f"[callouts] saved {saved} pages; fixed {fixed} pairs; warnings {warning_count}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    load_config(args.config)
    repair_callouts(args.workspace.resolve(), pages=selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

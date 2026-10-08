#!/usr/bin/env python3
"""Stage 02.48: deterministically order objects within anchored page rows."""

import argparse
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from common.pdf_artifacts import read_json, write_json
from common.pdf_page_conversion import resolved_page_files, ROW_ORDER_STAGE
from common.pdf_selection import selected_pages

ROW_TOP_TOLERANCE_PX = 7


def order_row_segments(objects):
    """Move original objects only among their row's occupied array positions."""
    ordered = list(objects)
    by_top = sorted(enumerate(objects), key=lambda item: (item[1]["bbox"][1], item[0]))
    groups = 0
    start = 0
    while start < len(by_top):
        anchor = by_top[start][1]["bbox"][1]
        end = start + 1
        while end < len(by_top) and by_top[end][1]["bbox"][1] - anchor <= ROW_TOP_TOLERANCE_PX:
            end += 1
        members = by_top[start:end]
        if len(members) > 1:
            groups += 1
            positions = sorted(position for position, _ in members)
            row = sorted(members, key=lambda item: (item[1]["bbox"][0], item[0]))
            for position, (_, obj) in zip(positions, row):
                ordered[position] = obj
        start = end
    return ordered, groups


def order_page_rows(workspace, pages=None):
    paths = list(resolved_page_files(workspace, pages, before_stage="02.48"))
    output = workspace / ROW_ORDER_STAGE
    output.mkdir(parents=True, exist_ok=True)
    groups = changed_pages = 0
    for path in paths:
        page = read_json(path)
        objects = page["segments"]
        ordered, count = order_row_segments(objects)
        groups += count
        if ordered != objects:
            page["segments"] = ordered
            write_json(output / path.name, page)
            changed_pages += 1
    print(f"[row-order] {groups} multi-object groups; {changed_pages}/{len(paths)} changed pages; JSON: {output}")


def main():
    parser = argparse.ArgumentParser(description="Stage 02.48: order objects within page rows")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    order_page_rows(args.workspace.resolve(), selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Remove empty table rows without inference or changing predecessor artifacts."""

import argparse
from pathlib import Path
import re
import shutil
import sys

from bs4 import BeautifulSoup

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from common.pdf_artifacts import write_json
from common.pdf_page_conversion import read_conversion
from common.pdf_selection import selected_pages
from common.pdf_tables import STAGE, CLEANED_STAGE


def clean_markdown(markup):
    lines = markup.splitlines(keepends=True)
    # Keep the header, even when blank, and its required separator row.
    headers = {i - 1 for i, line in enumerate(lines) if i and re.fullmatch(
        r"\s*\|?\s*:?-{3,}:?\s*(?:\|\s*:?-{3,}:?\s*)+\|?\s*", line)}
    return "".join(line for i, line in enumerate(lines)
                   if i in headers or "|" not in line or line.strip(" \t\r\n|"))


def empty_cell(cell):
    return not cell.get_text().strip() and not cell.find(
        ["img", "svg", "math", "input", "video", "audio", "canvas", "iframe", "object", "embed"])


def clean_html(markup):
    soup = BeautifulSoup(markup, "html.parser")
    removed = False
    for table in soup.find_all("table"):
        rows = [row for row in table.find_all("tr") if row.find_parent("table") is table]
        spans = []
        remove = set()
        for i, row in enumerate(rows):
            cells = row.find_all(["td", "th"], recursive=False)
            section = row.parent
            group_end = next((j for j in range(i + 1, len(rows))
                              if rows[j].parent is not section), len(rows))
            for cell in cells:
                length = int(cell.get("rowspan", 1))
                # rowspan=0 extends to the end of the current row group.
                end = min(i + length, group_end) if length else group_end
                spans.append((i, end, cell))
            occupied = any(start < i < end and not empty_cell(cell)
                           for start, end, cell in spans)
            if not occupied and all(empty_cell(cell) for cell in cells):
                remove.add(i)
        for start, end, cell in spans:
            if start not in remove and "rowspan" in cell.attrs and cell["rowspan"] != "0":
                length = int(cell["rowspan"]) - sum(start < i < end for i in remove)
                if length != int(cell["rowspan"]):
                    cell["rowspan"] = str(length)
        for i in remove:
            rows[i].decompose()
            removed = True
    return str(soup) if removed else markup


def remove_empty_rows(workspace, pages=None):
    output = workspace / CLEANED_STAGE
    output.mkdir(parents=True, exist_ok=True)
    changed = 0
    for entry, page in read_conversion(workspace, STAGE, pages=pages):
        for table in page["segments"]:
            if table["type"] != "table":
                continue
            cleaner = {"markdown": clean_markdown, "html": clean_html}.get(table.get("table_format"))
            if cleaner:
                original = table["md_text"]
                table["md_text"] = cleaner(original)
                changed += table["md_text"] != original
            asset = table.get("table_source_asset")
            if asset:
                destination = output / asset
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(workspace / STAGE / asset, destination)
        write_json(output / f"{entry['page_id']}_segments.json", page)
    print(f"[tables] Removed empty rows in {changed} tables; JSON/assets: {output}")


def main():
    parser = argparse.ArgumentParser(description="Stage 02.82: remove empty table rows")
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--page-ranges")
    args = parser.parse_args()
    remove_empty_rows(args.workspace.resolve(), selected_pages(args.page_ranges))


if __name__ == "__main__":
    main()

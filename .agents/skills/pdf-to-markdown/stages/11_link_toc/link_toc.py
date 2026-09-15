#!/usr/bin/env python3
"""
stages/11_link_toc/link_toc.py:
Dedicated fuzzy cross-document Table of Contents linker:
1. Locates blocks demarcated by <!-- TOC34534 --> and <!-- /TOC34534 -->.
2. Catalogs all Markdown headers across all emitted files in <output_dir>.
3. Fuzzy matches each TOC line against cataloged headers to find the best match.
4. Converts each TOC line into an Obsidian cross-file wikilink: [[file#header|title]].
5. Completely strips the <!-- TOC34534 --> and <!-- /TOC34534 --> markers.
"""

import argparse
import difflib
import json
import re
import sys
from pathlib import Path
import yaml


TOC_START_MARKER = "<!-- TOC34534 -->"
TOC_END_MARKER = "<!-- /TOC34534 -->"


def catalog_headers_across_documents(output_dir: Path) -> list:
    """
    Extracts all markdown headers from all .md files in output_dir.
    Returns list of dicts: [{'file': '02_the_copper.md', 'stem': '02_the_copper', 'header': 'Copper Registers', 'level': 2}]
    """
    header_catalog = []
    md_files = sorted(list(output_dir.glob("*.md")))

    for md_path in md_files:
        stem = md_path.stem
        with open(md_path, "r", encoding="utf-8") as f:
            lines = f.readlines()

        for line in lines:
            m = re.match(r"^(#{1,6})\s+(.+)$", line.strip())
            if m:
                level = len(m.group(1))
                h_text = m.group(2).strip()
                # Clean header (remove bold/italic)
                clean_h = re.sub(r"[*_`]", "", h_text)
                header_catalog.append({
                    "file": md_path.name,
                    "stem": stem,
                    "header": clean_h,
                    "raw_header": h_text,
                    "level": level
                })

    return header_catalog


def find_best_header_match(query: str, catalog: list, cutoff: float = 0.5) -> dict:
    """
    Performs fuzzy matching of query string against cataloged headers.
    """
    cleaned_query = re.sub(r"^[-*\d.\s]+", "", query).strip().lower()
    if not cleaned_query:
        return None

    best_entry = None
    best_score = 0.0

    for entry in catalog:
        candidate = entry["header"].lower()
        score = difflib.SequenceMatcher(None, cleaned_query, candidate).ratio()

        # Bonus if query is exact substring
        if cleaned_query in candidate or candidate in cleaned_query:
            score = max(score, 0.85)

        if score > best_score and score >= cutoff:
            best_score = score
            best_entry = entry

    return best_entry


def link_toc_in_file(md_path: Path, catalog: list) -> bool:
    with open(md_path, "r", encoding="utf-8") as f:
        content = f.read()

    if TOC_START_MARKER not in content or TOC_END_MARKER not in content:
        return False

    pattern = re.compile(rf"{re.escape(TOC_START_MARKER)}(.*?){re.escape(TOC_END_MARKER)}", re.DOTALL)
    match = pattern.search(content)
    if not match:
        return False

    toc_body = match.group(1)
    toc_lines = toc_body.splitlines()
    linked_lines = []

    for line in toc_lines:
        stripped = line.strip()
        if not stripped:
            linked_lines.append(line)
            continue

        # Extract indent and list bullet
        bullet_match = re.match(r"^(\s*[-*]\s+)(.*)$", line)
        if bullet_match:
            prefix = bullet_match.group(1)
            title = bullet_match.group(2).strip()
        else:
            prefix = "- "
            title = stripped

        match_entry = find_best_header_match(title, catalog)
        if match_entry:
            file_stem = match_entry["stem"]
            target_h = match_entry["header"]
            # Create Obsidian wikilink
            linked_line = f"{prefix}[[{file_stem}#{target_h}|{title}]]"
            linked_lines.append(linked_line)
        else:
            linked_lines.append(line)

    new_toc_block = "\n".join(linked_lines)
    # Replace entire marker block with just the linked lines (markers removed!)
    updated_content = content[:match.start()] + new_toc_block + content[match.end():]

    with open(md_path, "w", encoding="utf-8") as f:
        f.write(updated_content)

    return True


def process_toc_linking(output_dir: Path, workspace_dir: Path):
    if not output_dir.exists():
        raise FileNotFoundError(f"Output directory does not exist: {output_dir}")

    print(f"[*] Cataloging headers across documents in {output_dir}...")
    catalog = catalog_headers_across_documents(output_dir)
    print(f"    Cataloged {len(catalog)} headers.")

    md_files = sorted(list(output_dir.glob("*.md")))
    linked_count = 0

    for md_path in md_files:
        if link_toc_in_file(md_path, catalog):
            print(f"[+] Converted TOC wikilinks and removed markers in {md_path.name}")
            linked_count += 1

    print(f"[+] Stage 11 complete. Processed {linked_count} TOC sections.")


def main():
    parser = argparse.ArgumentParser(description="Stage 11: Fuzzy cross-document Table of Contents linker")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--output-dir", type=str, default="output_markdown", help="Output directory")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)
    output_dir = Path(args.output_dir)

    process_toc_linking(output_dir, workspace_dir)


if __name__ == "__main__":
    main()

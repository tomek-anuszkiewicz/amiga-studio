#!/usr/bin/env python3
"""
stages/14_link_toc/link_toc.py:
Dedicated fuzzy cross-document Table of Contents linker:
1. Locates blocks demarcated by <!-- TOC34534 --> and <!-- /TOC34534 -->.
2. Catalogs all Markdown headers across all emitted files in <output_dir>.
3. Fuzzy matches each TOC line against cataloged headers to find the best match.
4. Converts each TOC line into an Obsidian cross-file wikilink: [[file#header|title]].
5. Completely strips the <!-- TOC34534 --> and <!-- /TOC34534 --> markers.
"""

import argparse
import difflib
import os
import re
import shutil
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


def norm_alphanumeric(s: str) -> str:
    """Returns lowercase string containing only ASCII letters and digits [a-z0-9]."""
    return re.sub(r"[^a-z0-9]+", "", s.lower())


PAGE_NUM_PATTERN = re.compile(r"(?:[\s.…_-]+|\s*[([])\b(\d{1,4})\b[)\]]?\s*$")


def strip_page_number(title: str) -> str:
    """Strips trailing printed page numbers (e.g. 'Chapter 2 ... 13' -> 'Chapter 2 ...')."""
    return PAGE_NUM_PATTERN.sub("", title).strip()


def should_strip_page_numbers(lines: list) -> bool:
    """Returns True if the majority (>= 40%) of bullet lines end with numbers."""
    bullet_lines = []
    for line in lines:
        m = re.match(r"^\s*[-*]\s+(.+)$", line)
        if m:
            bullet_lines.append(m.group(1).strip())
    if not bullet_lines:
        return False
    numbered = sum(1 for b in bullet_lines if PAGE_NUM_PATTERN.search(b))
    return (numbered / len(bullet_lines)) >= 0.40


def find_best_header_match(
    query: str,
    catalog: list,
    used_headers: set = None,
    preferred_stem: str = None,
    cutoff: float = 0.75
) -> dict:
    """
    Performs fuzzy matching of query string against cataloged headers.
    Ensures that headers in used_headers are NEVER matched a second time.
    If preferred_stem is provided, only considers headers from that document.
    """
    cleaned_query = re.sub(r"^[-*\d.\s]+", "", query).strip()
    if not cleaned_query:
        return None

    def norm(s: str) -> str:
        return re.sub(r"[^a-z0-9]+", " ", s.lower()).strip()

    norm_q = norm(cleaned_query)
    alpha_q = norm_alphanumeric(cleaned_query)
    if not norm_q and not alpha_q:
        return None

    best_entry = None
    best_score = 0.0

    for entry in catalog:
        key = (entry["stem"], entry["header"])
        if used_headers is not None and key in used_headers:
            continue

        if preferred_stem is not None and entry["stem"] != preferred_stem:
            continue

        cand_raw = entry["header"]
        norm_c = norm(cand_raw)
        alpha_c = norm_alphanumeric(cand_raw)
        if not norm_c and not alpha_c:
            continue

        # 1. Exact alphanumeric match (ignoring all punctuation, spaces, and formatting)
        if alpha_q and alpha_q == alpha_c:
            score = 1.0
        elif norm_q == norm_c:
            score = 1.0
        else:
            score = difflib.SequenceMatcher(None, norm_q, norm_c).ratio()
            # Award substring bonus if query and candidate are closely sized and meaningful
            if len(norm_q) >= 6 and len(norm_c) >= 6:
                if norm_q in norm_c or norm_c in norm_q:
                    len_ratio = min(len(norm_q), len(norm_c)) / max(len(norm_q), len(norm_c))
                    if len_ratio >= 0.75:
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


def process_toc_linking(input_dir: Path, output_dir: Path, workspace_dir: Path, config: dict = None):
    if not input_dir.exists():
        raise FileNotFoundError(f"Input directory does not exist: {input_dir}")

    start_marker = (config or {}).get("markers", {}).get("toc_start", TOC_START_MARKER) if config else TOC_START_MARKER
    end_marker = (config or {}).get("markers", {}).get("toc_end", TOC_END_MARKER) if config else TOC_END_MARKER

    output_dir.mkdir(parents=True, exist_ok=True)
    for old_md in output_dir.glob("*.md"):
        old_md.unlink()
    out_assets_dir = output_dir / "assets"
    out_assets_dir.mkdir(parents=True, exist_ok=True)

    # Synchronize assets
    src_assets = input_dir / "assets"
    if src_assets.exists():
        for f in src_assets.glob("*"):
            if f.is_file():
                shutil.copy2(f, out_assets_dir / f.name)

    if out_assets_dir.exists() and not any(out_assets_dir.iterdir()):
        try:
            out_assets_dir.rmdir()
        except Exception:
            pass

    print(f"[*] Cataloging headers across documents in {input_dir}...")
    catalog = catalog_headers_across_documents(input_dir)
    print(f"    Cataloged {len(catalog)} headers.")

    md_files = sorted(list(input_dir.glob("*.md")))
    linked_count = 0

    for md_path in md_files:
        with open(md_path, "r", encoding="utf-8") as f:
            content = f.read()

        toc_pattern = rf"{re.escape(start_marker)}[ \t]*\r?\n(.*?)\r?\n[ \t]*{re.escape(end_marker)}"
        file_matched_blocks = 0
        used_headers = set()
        current_chapter_stem = None

        def _replace_toc_block(m):
            nonlocal file_matched_blocks, current_chapter_stem
            file_matched_blocks += 1
            toc_lines = m.group(1).splitlines()
            linked_lines = []
            strip_pages = should_strip_page_numbers(toc_lines)

            for line in toc_lines:
                m_bullet = re.match(r"^(\s*)([-*]\s*)(.+)$", line)
                if not m_bullet:
                    linked_lines.append(line)
                    continue

                indent = m_bullet.group(1)
                bullet = m_bullet.group(2)
                raw_title = m_bullet.group(3).strip()
                title = strip_page_number(raw_title) if strip_pages else raw_title

                def norm_s(s: str) -> str:
                    return re.sub(r"[^a-z0-9]+", " ", s.lower()).strip()

                # Top-level item in TOC (no indentation): represents a Chapter / Section title
                if len(indent) == 0:
                    m_ch = re.search(r"(?:Chapter|Section)\s+(\d+)", title, re.IGNORECASE)
                    stem_cand = None
                    if m_ch:
                        ch_num = int(m_ch.group(1))
                        ch_pattern = re.compile(rf"^0*{ch_num}\s*[-_.]", re.IGNORECASE)
                        for e in catalog:
                            if ch_pattern.search(e["stem"]):
                                stem_cand = e["stem"]
                                break

                    if stem_cand:
                        current_chapter_stem = stem_cand
                        doc_headers = [
                            e for e in catalog
                            if e["stem"] == stem_cand and (stem_cand, e["header"]) not in used_headers
                        ]
                        matched_h = None

                        # Check multiple headers combined from beginning of candidate chapter (e.g. H1: "Chapter 1" + H2: "INTRODUCTION")
                        opening_headers = [e for e in doc_headers if e["level"] <= 2][:3]
                        if len(opening_headers) >= 2:
                            combined_text = " ".join(e["header"] for e in opening_headers)
                            if norm_alphanumeric(title) and norm_alphanumeric(title) == norm_alphanumeric(combined_text):
                                matched_h = opening_headers[0]
                            else:
                                sim = difflib.SequenceMatcher(None, norm_s(title), norm_s(combined_text)).ratio()
                                if sim >= 0.75:
                                    matched_h = opening_headers[0]

                        if not matched_h:
                            matched_h = find_best_header_match(
                                title,
                                catalog,
                                used_headers=used_headers,
                                preferred_stem=stem_cand,
                                cutoff=0.65
                            )

                        if not matched_h:
                            matched_h = next((e for e in doc_headers if e["level"] == 1), None)

                        if matched_h:
                            used_headers.add((stem_cand, matched_h["header"]))
                            wikilink = f"[[{stem_cand}#{matched_h['header']}|{title}]]"
                            linked_lines.append(f"{indent}{bullet}{wikilink}")
                        else:
                            linked_lines.append(f"{indent}{bullet}{title}")
                    else:
                        match_entry = find_best_header_match(
                            title,
                            catalog,
                            used_headers=used_headers,
                            preferred_stem=None,
                            cutoff=0.75
                        )
                        if match_entry:
                            current_chapter_stem = match_entry["stem"]
                            target_file = match_entry["stem"]
                            target_header = match_entry["header"]
                            used_headers.add((target_file, target_header))
                            wikilink = f"[[{target_file}#{target_header}|{title}]]"
                            linked_lines.append(f"{indent}{bullet}{wikilink}")
                        else:
                            current_chapter_stem = None
                            linked_lines.append(f"{indent}{bullet}{title}")
                else:
                    # Sub-bullet within a chapter: match strictly within current_chapter_stem
                    if current_chapter_stem is not None:
                        match_entry = find_best_header_match(
                            title,
                            catalog,
                            used_headers=used_headers,
                            preferred_stem=current_chapter_stem,
                            cutoff=0.75
                        )
                        if match_entry:
                            target_file = match_entry["stem"]
                            target_header = match_entry["header"]
                            used_headers.add((target_file, target_header))
                            wikilink = f"[[{target_file}#{target_header}|{title}]]"
                            linked_lines.append(f"{indent}{bullet}{wikilink}")
                        else:
                            linked_lines.append(f"{indent}{bullet}{title}")
                    else:
                        # Chapter does not exist in catalog: retain clean text bullet (stripped of trailing numbers)
                        linked_lines.append(f"{indent}{bullet}{title}")

            return "\n".join(linked_lines)

        updated_content = re.sub(toc_pattern, _replace_toc_block, content, flags=re.DOTALL)

        # Ensure any residual stray markers are removed
        if start_marker in updated_content:
            updated_content = updated_content.replace(start_marker, "")
        if end_marker in updated_content:
            updated_content = updated_content.replace(end_marker, "")

        if file_matched_blocks > 0:
            linked_count += file_matched_blocks
            print(f"[+] Converted {file_matched_blocks} TOC block(s) and removed markers in {md_path.name}")

        target_file = output_dir / md_path.name
        with open(target_file, "w", encoding="utf-8") as f:
            f.write(updated_content)

    print(f"[+] Stage 14 complete. Processed {linked_count} TOC sections into {output_dir}.")


def main():
    parser = argparse.ArgumentParser(description="Stage 14: Fuzzy cross-document Table of Contents linker")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory")
    parser.add_argument("--input-dir", type=str, default=None, help="Input directory (defaults to workspace/13_refine_first_chapter_name)")
    parser.add_argument("--output-dir", type=str, default=None, help="Output directory (defaults to workspace/14_link_toc)")
    parser.add_argument("--config", type=str, required=True, help="Path to config.yaml")

    args = parser.parse_args()
    workspace_dir = Path(args.workspace)

    config_path = Path(args.config)
    if not config_path.is_file():
        raise FileNotFoundError(f"Stage 14: Config file not found: {config_path}")

    with open(config_path, "r", encoding="utf-8") as f:
        config = yaml.safe_load(f)
    if not config or not isinstance(config, dict):
        raise ValueError(f"Stage 14: Config file is empty or invalid: {config_path}")

    input_candidates = [
        Path(args.input_dir) if args.input_dir else None,
        workspace_dir / "13_refine_first_chapter_name",
        workspace_dir / "12_generate_properties",
        workspace_dir / "11_emit_markdown",
        Path("output_markdown")
    ]
    input_dir = next((p for p in input_candidates if p and p.exists() and list(p.glob("*.md"))), None)
    if not input_dir:
        raise FileNotFoundError("No input markdown files found for Stage 14")

    output_dir = Path(args.output_dir) if args.output_dir else (workspace_dir / "14_link_toc")

    process_toc_linking(input_dir, output_dir, workspace_dir, config=config)


if __name__ == "__main__":
    main()

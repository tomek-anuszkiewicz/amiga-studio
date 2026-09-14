#!/usr/bin/env python3
"""Link & Anchor Validation Tool for Markdown Documents.

Audits Markdown files for broken local image paths, invalid anchor slugs,
and relative file links. Supports both standard Markdown links and Obsidian Wikilinks.
"""

from __future__ import annotations

import argparse
import re
import sys
import urllib.parse
from pathlib import Path
from typing import Dict, List, Set, Tuple


def make_anchor_slug(heading_text: str) -> str:
    """Generate a GitHub compatible anchor slug."""
    clean = re.sub(r"[^\w\s-]", "", heading_text.lower())
    return re.sub(r"[\s_]+", "-", clean).strip("-")


def extract_anchors_and_links(file_path: Path) -> Tuple[Set[str], Set[str], List[Tuple[int, str, str, bool]]]:
    """Extract valid heading titles, github slugs, and all links (markdown and wikilinks) from a file."""
    exact_headings: Set[str] = set()
    github_slugs: Set[str] = set()
    # links: (line_num, text, target, is_wikilink)
    links: List[Tuple[int, str, str, bool]] = []

    with open(file_path, "r", encoding="utf-8", errors="replace") as stream:
        lines = stream.readlines()

    in_frontmatter = False
    in_code_block = False

    for line_num, line in enumerate(lines, 1):
        stripped = line.strip()

        if line_num == 1 and stripped == "---":
            in_frontmatter = True
            continue
        if in_frontmatter:
            if stripped == "---":
                in_frontmatter = False
            continue

        if stripped.startswith("```"):
            in_code_block = not in_code_block
            continue
        if in_code_block:
            continue

        # Extract heading anchor
        heading_match = re.match(r"^#{1,6}\s+(.+)$", stripped)
        if heading_match:
            heading_title = heading_match.group(1).strip()
            # Strip link formatting inside heading if any
            heading_title = re.sub(r"\[(.*?)\]\(.*?\)", r"\1", heading_title)
            heading_title = re.sub(r"\[\[(.*?)(?:\|.*?)?\]\]", r"\1", heading_title)
            clean_title = heading_title.strip()
            exact_headings.add(clean_title.lower())
            slug = make_anchor_slug(clean_title)
            github_slugs.add(slug)

        # 1. Extract markdown links [text](target) and images ![alt](target)
        found_md_links = re.findall(r"!?\[(.*?)\]\((.*?)\)", line)
        for text, target in found_md_links:
            clean_target = target.split()[0].strip()
            links.append((line_num, text, clean_target, False))

        # 2. Extract Obsidian Wikilinks [[target]] or [[target|display]]
        found_wiki_links = re.findall(r"!?\[\[(.*?)\]\]", line)
        for wiki_content in found_wiki_links:
            if "|" in wiki_content:
                target_part, display_part = wiki_content.split("|", 1)
            else:
                target_part = wiki_content
                display_part = wiki_content
            clean_target = target_part.strip()
            clean_display = display_part.strip()
            links.append((line_num, clean_display, clean_target, True))

    return exact_headings, github_slugs, links


def validate_file(file_path: Path, warn_github_slugs: bool = True) -> Tuple[int, int, List[str]]:
    """Validate links and anchors within a single Markdown file."""
    exact_headings, github_slugs, links = extract_anchors_and_links(file_path)
    errors: List[str] = []
    checked_count = 0

    base_dir = file_path.parent

    for line_num, text, target, is_wikilink in links:
        checked_count += 1

        # Remote HTTP URLs (skip network checking)
        if target.startswith(("http://", "https://", "ftp://", "mailto:")):
            continue

        # Anchor links (internal to this document)
        if target.startswith("#"):
            raw_anchor = target[1:].strip()
            unquoted_anchor = urllib.parse.unquote(raw_anchor).strip()

            # Check if it matches an exact heading (Obsidian style)
            if unquoted_anchor.lower() in exact_headings or raw_anchor.lower() in exact_headings:
                continue

            # Check if it matches a GitHub kebab-case slug
            if raw_anchor.lower() in github_slugs or unquoted_anchor.lower() in github_slugs:
                if warn_github_slugs:
                    errors.append(
                        f"Line {line_num}: GitHub-style slug anchor '{target}' will fail in Obsidian! "
                        f"Obsidian requires exact heading text like '[[#{unquoted_anchor}]]' or '[{text}](#{urllib.parse.quote(unquoted_anchor)})'."
                    )
                continue

            errors.append(f"Line {line_num}: Broken anchor link '{target}' (heading not found in document)")
            continue

        # Wikilink heading in same document: [[#Heading Name]]
        if is_wikilink and target.startswith("#"):
            heading_target = target[1:].strip()
            if heading_target.lower() not in exact_headings:
                errors.append(f"Line {line_num}: Broken Wikilink anchor '{target}' (heading not found in document)")
            continue

        # File and asset links
        # Strip potential anchor from target: path.md#anchor
        file_part = target.split("#")[0]
        if not file_part:
            continue

        resolved_path = (base_dir / file_part).resolve()
        if not resolved_path.exists():
            errors.append(f"Line {line_num}: Broken relative path '{target}' -> '{resolved_path}' does not exist")

    return checked_count, len(errors), errors


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate links and anchors in Markdown files.")
    parser.add_argument("target", help="Path to Markdown file or directory to validate")
    parser.add_argument("--allow-github-slugs", action="store_true", help="Do not warn on GitHub kebab-case slugs")
    args = parser.parse_args()

    target_path = Path(args.target)
    if not target_path.exists():
        print(f"Error: Target path '{target_path}' does not exist.", file=sys.stderr)
        return 1

    md_files = [target_path] if target_path.is_file() else list(target_path.glob("**/*.md"))

    if not md_files:
        print(f"No Markdown files found under '{target_path}'.")
        return 0

    total_checked = 0
    total_errors = 0

    print(f"Validating {len(md_files)} Markdown file(s)...")

    for file_path in md_files:
        checked, err_count, errors = validate_file(file_path, warn_github_slugs=not args.allow_github_slugs)
        total_checked += checked
        total_errors += err_count

        if errors:
            print(f"\n[FAIL] {file_path}:")
            for err in errors:
                print(f"  - {err}")
        else:
            print(f"  [PASS] {file_path} ({checked} links checked)")

    print(f"\nValidation summary: {total_checked} links checked, {total_errors} errors found.")
    return 1 if total_errors > 0 else 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Link & Anchor Validation Tool for Markdown Documents.

Audits Markdown files for broken local image paths, invalid anchor slugs,
and relative file links.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Dict, List, Set, Tuple


def make_anchor_slug(heading_text: str) -> str:
    """Generate a GitHub and Obsidian compatible anchor slug."""
    clean = re.sub(r"[^\w\s-]", "", heading_text.lower())
    return re.sub(r"[\s_]+", "-", clean).strip("-")


def extract_anchors_and_links(file_path: Path) -> Tuple[Set[str], List[Tuple[int, str, str]]]:
    """Extract valid heading anchor slugs and all markdown links from a file."""
    anchors: Set[str] = set()
    links: List[Tuple[int, str, str]] = []

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
            slug = make_anchor_slug(heading_title)
            anchors.add(slug)

        # Extract markdown links [text](target) and images ![alt](target)
        found_links = re.findall(r"!?\[(.*?)\]\((.*?)\)", line)
        for text, target in found_links:
            clean_target = target.split()[0].strip()
            links.append((line_num, text, clean_target))

    return anchors, links


def validate_file(file_path: Path) -> Tuple[int, int, List[str]]:
    """Validate links and anchors within a single Markdown file."""
    anchors, links = extract_anchors_and_links(file_path)
    errors: List[str] = []
    checked_count = 0

    base_dir = file_path.parent

    for line_num, text, target in links:
        checked_count += 1

        # Anchor links
        if target.startswith("#"):
            anchor_slug = target[1:].lower()
            if anchor_slug not in anchors:
                errors.append(f"Line {line_num}: Broken anchor link '{target}' (heading not found in document)")
            continue

        # Remote HTTP URLs (skip network checking)
        if target.startswith(("http://", "https://", "ftp://", "mailto:")):
            continue

        # File and asset links
        # Strip potential anchor from target: path.md#anchor
        file_part = target.split("#")[0]
        resolved_path = (base_dir / file_part).resolve()

        if not resolved_path.exists():
            errors.append(f"Line {line_num}: Broken relative path '{target}' -> '{resolved_path}' does not exist")

    return checked_count, len(errors), errors


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate links and anchors in Markdown files.")
    parser.add_argument("target", help="Path to Markdown file or directory to validate")
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
        checked, err_count, errors = validate_file(file_path)
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

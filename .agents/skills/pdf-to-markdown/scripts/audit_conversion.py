#!/usr/bin/env python3
"""Automated Semantic & Visual Sanity Auditor for PDF/HTML-to-Markdown.

Detects glaring conversion flaws such as prose paragraphs mistakenly turned into
code blocks, fragmented consecutive code fences, missing headings, and mangled
tables before human review.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Tuple

PROSE_STOPWORDS = {
    "the", "and", "that", "have", "for", "not", "with", "you", "this",
    "but", "his", "from", "they", "say", "her", "she", "will", "one",
    "all", "would", "there", "their", "what", "out", "about", "who",
    "get", "which", "when", "make", "can", "like", "time", "just",
    "him", "know", "take", "people", "into", "year", "your", "good",
    "some", "could", "them", "see", "other", "than", "then", "now",
    "look", "only", "come", "its", "over", "think", "also", "back",
    "after", "use", "two", "how", "our", "work", "first", "well",
    "way", "even", "new", "want", "because", "any", "these", "give",
    "day", "most", "us", "processor", "processors", "execution", "instruction",
}


@dataclass
class AuditReport:
    file_path: Path
    total_lines: int = 0
    total_code_lines: int = 0
    code_blocks_count: int = 0
    prose_in_code_issues: List[Tuple[int, str]] = field(default_factory=list)
    fragmented_code_issues: List[Tuple[int, str]] = field(default_factory=list)
    table_row_issues: List[Tuple[int, str]] = field(default_factory=list)
    raw_details_issues: List[Tuple[int, str]] = field(default_factory=list)
    warnings: List[str] = field(default_factory=list)
    is_clean: bool = True


def audit_markdown_file(file_path: Path) -> AuditReport:
    """Audit a Markdown file for common automated conversion blunders."""
    report = AuditReport(file_path=file_path)

    with open(file_path, "r", encoding="utf-8", errors="replace") as f:
        content = f.read()

    # Check for raw carriage returns in content (excluding CRLF line endings)
    # i.e., \r not followed immediately by \n, or \r inside a string literal like \rightarrow
    lines = content.splitlines(keepends=False)
    report.total_lines = len(lines)

    in_code_block = False
    current_code_lines: List[str] = []
    code_block_start_line = 0
    last_code_block_end = -999
    is_mermaid = False

    for line_num, line in enumerate(lines, 1):
        stripped = line.strip().lstrip(">").strip()

        # Check for unescaped carriage returns inside the line
        if "\r" in line:
            report.table_row_issues.append(
                (line_num, "Line contains raw carriage return character (\\r) causing unintended line split")
            )

        # Check for raw HTML details/summary tags
        if ("<details" in line.lower() or "<summary" in line.lower()) and not in_code_block:
            report.raw_details_issues.append(
                (line_num, "Raw HTML <details>/<summary> tag found; use native Obsidian callout '> [!NOTE]-' instead")
            )

        # Check for malformed table rows (orphaned table cell split across lines)
        if "|" in line and not in_code_block:
            # If line has pipe but doesn't start or end with pipe (and isn't inside a quote block)
            if stripped.endswith("|") and not stripped.startswith("|") and not stripped.startswith("+-") and not stripped.startswith("+="):
                report.table_row_issues.append(
                    (line_num, f"Orphaned or split table row detected: '{stripped[:60]}'")
                )

        if stripped.startswith("```"):
            if not in_code_block:
                in_code_block = True
                code_block_start_line = line_num
                current_code_lines = []
                is_mermaid = stripped.startswith("```mermaid")
                if not is_mermaid:
                    report.code_blocks_count += 1

                # Check for fragmented adjacent code blocks (separated by <= 2 lines)
                if 0 < (line_num - last_code_block_end) <= 2:
                    report.fragmented_code_issues.append(
                        (line_num, f"Fragmented adjacent code blocks at line {last_code_block_end} and {line_num}")
                    )
            else:
                in_code_block = False
                last_code_block_end = line_num

                # Analyze completed code block for prose leakage
                full_block_text = " ".join(current_code_lines)
                words = re.findall(r"\b[a-z]{3,}\b", full_block_text.lower())
                prose_hits = sum(1 for w in words if w in PROSE_STOPWORDS)

                # If code block contains multiple sentences with English stop words
                if len(words) > 15 and (prose_hits / max(1, len(words))) > 0.35:
                    sample = full_block_text[:90] + ("..." if len(full_block_text) > 90 else "")
                    report.prose_in_code_issues.append(
                        (code_block_start_line, f"Prose sentence mistakenly inside code fence: '{sample}'")
                    )
            continue

        if in_code_block:
            if not is_mermaid:
                report.total_code_lines += 1
            current_code_lines.append(stripped)

    # Check overall code ratio
    if report.total_lines > 50:
        ratio = report.total_code_lines / report.total_lines
        if ratio > 0.40:
            report.warnings.append(
                f"Abnormally high code block density ({ratio:.1%} of all lines are in code blocks)"
            )

    if report.prose_in_code_issues or report.fragmented_code_issues or report.table_row_issues or report.raw_details_issues:
        report.is_clean = False

    return report


def main() -> int:
    parser = argparse.ArgumentParser(description="Audit Markdown files for visual and semantic conversion flaws.")
    parser.add_argument("target", help="Path to Markdown file or directory to audit")
    args = parser.parse_args()

    target_path = Path(args.target)
    if not target_path.exists():
        print(f"Error: Target path '{target_path}' does not exist.", file=sys.stderr)
        return 1

    md_files = [target_path] if target_path.is_file() else list(target_path.glob("**/*.md"))
    if not md_files:
        print(f"No Markdown files found under '{target_path}'.")
        return 0

    overall_clean = True
    print(f"Auditing {len(md_files)} Markdown file(s) for visual & semantic fidelity...\n")

    for file_path in md_files:
        report = audit_markdown_file(file_path)
        if report.is_clean and not report.warnings:
            print(f"  [PASS] {file_path.name} (Lines: {report.total_lines}, Code blocks: {report.code_blocks_count})")
        else:
            if not report.is_clean:
                overall_clean = False
            status = "[FAIL]" if not report.is_clean else "[WARN]"
            print(f"{status} {file_path}:")
            for line_no, issue in report.prose_in_code_issues:
                print(f"  - Line {line_no} [PROSE LEAK]: {issue}")
            for line_no, issue in report.fragmented_code_issues:
                print(f"  - Line {line_no} [FRAGMENT]: {issue}")
            for line_no, issue in report.table_row_issues:
                print(f"  - Line {line_no} [TABLE ROW]: {issue}")
            for line_no, issue in report.raw_details_issues:
                print(f"  - Line {line_no} [RAW DETAILS]: {issue}")
            for warn in report.warnings:
                print(f"  - [WARNING]: {warn}")
            print()

    if overall_clean:
        print("\nAll files passed semantic and visual sanity audit.")
        return 0
    else:
        print("\nSanity audit FAILED: Visual/semantic anomalies detected.")
        return 1


if __name__ == "__main__":
    sys.exit(main())

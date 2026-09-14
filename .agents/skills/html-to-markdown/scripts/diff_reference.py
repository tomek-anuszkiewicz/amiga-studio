#!/usr/bin/env python3
"""Structural Differencing & Quality Benchmark Tool.

Compares a generated Markdown document against a ground truth reference
document and reports structural similarity, heading alignment, code blocks,
tables, and metadata fidelity.
"""

from __future__ import annotations

import argparse
import difflib
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Set


@dataclass
class DocumentMetrics:
    path: Path
    total_lines: int = 0
    total_words: int = 0
    total_chars: int = 0
    headings_h1: List[str] = field(default_factory=list)
    headings_h2: List[str] = field(default_factory=list)
    headings_h3: List[str] = field(default_factory=list)
    headings_h4: List[str] = field(default_factory=list)
    code_blocks: List[str] = field(default_factory=list)
    tables_count: int = 0
    mermaid_count: int = 0
    has_frontmatter: bool = False
    has_toc: bool = False


def extract_metrics(file_path: Path) -> DocumentMetrics:
    """Extract structural metrics from a Markdown file."""
    metrics = DocumentMetrics(path=file_path)
    with open(file_path, "r", encoding="utf-8", errors="replace") as stream:
        lines = stream.readlines()

    metrics.total_lines = len(lines)
    full_text = "".join(lines)
    metrics.total_chars = len(full_text)
    metrics.total_words = len(re.findall(r"\b\w+\b", full_text))

    in_frontmatter = False
    in_code_block = False
    current_code_lang = ""

    for i, line in enumerate(lines):
        stripped = line.strip()

        # Frontmatter detection
        if i == 0 and stripped == "---":
            in_frontmatter = True
            metrics.has_frontmatter = True
            continue
        if in_frontmatter:
            if stripped == "---":
                in_frontmatter = False
            continue

        # Code block tracking
        if stripped.startswith("```"):
            if not in_code_block:
                in_code_block = True
                current_code_lang = stripped[3:].strip()
                metrics.code_blocks.append(current_code_lang)
                if current_code_lang == "mermaid":
                    metrics.mermaid_count += 1
            else:
                in_code_block = False
            continue

        if in_code_block:
            continue

        # TOC detection
        if re.match(r"^#+\s+Table of Contents", stripped, re.IGNORECASE):
            metrics.has_toc = True

        # Headings
        if stripped.startswith("# "):
            metrics.headings_h1.append(stripped[2:].strip())
        elif stripped.startswith("## "):
            metrics.headings_h2.append(stripped[3:].strip())
        elif stripped.startswith("### "):
            metrics.headings_h3.append(stripped[4:].strip())
        elif stripped.startswith("#### "):
            metrics.headings_h4.append(stripped[5:].strip())

        # Table row indicator
        if stripped.startswith("|") and stripped.endswith("|") and ":---" in stripped:
            metrics.tables_count += 1

    return metrics


def compute_vocabulary_similarity(text_a: str, text_b: str) -> float:
    """Compute Jaccard similarity of vocabulary words."""
    words_a = set(re.findall(r"\b[a-zA-Z0-9_]{3,}\b", text_a.lower()))
    words_b = set(re.findall(r"\b[a-zA-Z0-9_]{3,}\b", text_b.lower()))
    if not words_a or not words_b:
        return 0.0
    intersection = len(words_a & words_b)
    union = len(words_a | words_b)
    return (intersection / union) * 100.0


def compare_documents(candidate: DocumentMetrics, reference: DocumentMetrics) -> str:
    """Generate a Markdown comparison report."""
    with open(candidate.path, "r", encoding="utf-8", errors="replace") as f:
        cand_text = f.read()
    with open(reference.path, "r", encoding="utf-8", errors="replace") as f:
        ref_text = f.read()

    vocab_sim = compute_vocabulary_similarity(cand_text, ref_text)

    all_cand_headings = candidate.headings_h1 + candidate.headings_h2 + candidate.headings_h3 + candidate.headings_h4
    all_ref_headings = reference.headings_h1 + reference.headings_h2 + reference.headings_h3 + reference.headings_h4

    ref_heading_set = {h.lower() for h in all_ref_headings}
    matched_headings = [h for h in all_cand_headings if any(r in h.lower() or h.lower() in r for r in ref_heading_set)]
    heading_coverage = (len(matched_headings) / max(1, len(all_ref_headings))) * 100.0

    lines = [
        "# Structural Benchmark & Difference Report",
        "",
        f"- **Candidate Document:** `{candidate.path}`",
        f"- **Reference Document:** `{reference.path}`",
        "",
        "## Quantitative Summary",
        "",
        "| Metric | Candidate | Reference Ground Truth | Alignment / Status |",
        "| :--- | :---: | :---: | :--- |",
        f"| Total Lines | {candidate.total_lines} | {reference.total_lines} | {candidate.total_lines / max(1, reference.total_lines):.1%} ratio |",
        f"| Word Count | {candidate.total_words} | {reference.total_words} | {candidate.total_words / max(1, reference.total_words):.1%} ratio |",
        f"| Vocabulary Overlap | - | - | {vocab_sim:.1f}% Jaccard similarity |",
        f"| Frontmatter Present | {'Yes' if candidate.has_frontmatter else 'No'} | {'Yes' if reference.has_frontmatter else 'No'} | {'MATCH' if candidate.has_frontmatter == reference.has_frontmatter else 'DIFF'} |",
        f"| Table of Contents | {'Yes' if candidate.has_toc else 'No'} | {'Yes' if reference.has_toc else 'No'} | {'MATCH' if candidate.has_toc == reference.has_toc else 'DIFF'} |",
        f"| H1 Headings | {len(candidate.headings_h1)} | {len(reference.headings_h1)} | Count match: {len(candidate.headings_h1) == len(reference.headings_h1)} |",
        f"| H2 Headings | {len(candidate.headings_h2)} | {len(reference.headings_h2)} | {len(candidate.headings_h2)} vs {len(reference.headings_h2)} |",
        f"| H3 Headings | {len(candidate.headings_h3)} | {len(reference.headings_h3)} | {len(candidate.headings_h3)} vs {len(reference.headings_h3)} |",
        f"| Heading Alignment | - | - | {heading_coverage:.1f}% covered |",
        f"| Code Blocks | {len(candidate.code_blocks)} | {len(reference.code_blocks)} | {len(candidate.code_blocks)} vs {len(reference.code_blocks)} |",
        f"| Tables Count | {candidate.tables_count} | {reference.tables_count} | {candidate.tables_count} vs {reference.tables_count} |",
        f"| Mermaid Diagrams | {candidate.mermaid_count} | {reference.mermaid_count} | {candidate.mermaid_count} vs {reference.mermaid_count} |",
        "",
        "## Heading Alignment Details",
        "",
        "### Candidate Headings:",
    ]
    for h in all_cand_headings:
        lines.append(f"- `{h}`")

    lines.append("")
    lines.append("### Reference Headings:")
    for h in all_ref_headings:
        matched = any(r in h.lower() or h.lower() in r for r in {c.lower() for c in all_cand_headings})
        status = "[MATCH]" if matched else "[MISSING]"
        lines.append(f"- {status} `{h}`")

    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Compare candidate Markdown against ground truth reference.")
    parser.add_argument("candidate", help="Path to generated/candidate Markdown file")
    parser.add_argument("reference", help="Path to ground truth reference Markdown file")
    parser.add_argument("--report", "-r", help="Optional output report Markdown file path")

    args = parser.parse_args()
    cand_path = Path(args.candidate)
    ref_path = Path(args.reference)

    if not cand_path.exists():
        print(f"Error: Candidate file '{cand_path}' does not exist.", file=sys.stderr)
        return 1
    if not ref_path.exists():
        print(f"Error: Reference file '{ref_path}' does not exist.", file=sys.stderr)
        return 1

    cand_metrics = extract_metrics(cand_path)
    ref_metrics = extract_metrics(ref_path)

    report = compare_documents(cand_metrics, ref_metrics)

    if args.report:
        report_path = Path(args.report)
        report_path.parent.mkdir(parents=True, exist_ok=True)
        with open(report_path, "w", encoding="utf-8") as stream:
            stream.write(report)
        print(f"Comparison report saved to '{report_path}'")
    else:
        print(report)

    return 0


if __name__ == "__main__":
    sys.exit(main())

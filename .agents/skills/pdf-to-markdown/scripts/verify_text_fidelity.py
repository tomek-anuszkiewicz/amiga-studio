#!/usr/bin/env python3
"""
verify_text_fidelity.py - Statistical Word & Lexical Fidelity Auditor

Compares converted Markdown pages against the source PDF text layer using statistical
word frequency, total word count ratios, and vocabulary recall metrics. Automatically
detects LLM summarization, dropped paragraphs, and hallucinated additions.
"""

import sys
import re
import argparse
from pathlib import Path
from collections import Counter
from typing import Dict, Set, Tuple, List, Optional

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8', errors='backslashreplace')
if hasattr(sys.stderr, 'reconfigure'):
    sys.stderr.reconfigure(encoding='utf-8', errors='backslashreplace')

try:
    import fitz  # PyMuPDF
except ImportError:
    print("Error: PyMuPDF (fitz) is required. Install via 'pip install pymupdf'.", file=sys.stderr)
    sys.exit(1)


def extract_normalized_words(text: str) -> List[str]:
    """Cleans line wraps and extracts normalized alphanumeric word tokens."""
    # Fix hyphenation across lines: e.g. "graph-\nics" -> "graphics"
    t = re.sub(r'(\w+)[-\xad]\n+(\w+)', r'\1\2', text)
    # Strip markdown markup characters
    t = re.sub(r'[#*`_~\[\]()|><\-=+]', ' ', t)
    # Find word tokens of length >= 3
    words = re.findall(r'\b[a-zA-Z0-9_\$]{3,}\b', t.lower())
    return words


def audit_page_fidelity(
    pdf_text: str,
    md_text: str,
    min_ratio: float = 0.75,
    max_ratio: float = 1.35,
    min_recall: float = 0.85
) -> Tuple[bool, Dict[str, any]]:
    """
    Statistically audits text fidelity between source PDF text and generated Markdown.
    """
    pdf_words = extract_normalized_words(pdf_text)
    md_words = extract_normalized_words(md_text)

    pdf_count = len(pdf_words)
    md_count = len(md_words)

    # Empty / blank page check
    if pdf_count == 0:
        is_clean = md_count < 20
        return is_clean, {
            "pdf_words": 0,
            "md_words": md_count,
            "ratio": 1.0 if md_count == 0 else 99.0,
            "recall": 1.0,
            "missing_words": [],
            "status": "BLANK" if is_clean else "FAIL_NON_EMPTY_BLANK",
            "message": "Blank page verified" if is_clean else "Unexpected text on blank page"
        }

    ratio = md_count / max(1, pdf_count)
    
    pdf_vocab = set(pdf_words)
    md_vocab = set(md_words)
    
    # Common scanner OCR noise words that might be normalized/cleaned
    OCR_NOISE_IGNORABLES = {"die", "in", "the", "iii", "vii", "viii", "xvi", "xii"}
    
    missing_words = [w for w in (pdf_vocab - md_vocab) if w not in OCR_NOISE_IGNORABLES]
    recall = (len(pdf_vocab) - len(missing_words)) / max(1, len(pdf_vocab))

    # Determine pass/fail
    passed = True
    messages = []

    if ratio < min_ratio:
        passed = False
        messages.append(f"Summarization/Drop detected: Word ratio {ratio:.2f} < {min_ratio:.2f}")
    elif ratio > max_ratio:
        passed = False
        messages.append(f"Hallucination/Expansion detected: Word ratio {ratio:.2f} > {max_ratio:.2f}")

    if recall < min_recall and pdf_count > 40:
        passed = False
        messages.append(f"Low vocabulary recall {recall:.1%} < {min_recall:.1%}")

    status = "PASS" if passed else "FAIL"
    message = "; ".join(messages) if messages else "Fidelity verified"

    return passed, {
        "pdf_words": pdf_count,
        "md_words": md_count,
        "ratio": ratio,
        "recall": recall,
        "missing_words": missing_words[:12],
        "status": status,
        "message": message
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Statistically audit converted Markdown pages against source PDF text layer."
    )
    parser.add_argument("--pdf", required=True, help="Path to source PDF manual")
    parser.add_argument("--markdown-dir", required=True, help="Directory containing page_XXX.md files")
    parser.add_argument("--min-ratio", type=float, default=0.75, help="Minimum allowed word count ratio (default: 0.75)")
    parser.add_argument("--max-ratio", type=float, default=1.35, help="Maximum allowed word count ratio (default: 1.35)")
    parser.add_argument("--min-recall", type=float, default=0.85, help="Minimum allowed vocabulary recall (default: 0.85)")
    args = parser.parse_args()

    pdf_path = Path(args.pdf)
    md_dir = Path(args.markdown_dir)

    if not pdf_path.is_file():
        print(f"Error: PDF '{pdf_path}' does not exist.", file=sys.stderr)
        return 1
    if not md_dir.is_dir():
        print(f"Error: Markdown directory '{md_dir}' does not exist.", file=sys.stderr)
        return 1

    doc = fitz.open(pdf_path)
    md_files = sorted(md_dir.glob("page_*.md"))

    if not md_files:
        print(f"No page_XXX.md files found in '{md_dir}'.", file=sys.stderr)
        return 1

    print(f"Running Statistical Word & Lexical Fidelity Audit on {len(md_files)} page(s)...\n")
    print(f"{'Page':<12} | {'PDF Words':<10} | {'MD Words':<10} | {'Ratio':<7} | {'Recall':<8} | {'Status'}")
    print("-" * 75)

    failures = 0

    for md_file in md_files:
        # Extract page number from filename: page_005.md -> 5
        m = re.search(r'page_(\d+)\.md', md_file.name)
        if not m:
            continue
        page_num = int(m.group(1))
        if page_num < 1 or page_num > len(doc):
            continue

        pdf_page_text = doc[page_num - 1].get_text()
        md_text = md_file.read_text(encoding="utf-8")

        passed, stats = audit_page_fidelity(
            pdf_text=pdf_page_text,
            md_text=md_text,
            min_ratio=args.min_ratio,
            max_ratio=args.max_ratio,
            min_recall=args.min_recall
        )

        status_str = f"[{stats['status']}]"
        ratio_str = f"{stats['ratio']:.2f}"
        recall_str = f"{stats['recall']:.1%}"

        print(
            f"{md_file.name:<12} | {stats['pdf_words']:<10} | {stats['md_words']:<10} | "
            f"{ratio_str:<7} | {recall_str:<8} | {status_str} {stats['message']}"
        )

        if not passed:
            failures += 1
            if stats["missing_words"]:
                print(f"    -> Missing sample: {stats['missing_words']}")

    print("-" * 75)
    if failures == 0:
        print("\nAll pages PASSED statistical word and lexical fidelity audit!")
        return 0
    else:
        print(f"\nAudit FAILED: {failures} page(s) exhibited statistical divergence (summarization or omission).")
        return 1


if __name__ == "__main__":
    sys.exit(main())

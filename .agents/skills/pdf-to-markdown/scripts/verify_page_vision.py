#!/usr/bin/env python3
"""
verify_page_vision.py - Visual Double-Check QA Verification Tool

Prepares structured comparison prompts and automated checklists to audit
transcribed Markdown and cropped visual assets against the original PDF page PNGs,
detecting dropped paragraphs, cut-off diagram edges, text duplication, and table issues.
"""

import sys
import re
import json
import argparse
from pathlib import Path
from typing import Dict, List, Any

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8', errors='backslashreplace')
if hasattr(sys.stderr, 'reconfigure'):
    sys.stderr.reconfigure(encoding='utf-8', errors='backslashreplace')

def generate_qa_prompt(page_num: int, md_text: str, image_assets: List[str]) -> str:
    """
    Generates a rigorous LLM Vision QA comparison prompt.
    """
    asset_list = ", ".join(image_assets) if image_assets else "None"
    return f"""You are a rigorous technical quality assurance auditor for document digitization.
Compare the attached original PDF page image (Page {page_num:03d}) against the transcribed Markdown text and image assets below.

Transcribed Markdown:
```markdown
{md_text}
```

Extracted Image Assets: {asset_list}

Verify each item on this checklist:
1. DROPPED / MISSING TEXT: Are there any sentences, fine-print notes, footnote asterisks (*), or table cells visible in the original page that are missing in the Markdown?
2. CUT-OFF GRAPHICS: For any diagrams or figures, are any outer labels, pin names (e.g. IPL0, DTACK), clock signal labels, or borders truncated or clipped?
3. DUPLICATE NOTES: Does the diagram contain embedded notes or legends that were ALSO transcribed in the Markdown below? (Notes should be in Markdown only).
4. TABLE INTEGRITY: If there is a table, does every column and row match the original page without shifted columns or missing headers?
5. MOTOROLA HEX ADDRESSES: Are all hexadecimal addresses (e.g. $000004, $DFF000) enclosed in backticks (`$DFF000`) rather than unescaped math delimiters?

Output a JSON audit report:
{{
  "page": {page_num},
  "pass": true | false,
  "missing_text": ["..."],
  "cut_off_graphics": ["..."],
  "duplicate_notes": ["..."],
  "table_issues": ["..."],
  "hex_escaping_issues": ["..."],
  "recommendations": ["..."]
}}
"""

def audit_markdown_file(md_path: Path, page_num: int) -> Dict[str, Any]:
    """Performs static checks on the Markdown file prior to vision check."""
    content = md_path.read_text(encoding="utf-8")
    issues = []
    
    # 1. Check for unescaped bare hex ($00000004 outside backticks)
    bare_hex = re.findall(r'(?<![`\$\w])\$[0-9a-fA-F]{2,8}\b(?![`\$\w])', content)
    if bare_hex:
        issues.append(f"Found {len(bare_hex)} bare hex values not in backticks: {bare_hex[:5]}")

    # 2. Check for unresolved <crop> tags
    crops = re.findall(r'<crop\s+[^>]+/>', content, re.IGNORECASE)
    if crops:
        issues.append(f"Found {len(crops)} unextracted <crop> tags.")

    # 3. Check for broken table formatting
    for line_idx, line in enumerate(content.splitlines(), start=1):
        if line.strip().startswith('|') and not line.strip().endswith('|'):
            issues.append(f"Line {line_idx}: Unterminated table row.")

    # Find embedded images
    images = re.findall(r'!\[([^\]]*)\]\(([^)]+)\)', content)

    return {
        'page': page_num,
        'issues': issues,
        'images': [img[1] for img in images]
    }

def main():
    parser = argparse.ArgumentParser(description="Audit page markdown and generate Vision QA prompts.")
    parser.add_argument("--page-png", type=str, required=True, help="Path to original page PNG")
    parser.add_argument("--markdown", type=str, required=True, help="Path to transcribed page Markdown")
    parser.add_argument("--prompt-only", action="store_true", help="Print vision QA prompt and exit")
    args = parser.parse_args()

    png_path = Path(args.page_png)
    md_path = Path(args.markdown)

    if not png_path.exists() or not md_path.exists():
        print("Error: Input files do not exist.")
        sys.exit(1)

    m_page = re.search(r'(\d+)', png_path.stem)
    page_num = int(m_page.group(1)) if m_page else 1

    md_text = md_path.read_text(encoding="utf-8")
    audit_res = audit_markdown_file(md_path, page_num)

    print("=" * 60)
    print(f"STATIC QA AUDIT REPORT FOR PAGE {page_num:03d}")
    print("=" * 60)
    if audit_res['issues']:
        print("Issues Detected:")
        for iss in audit_res['issues']:
            print(f"  ❌ {iss}")
    else:
        print("  ✓ No static markdown formatting or hex escaping errors detected.")

    print(f"\nEmbedded Assets: {audit_res['images']}")

    if args.prompt_only:
        print("\n--- VISION AUDIT PROMPT ---")
        prompt = generate_qa_prompt(page_num, md_text, audit_res['images'])
        print(prompt)

if __name__ == "__main__":
    main()

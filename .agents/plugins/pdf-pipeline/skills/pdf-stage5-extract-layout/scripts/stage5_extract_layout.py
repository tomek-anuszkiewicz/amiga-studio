"""Stage 5: Extract Page Layout, Text Blocks, and Visual Crop Candidates.

Non-destructive preprocessing step for technical PDF manuals.
Extracts per-page:
  - Clean text blocks (headers/footers stripped by geometry)
  - Visual element bounding boxes (drawings, vector graphics, images, vertical gaps)
  - Interleaved text stream with CROP_CANDIDATE anchors
  - High-resolution page preview image (PNG)
Outputs to: build/01_page_layout/page_XXXX.json and page_XXXX.png

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-stage5-extract-layout/scripts/stage5_extract_layout.py "path/to/manual_ocr.pdf"
    python .agents/plugins/pdf-pipeline/skills/pdf-stage5-extract-layout/scripts/stage5_extract_layout.py "path/to/manual_ocr.pdf" --pages 4-15
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

try:
    import pymupdf as fitz
except ImportError:
    try:
        import fitz  # type: ignore
    except ImportError:
        sys.stderr.write("Error: PyMuPDF (fitz) is required. Install via 'pip install pymupdf'.\n")
        sys.exit(1)


def parse_page_range(range_str: str, max_pages: int) -> List[int]:
    """Parse comma/dash separated page ranges like '1-5,8,11-13' (1-based)."""
    pages: List[int] = []
    for part in range_str.split(","):
        part = part.strip()
        if not part:
            continue
        if "-" in part:
            start_s, end_s = part.split("-", 1)
            start = max(1, int(start_s))
            end = min(max_pages, int(end_s))
            pages.extend(range(start, end + 1))
        else:
            p = int(part)
            if 1 <= p <= max_pages:
                pages.append(p)
    return sorted(list(set(pages)))


def is_noise_header_footer(
    bbox: Tuple[float, float, float, float],
    text: str,
    page_height: float,
    header_margin: float,
    footer_margin: float,
) -> bool:
    """Detect if a text block is running header or footer based on Y geometry and content."""
    y0, y1 = bbox[1], bbox[3]
    t = text.strip()

    if y1 <= header_margin:
        return True
    if y0 >= (page_height - footer_margin):
        return True

    # Single-number or roman numeral page footer or header
    if re.match(r"^(?:\d{1,4}|[ivxlcdm]{1,6})$", t, re.IGNORECASE) and (y0 < header_margin + 25 or y1 > page_height - footer_margin - 25):
        return True

    # Standard section page numbers like 'Section 1 - 5' or '1-5' at page edges
    if re.match(r"^(?:Section\s+\d+|Chapter\s+\d+|Appendix\s+[A-Z]|\d+)\s*[-–—]\s*\d+$", t, re.IGNORECASE):
        if y0 < header_margin + 20 or y1 > page_height - footer_margin - 20:
            return True

    return False


COMMON_PROSE_WORDS = {
    "the", "and", "are", "use", "of", "to", "in", "is", "for", "with", "that", "this",
    "from", "part", "even", "such", "claim", "design", "been", "each", "were", "when",
    "will", "would", "shall", "about", "other", "into", "their", "there", "then", "them",
    "these", "those", "have", "has", "had", "not", "any", "all", "more", "most", "also",
    "buyer", "costs", "fees", "which", "could", "create", "death", "should", "injury",
    "patent", "rights", "body", "life", "only", "first", "second", "third", "after",
    "regarding", "regard", "damages", "damage", "notice", "without", "further", "products"
}


def format_line_math(
    spans: List[Dict[str, Any]],
    line_bbox: Tuple[float, float, float, float],
) -> Tuple[str, str, bool]:
    """Convert spans in a line to plain text and LaTeX with math detection."""
    if not spans:
        return "", "", False

    sizes = [s["size"] for s in spans if s.get("text", "").strip()]
    base_size = max(sizes) if sizes else 10.0

    tokens = []
    has_math = False

    for s in spans:
        t = s.get("text", "")
        size = s.get("size", 10.0)
        flags = s.get("flags", 0)
        font = s.get("font", "")
        origin = s.get("origin", (0, 0))

        # Typography normalization
        t_clean = (
            t.replace("\u2019", "'")
             .replace("\u2018", "'")
             .replace("\u2013", "-")
             .replace("\u2014", "-")
             .replace("\u2026", "...")
        )

        clean_t = t_clean.strip()
        is_candidate_exponent = (
            len(clean_t) <= 7 and (
                (any(c.isdigit() for c in clean_t) and bool(re.match(r"^[a-zA-Z]?[-–+]?\d{1,5}$", clean_t) or re.match(r"^[a-zA-Z][-–+]\d{1,5}$", clean_t)))
                or (len(clean_t) == 1 and clean_t.lower() in "snxykmij")
            )
        )

        # Baseline & font detection:
        # flags & 1 == 1 is fitz.TEXT_FONT_SUPERSCRIPT
        is_sup = is_candidate_exponent and (
            (flags & 1 != 0) or (size < base_size * 0.88 and origin[1] < line_bbox[3] - 2.0)
        )
        is_sub = is_candidate_exponent and (
            size < base_size * 0.88 and origin[1] > line_bbox[3] - 0.5
        )

        has_symbol = ("Symbol" in font) or any(c in t for c in ["\u00d7", "±", "÷", "≤", "≥", "≠", "≈", "∞"])

        if is_sup or is_sub or has_symbol:
            has_math = True

        tokens.append({
            "raw": t_clean,
            "is_sup": is_sup,
            "is_sub": is_sub,
            "has_symbol": has_symbol,
            "font": font,
        })

    plain_line = "".join(tok["raw"] for tok in tokens)

    if not has_math:
        return plain_line.strip(), plain_line.strip(), False

    # Build LaTeX string
    latex_parts = []
    for tok in tokens:
        t = tok["raw"]
        clean_t = t.strip()
        leading_space = " " if t.startswith(" ") else ""
        trailing_space = " " if t.endswith(" ") else ""

        if tok["is_sup"] and clean_t:
            latex_parts.append(f"{leading_space}^{{{clean_t}}}{trailing_space}")
        elif tok["is_sub"] and clean_t:
            latex_parts.append(f"{leading_space}_{{{clean_t}}}{trailing_space}")
        elif "\u00d7" in t or ("Symbol" in tok["font"] and "\u00d7" in t):
            latex_parts.append(t.replace("\u00d7", r"\times "))
        elif "±" in t or ("Symbol" in tok["font"] and "±" in t):
            latex_parts.append(t.replace("±", r"\pm "))
        elif "≤" in t:
            latex_parts.append(t.replace("≤", r"\le "))
        elif "≥" in t:
            latex_parts.append(t.replace("≥", r"\ge "))
        elif "≠" in t:
            latex_parts.append(t.replace("≠", r"\ne "))
        else:
            latex_parts.append(t)

    raw_latex = "".join(latex_parts)

    # Normalization:
    # 1. Minus attached to base before exponent: e.g. 10-^{38} -> 10^{-38}, 2-^{126} -> 2^{-126}
    raw_latex = re.sub(r'(\b\w+)[-–]\^\{([^}]+)\}', r'\1^{-\2}', raw_latex)
    # 2. Convert ' x 10' when used as scientific notation to '\times 10'
    raw_latex = re.sub(r'(\d+)\s+x\s+10\^\{', r'\1 \\times 10^{', raw_latex)
    raw_latex = re.sub(r'(\d+)\s+x\s+10[-–]\^\{([^}]+)\}', r'\1 \\times 10^{-\2}', raw_latex)
    # 3. Clean spaces around \times and \pm
    raw_latex = re.sub(r'\s*\\times\s*', r' \\times ', raw_latex)
    raw_latex = re.sub(r'\s*\\pm\s*', r'\\pm ', raw_latex)

    # Wrap math expressions in $...$
    words = [w for w in re.split(r'\s+', plain_line) if len(w) > 3 and w.isalpha()]
    if len(words) <= 1 and any(sym in raw_latex for sym in ["^", "_", r"\times", r"\pm", "=", "+"]):
        # Pure math line
        latex_line = f"${raw_latex.strip()}$"
    else:
        # Prose with inline math: isolate math expressions
        def wrap_expr(m):
            expr = m.group(0).strip()
            return f"${expr}$"

        pattern = r'(?:\\pm\s*)?(?:\(?[-–]?\d*\.?\d+\)?|[A-Za-z0-9\.\(\)\-]+)(?:\^|\_)\{[^}]+\}(?:\s*\\times\s*(?:\\pm\s*)?(?:\(?[-–]?\d*\.?\d+\)?|[A-Za-z0-9\.\(\)\-]+)(?:\^|\_)\{[^}]+\})*(?:\s*\\times\s*[A-Za-z0-9\.\(\)\-]+)?'
        latex_line = re.sub(pattern, wrap_expr, raw_latex)
        if "$" not in latex_line and any(c in raw_latex for c in ["^{", "_{", r"\times", r"\pm"]):
            latex_line = f"${raw_latex.strip()}$"

    return plain_line.strip(), latex_line.strip(), True


def extract_clean_text_blocks(
    page: fitz.Page,
    header_margin: float,
    footer_margin: float,
) -> List[Dict[str, Any]]:
    """Extract text blocks using dict/span inspection with dual-layer text and LaTeX math."""
    page_dict = page.get_text("dict")
    h = page.rect.height
    clean_blocks: List[Dict[str, Any]] = []

    for b in page_dict.get("blocks", []):
        if b.get("type") != 0:
            continue
        bbox = (b["bbox"][0], b["bbox"][1], b["bbox"][2], b["bbox"][3])

        plain_lines: List[str] = []
        latex_lines: List[str] = []
        block_has_math = False

        for l in b.get("lines", []):
            spans = l.get("spans", [])
            plain_l, latex_l, line_has_math = format_line_math(spans, l["bbox"])
            if plain_l:
                plain_lines.append(plain_l)
                latex_lines.append(latex_l)
            if line_has_math:
                block_has_math = True

        if not plain_lines:
            continue

        full_plain = "\n".join(plain_lines).strip()
        full_latex = "\n".join(latex_lines).strip()

        if is_noise_header_footer(bbox, full_plain, h, header_margin, footer_margin):
            continue

        block_item: Dict[str, Any] = {
            "bbox": [round(c, 1) for c in bbox],
            "text": full_plain,
        }
        if block_has_math and full_latex != full_plain:
            block_item["latex"] = full_latex

        clean_blocks.append(block_item)

    clean_blocks.sort(key=lambda b: (round(b["bbox"][1] / 30) * 30, b["bbox"][0]))
    return clean_blocks


def process_page(
    doc: fitz.Document,
    page_index: int,
    output_dir: Path,
    dpi: int = 150,
    header_margin: float = 45.0,
    footer_margin: float = 50.0,
) -> Dict[str, Any]:
    """Process a single PDF page into page_XXXX.json and page_XXXX.png."""
    page_num = page_index + 1
    page = doc.load_page(page_index)
    w, h = page.rect.width, page.rect.height

    # 1. Save page preview PNG
    png_path = output_dir / f"page_{page_num:04d}.png"
    pix = page.get_pixmap(dpi=dpi)
    pix.save(str(png_path))

    # 2. Extract text blocks and filter header/footer
    clean_blocks = extract_clean_text_blocks(page, header_margin, footer_margin)

    # 3. Clean full text stream in reading order
    text_stream = "\n\n".join(b.get("latex", b["text"]) for b in clean_blocks)

    result_data = {
        "page_number": page_num,
        "width_pt": round(w, 1),
        "height_pt": round(h, 1),
        "text_blocks_count": len(clean_blocks),
        "text_blocks": clean_blocks,
        "text_stream": text_stream,
    }

    json_path = output_dir / f"page_{page_num:04d}.json"
    with open(json_path, "w", encoding="utf-8") as f:
        json.dump(result_data, f, indent=2, ensure_ascii=False)

    return result_data


def main() -> None:
    parser = argparse.ArgumentParser(description="Stage 5: Extract Page Layout and Text Blocks.")
    parser.add_argument("pdf", help="Path to input PDF file")
    parser.add_argument("--out", "-o", default="build/01_page_layout", help="Output directory")
    parser.add_argument("--pages", "-p", help="Page range to process (e.g. 1-10 or 4). Default: all")
    parser.add_argument("--dpi", type=int, default=150, help="Preview image DPI (default: 150)")
    parser.add_argument("--header-margin", type=float, default=45.0, help="Top header margin in pt (default: 45.0)")
    parser.add_argument("--footer-margin", type=float, default=50.0, help="Bottom footer margin in pt (default: 50.0)")
    parser.add_argument("--force", action="store_true", help="Force re-extraction even if page_XXXX.json and page_XXXX.png exist")

    args = parser.parse_args()
    pdf_path = Path(args.pdf).resolve()

    if not pdf_path.exists():
        sys.stderr.write(f"Error: PDF not found: {pdf_path}\n")
        sys.exit(1)

    if pdf_path.is_dir():
        manual_dir = pdf_path.parent if pdf_path.name == "build" else pdf_path
        candidate = None
        build_dir = manual_dir / "build"
        if build_dir.is_dir():
            for f in build_dir.glob("*.pdf"):
                if f.name.endswith("_ocr.pdf"):
                    candidate = f
                    break
            if not candidate:
                for f in build_dir.glob("*.pdf"):
                    if not f.name.endswith("_ocr.pdf"):
                        if candidate is None or len(f.name) < len(candidate.name):
                            candidate = f
        if not candidate:
            for f in manual_dir.glob("*.pdf"):
                if f.name.endswith("_ocr.pdf"):
                    candidate = f
                    break
                else:
                    if candidate is None or len(f.name) < len(candidate.name):
                        candidate = f
        if candidate is None:
            sys.stderr.write(f"Error: No PDF found in directory: {manual_dir} or {build_dir}\n")
            sys.exit(1)
        pdf_path = candidate

    manual_dir = pdf_path.parent.parent if pdf_path.parent.name == "build" else pdf_path.parent
    out_dir = Path(args.out)
    if not out_dir.is_absolute():
        out_dir = manual_dir / args.out
    out_dir.mkdir(parents=True, exist_ok=True)

    doc = fitz.open(str(pdf_path))
    total_pages = len(doc)

    if args.pages:
        page_numbers = parse_page_range(args.pages, total_pages)
    else:
        page_numbers = list(range(1, total_pages + 1))

    # Filter out already extracted pages unless --force
    to_process: List[int] = []
    already_done = 0
    for p_num in page_numbers:
        j_file = out_dir / f"page_{p_num:04d}.json"
        p_file = out_dir / f"page_{p_num:04d}.png"
        if not args.force and j_file.exists() and p_file.exists() and j_file.stat().st_size > 0 and p_file.stat().st_size > 0:
            already_done += 1
        else:
            to_process.append(p_num)

    if already_done > 0 and not args.force:
        print(f"[STAGE 5] {already_done}/{len(page_numbers)} pages already extracted (skipped).")

    if not to_process:
        print(f"[STAGE 5 SUCCESS] All {len(page_numbers)} target pages already present in: {out_dir}")
        return

    print(f"[STAGE 5] Extracting layout for {len(to_process)} missing/requested page(s) from '{pdf_path.name}'...")
    print(f"[STAGE 5] Output directory: {out_dir}")

    for idx, page_num in enumerate(to_process, start=1):
        data = process_page(
            doc,
            page_num - 1,
            out_dir,
            dpi=args.dpi,
            header_margin=args.header_margin,
            footer_margin=args.footer_margin,
        )
        print(f"  - Page {page_num:4d}/{total_pages}: {data['text_blocks_count']} text blocks.")

    print(f"[STAGE 5 SUCCESS] Done. Artifacts saved in: {out_dir}")


if __name__ == "__main__":
    main()

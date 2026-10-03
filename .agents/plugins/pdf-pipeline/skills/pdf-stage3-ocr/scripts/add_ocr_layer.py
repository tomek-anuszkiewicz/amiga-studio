"""Add Searchable OCR Text Layer to Scanned PDFs using PyMuPDF + Tesseract.

Features:
- Injects invisible text line-by-line, ensuring continuous sentences and natural word selection.
- Cleans scanner edge noise (binding marks, crop ticks, scanner bed borders on margins).
- Automatically detects large format schematics / foldouts (e.g. Appendix B Schematics)
  and handles them cleanly (skip OCR, full page scan, or omit).
- Preserves original vector graphics, DPI, and PDF bookmarks (TOC).

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-stage3-ocr/scripts/add_ocr_layer.py "path/to/document.pdf"
    python .agents/plugins/pdf-pipeline/skills/pdf-stage3-ocr/scripts/add_ocr_layer.py "path/to/document.pdf" --sample 10
    python .agents/plugins/pdf-pipeline/skills/pdf-stage3-ocr/scripts/add_ocr_layer.py "path/to/document.pdf" --schematics skip
"""

from __future__ import annotations

import argparse
import os
import shutil
import sys
import time
from pathlib import Path
from typing import List, Optional, Set

# Safe console output on Windows
if sys.stdout.encoding and sys.stdout.encoding.lower() != "utf-8":
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

try:
    import pymupdf as fitz
except ImportError:
    try:
        import fitz  # type: ignore
    except ImportError:
        sys.stderr.write("Error: PyMuPDF is required. Install via 'pip install pymupdf'.\n")
        sys.exit(1)


def find_tessdata_dir(custom_path: Optional[str] = None) -> Optional[str]:
    """Locate the Tesseract tessdata directory on Windows or PATH."""
    if custom_path:
        p = Path(custom_path)
        if (p / "eng.traineddata").is_file():
            return str(p.resolve())
        if p.name.lower() != "tessdata" and (p / "tessdata" / "eng.traineddata").is_file():
            return str((p / "tessdata").resolve())

    prefix = os.environ.get("TESSDATA_PREFIX")
    if prefix:
        p = Path(prefix)
        if (p / "eng.traineddata").is_file():
            return str(p.resolve())
        if (p / "tessdata" / "eng.traineddata").is_file():
            return str((p / "tessdata").resolve())

    program_files = os.environ.get("PROGRAMFILES", "C:\\Program Files")
    program_files_x86 = os.environ.get("PROGRAMFILES(X86)", "C:\\Program Files (x86)")
    local_app_data = os.environ.get("LOCALAPPDATA", "")

    candidates = [
        Path(program_files) / "Tesseract-OCR" / "tessdata",
        Path(program_files_x86) / "Tesseract-OCR" / "tessdata",
        Path(local_app_data) / "Programs" / "Tesseract-OCR" / "tessdata",
        Path(r"C:\Program Files\Tesseract-OCR\tessdata"),
    ]

    for candidate in candidates:
        if (candidate / "eng.traineddata").is_file():
            return str(candidate.resolve())

    tess_exe = shutil.which("tesseract")
    if tess_exe:
        p = Path(tess_exe).parent / "tessdata"
        if (p / "eng.traineddata").is_file():
            return str(p.resolve())

    return None


def parse_page_range(range_str: str, total_pages: int) -> List[int]:
    """Parse string like '1-10,15,20-25' into 0-indexed page indices."""
    pages: Set[int] = set()
    parts = [p.strip() for p in range_str.split(",") if p.strip()]
    for part in parts:
        if "-" in part:
            sub = part.split("-", 1)
            start = int(sub[0].strip()) if sub[0].strip() else 1
            end = int(sub[1].strip()) if sub[1].strip() else total_pages
            for p in range(max(1, start), min(total_pages, end) + 1):
                pages.add(p - 1)
        else:
            p = int(part)
            if 1 <= p <= total_pages:
                pages.add(p - 1)
    return sorted(list(pages))


def is_edge_noise(
    bbox: tuple[float, float, float, float],
    text: str,
    page_w: float,
    page_h: float,
    margin_x: float = 26.0,
    margin_y: float = 22.0,
) -> bool:
    """Check if a recognized text chunk is scanner edge noise / margin tick marks."""
    clean_text = text.strip()
    if not clean_text:
        return True

    x0, y0, x1, y1 = bbox
    is_left = x1 < margin_x + 5
    is_right = x0 > (page_w - margin_x)
    is_top = y1 < margin_y
    is_bottom = y0 > (page_h - margin_y)

    # Isolated short symbol on or near margin
    noise_chars = "-_~|Ili`'.,:;=>< /\\?\\#\ufffd°"
    is_symbol_only = len(clean_text) <= 4 and all(c in noise_chars for c in clean_text)

    if (is_left or is_right or is_top or is_bottom) and is_symbol_only:
        return True

    # Extreme edge items with very short content (e.g. 1-2 chars outside normal text margins)
    if (x0 > page_w - 20 or x1 < 22) and len(clean_text) <= 2:
        return True

    return False


def is_schematic_page(page: fitz.Page) -> bool:
    """Detect if page is a large format foldout schematic drawing."""
    return page.rect.width > 1000 or page.rect.height > 1200


def add_ocr_layer(
    input_pdf: Path,
    output_pdf: Path,
    pages_to_process: Optional[List[int]] = None,
    dpi: int = 300,
    lang: str = "eng",
    tessdata_path: Optional[str] = None,
    margin_filter: bool = True,
    margin_size: float = 26.0,
    schematics_mode: str = "skip",
    force: bool = False,
) -> None:
    """Run line-level OCR across pages and produce output searchable PDF."""
    resolved_tessdata = find_tessdata_dir(tessdata_path)
    if not resolved_tessdata:
        sys.stderr.write(
            "\n"
            + "=" * 70 + "\n"
            + " [ERROR] Tesseract tessdata directory with 'eng.traineddata' was not found!\n"
            + "=" * 70 + "\n"
            + "To install Tesseract on Windows:\n"
            + "  1. Run in PowerShell:\n"
            + "       powershell -ExecutionPolicy Bypass -File scripts/install_tesseract.ps1\n"
            + "  2. Or download directly from:\n"
            + "       https://github.com/UB-Mannheim/tesseract/wiki\n"
            + "=" * 70 + "\n\n"
        )
        sys.exit(1)

    os.environ["TESSDATA_PREFIX"] = resolved_tessdata

    print(f"[OCR] Using tessdata:     {resolved_tessdata}")
    print(f"[OCR] Input file:        {input_pdf}")
    print(f"[OCR] Output file:       {output_pdf}")

    src_doc = fitz.open(str(input_pdf))
    total_pages = len(src_doc)
    bookmarks = src_doc.get_toc()

    target_pages = pages_to_process if pages_to_process is not None else list(range(total_pages))
    target_set = set(target_pages)
    num_to_process = len(target_pages)

    print(f"[OCR] Total document pages: {total_pages}, to process: {num_to_process}")
    print(f"[OCR] Resolution:          {dpi} DPI | Language: '{lang}'")
    print(f"[OCR] Margin Filter:       {'Enabled (' + str(margin_size) + ' pt)' if margin_filter else 'Disabled'}")
    print(f"[OCR] Schematics Mode:     '{schematics_mode}' (skip = preserve image without OCR noise)")

    out_doc = fitz.open()
    t0 = time.perf_counter()
    processed_count = 0
    skipped_schematics = 0

    for p_idx in range(total_pages):
        page = src_doc[p_idx]
        is_large_schematic = is_schematic_page(page)

        # Handle schematics
        if is_large_schematic:
            if schematics_mode == "omit":
                continue
            elif schematics_mode == "skip":
                new_page = out_doc.new_page(width=page.rect.width, height=page.rect.height)
                new_page.show_pdf_page(page.rect, src_doc, p_idx)
                if p_idx in target_set:
                    skipped_schematics += 1
                    processed_count += 1
                    progress = (processed_count / num_to_process) * 100
                    print(
                        f"[{processed_count:3d}/{num_to_process:3d}] ({progress:5.1f}%) "
                        f"Page {p_idx + 1:3d}: Large schematic ({int(page.rect.width)}x{int(page.rect.height)} pt) -> Preserved image (OCR skipped)"
                    )
                continue

        # Standard page: copy background graphics
        new_page = out_doc.new_page(width=page.rect.width, height=page.rect.height)
        new_page.show_pdf_page(page.rect, src_doc, p_idx)

        if p_idx not in target_set:
            continue

        existing_text = page.get_text().strip()
        if len(existing_text) >= 20 and not force:
            processed_count += 1
            progress = (processed_count / num_to_process) * 100
            print(
                f"[{processed_count:3d}/{num_to_process:3d}] ({progress:5.1f}%) "
                f"Page {p_idx + 1:3d}: Already has text layer ({len(existing_text)} chars) -> Preserved native text (OCR skipped)"
            )
            continue

        t_page_start = time.perf_counter()
        effective_dpi = min(dpi, 150) if is_large_schematic else dpi
        tp = page.get_textpage_ocr(language=lang, dpi=effective_dpi, tessdata=resolved_tessdata)
        d = page.get_text("dict", textpage=tp)

        line_count = 0
        w = page.rect.width
        h = page.rect.height

        for block in d.get("blocks", []):
            if block.get("type") == 0:  # text block
                for line in block.get("lines", []):
                    spans = line.get("spans", [])
                    if not spans:
                        continue
                    line_text = "".join(s.get("text", "") for s in spans).strip()
                    if not line_text:
                        continue

                    if margin_filter and not is_large_schematic:
                        if is_edge_noise(line["bbox"], line_text, w, h, margin_x=margin_size):
                            continue

                    first_span = spans[0]
                    origin = first_span.get("origin", (line["bbox"][0], line["bbox"][3]))
                    size = first_span.get("size", 10.0)

                    new_page.insert_text(
                        fitz.Point(origin[0], origin[1]),
                        line_text,
                        fontsize=size,
                        render_mode=3,
                    )
                    line_count += 1

        t_page_elapsed = time.perf_counter() - t_page_start
        processed_count += 1
        progress = (processed_count / num_to_process) * 100
        print(
            f"[{processed_count:3d}/{num_to_process:3d}] ({progress:5.1f}%) "
            f"Page {p_idx + 1:3d}: {line_count:3d} lines injected in {t_page_elapsed:.2f}s"
        )

    if bookmarks and len(target_set) == total_pages and schematics_mode != "omit":
        try:
            out_doc.set_toc(bookmarks)
            print("[OCR] Preserved original table of contents (bookmarks).")
        except Exception:
            pass

    output_pdf.parent.mkdir(parents=True, exist_ok=True)
    tmp_out = output_pdf.with_name(f"{output_pdf.stem}_tmp_{os.getpid()}.pdf")
    out_doc.save(str(tmp_out), deflate=True)
    out_doc.close()
    src_doc.close()

    if output_pdf.exists():
        output_pdf.unlink()
    shutil.move(str(tmp_out), str(output_pdf))

    total_time = time.perf_counter() - t0
    avg_per_page = total_time / max(1, processed_count)
    print(
        f"\n[OCR SUCCESS] Saved: {output_pdf}\n"
        f"              Processed {processed_count} pages in {total_time:.1f}s "
        f"({avg_per_page:.2f}s per page)\n"
    )
    if skipped_schematics > 0:
        print(f"              [NOTE] {skipped_schematics} schematic foldouts preserved with pure image quality (no OCR noise).\n")


def check_has_text(
    pdf_path: Path,
    target_pages: Optional[List[int]] = None,
    min_chars: int = 20,
) -> tuple[bool, List[int]]:
    """Check whether all target pages (or all document pages) have an extractable text layer.

    Returns:
        (has_full_text, missing_pages_1_indexed)
    """
    doc = fitz.open(str(pdf_path))
    total = len(doc)
    pages = target_pages if target_pages is not None else list(range(total))
    missing = []
    for p_idx in pages:
        if 0 <= p_idx < total:
            page = doc[p_idx]
            if is_schematic_page(page):
                continue
            text = page.get_text().strip()
            if len(text) < min_chars:
                missing.append(p_idx + 1)
    doc.close()
    return len(missing) == 0, missing


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Add a searchable OCR text layer to scanned PDFs with continuous line-level selection and margin cleaning."
    )
    parser.add_argument("input_pdf", type=str, help="Path to input scanned PDF file.")
    parser.add_argument("-o", "--output", type=str, default=None, help="Path to output PDF.")
    parser.add_argument("--check-only", action="store_true", help="Only verify if text layer exists without running OCR.")
    parser.add_argument("--force", action="store_true", help="Force re-running OCR even if the PDF already has an extractable text layer.")
    parser.add_argument("--overwrite", action="store_true", help="Overwrite existing <stem>_ocr.pdf copy.")
    parser.add_argument("--pages", type=str, default=None, help="Page range to process, e.g. '1-10'.")
    parser.add_argument("--sample", type=int, default=None, help="Process first N pages only.")
    parser.add_argument("--dpi", type=int, default=300, help="OCR DPI (default: 300).")
    parser.add_argument("--lang", type=str, default="eng", help="Language code (default: 'eng').")
    parser.add_argument("--no-margin-filter", action="store_true", help="Disable margin noise filter.")
    parser.add_argument("--margin-size", type=float, default=26.0, help="Margin width in pt.")
    parser.add_argument("--schematics", choices=["skip", "full", "omit"], default="skip")
    parser.add_argument("--tessdata", type=str, default=None)

    args = parser.parse_args()
    input_path = Path(args.input_pdf).resolve()
    if input_path.is_dir():
        manual_dir = input_path.parent if input_path.name == "build" else input_path
        build_dir = manual_dir / "build"
        candidate = None
        if build_dir.is_dir():
            for f in build_dir.glob("*.pdf"):
                if not f.name.endswith("_ocr.pdf"):
                    if candidate is None or len(f.name) < len(candidate.name):
                        candidate = f
        if not candidate:
            for f in manual_dir.glob("*.pdf"):
                if not f.name.endswith("_ocr.pdf"):
                    if candidate is None or len(f.name) < len(candidate.name):
                        candidate = f
        if candidate is None:
            sys.stderr.write(f"Error: No source PDF found in directory: {manual_dir} or {build_dir}\n")
            sys.exit(1)
        input_path = candidate

    if not input_path.is_file():
        sys.stderr.write(f"Error: Input file does not exist: {input_path}\n")
        sys.exit(1)

    src = fitz.open(str(input_path))
    total_pages = len(src)
    src.close()

    target_pages = None
    if args.sample:
        target_pages = list(range(min(args.sample, total_pages)))
    elif args.pages:
        target_pages = parse_page_range(args.pages, total_pages)

    has_full_text, missing_pages = check_has_text(input_path, target_pages=target_pages)
    if args.check_only:
        if has_full_text:
            print(f"[STAGE 3 CHECK] '{input_path.name}' has complete extractable text layer: True (all evaluated pages have text)")
            sys.exit(0)
        else:
            sample_str = ", ".join(str(p) for p in missing_pages[:15])
            more_str = f" ... and {len(missing_pages) - 15} more" if len(missing_pages) > 15 else ""
            print(f"[STAGE 3 CHECK] '{input_path.name}' has extractable text layer: False ({len(missing_pages)} page(s) lack text: {sample_str}{more_str})")
            sys.exit(1)

    if args.output:
        output_path = Path(args.output).resolve()
    else:
        if input_path.parent.name == "build":
            output_path = input_path.parent / f"{input_path.stem}_ocr.pdf"
        else:
            build_dir = input_path.parent / "build"
            output_path = build_dir / f"{input_path.stem}_ocr.pdf"
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Invariant: If all target pages already have an extractable text layer, avoid unnecessary OCR
    if has_full_text and not args.force:
        print(f"[STAGE 3] PDF '{input_path.name}' already contains complete extractable text on all pages. OCR not needed.")
        if input_path.resolve() != output_path.resolve():
            if not output_path.exists() or args.overwrite:
                shutil.copy2(input_path, output_path)
                print(f"[STAGE 3] Copied original to: '{output_path.name}' for downstream pipeline consistency.")
            else:
                print(f"[STAGE 3] Pipeline target '{output_path.name}' already exists. Using as source.")
        else:
            print(f"[STAGE 3] Input is already: '{input_path.name}'.")

        print(f"[STAGE 3 SUCCESS] Ready for Stage 4: {output_path}")
        return

    sample_str = ", ".join(str(p) for p in missing_pages[:10])
    more_str = f" ... and {len(missing_pages) - 10} more" if len(missing_pages) > 10 else ""
    if missing_pages:
        print(f"[STAGE 3] PDF '{input_path.name}' requires OCR: {len(missing_pages)} page(s) lack text ({sample_str}{more_str}).")
    elif args.force:
        print(f"[STAGE 3] Force re-running OCR on '{input_path.name}'.")

    add_ocr_layer(
        input_pdf=input_path,
        output_pdf=output_path,
        pages_to_process=target_pages,
        dpi=args.dpi,
        lang=args.lang,
        tessdata_path=args.tessdata,
        margin_filter=not args.no_margin_filter,
        margin_size=args.margin_size,
        schematics_mode=args.schematics,
        force=args.force,
    )


if __name__ == "__main__":
    main()

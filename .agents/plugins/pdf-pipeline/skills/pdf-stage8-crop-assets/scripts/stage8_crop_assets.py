"""Stage 8: Crop Visual Assets and Inject Markdown Links.

Non-destructive asset extraction step.
Reads markdown files from Stage 7 containing <crop box="[ymin, xmin, ymax, xmax]" ... /> tags.
Fetches ground-truth bounding box coordinates from Stage 5 JSON metadata or parses normalized [0, 1000] boxes.
Extracts high-resolution (300 DPI) PNG crops from the original PDF to `assets/`.
Replaces <crop ... /> tags with standard markdown image links:
  ![Caption](assets/filename.png)
Outputs to: build/01_page_layout/ (page_XXXX-images.md)

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py
    python .agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py "path/to/manual_dir"
    python .agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py "path/to/manual_ocr.pdf"
    python .agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py --pages 4-15
    python .agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py "path/to/manual_ocr.pdf" --dpi 600
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

try:
    import pymupdf as fitz
except ImportError:
    try:
        import fitz  # type: ignore
    except ImportError:
        sys.stderr.write("Error: PyMuPDF (fitz) is required. Install via 'pip install pymupdf'.\n")
        sys.exit(1)


def sanitize_filename(name: str) -> str:
    """Create a filesystem-friendly slug from caption or ID."""
    s = name.strip().lower()
    s = re.sub(r"[^\w\s-]", "", s)
    s = re.sub(r"[\s_]+", "_", s)
    return s[:60].strip("_")


def parse_tag_attributes(tag_str: str) -> Dict[str, str]:
    """Extract key="value" pairs from an HTML-style <crop ...> tag."""
    attrs: Dict[str, str] = {}
    pattern = re.compile(r'([a-zA-Z_]+)\s*=\s*(?:"([^"]*)"|\'([^\']*)\'|\[([^\]]*)\]|(\S+))')
    for match in pattern.finditer(tag_str):
        key = match.group(1).lower()
        val = match.group(2) or match.group(3) or match.group(4) or match.group(5)
        attrs[key] = val
    return attrs


def load_page_json(layout_dir: Path, page_num: int) -> Optional[Dict[str, Any]]:
    """Load Stage 5 JSON metadata for a specific page."""
    json_path = layout_dir / f"page_{page_num:04d}.json"
    if not json_path.exists():
        return None
    try:
        with open(json_path, "r", encoding="utf-8") as f:
            return json.load(f)
    except Exception as e:
        sys.stderr.write(f"Warning: Could not read {json_path}: {e}\n")
        return None


def descale_box(
    box_str: str,
    page_width: float,
    page_height: float,
) -> Optional[Tuple[float, float, float, float]]:
    """Descale normalized [0, 1000] integer coordinates to PDF dimensions.

    Box format: [ymin, xmin, ymax, xmax] normalized to 1000x1000.
    Returns: (x0, y0, x1, y1) in PDF points.
    """
    cleaned = box_str.strip("[]() \t\r\n")
    parts = [p.strip() for p in cleaned.split(",") if p.strip()]
    if len(parts) != 4:
        return None

    try:
        ymin_n, xmin_n, ymax_n, xmax_n = [float(p) for p in parts]
    except ValueError:
        return None

    x0 = (xmin_n / 1000.0) * page_width
    y0 = (ymin_n / 1000.0) * page_height
    x1 = (xmax_n / 1000.0) * page_width
    y1 = (ymax_n / 1000.0) * page_height

    # Validate and clamp
    x0 = max(0.0, min(page_width, x0))
    y0 = max(0.0, min(page_height, y0))
    x1 = max(0.0, min(page_width, x1))
    y1 = max(0.0, min(page_height, y1))

    if x1 <= x0 or y1 <= y0:
        return None

    return (x0, y0, x1, y1)


def find_box_by_id(
    candidate_id: str,
    page_json: Optional[Dict[str, Any]],
) -> Optional[Tuple[float, float, float, float]]:
    """Look up exact PDF bbox for a legacy candidate ID from Stage 5 metadata."""
    if not page_json:
        return None
    for cand in page_json.get("visual_candidates", []):
        if cand.get("candidate_id") == candidate_id:
            b = cand.get("bbox")
            if b and len(b) == 4:
                return (float(b[0]), float(b[1]), float(b[2]), float(b[3]))
    return None


def crop_region_to_png(
    page: fitz.Page,
    bbox: Tuple[float, float, float, float],
    output_path: Path,
    dpi: int = 300,
    padding: float = 3.0,
) -> bool:
    """Render a cropped bounding box from a PDF page to a crisp PNG image."""
    x0, y0, x1, y1 = bbox
    p_rect = page.rect

    # Add padding and clamp to page rect
    x0 = max(p_rect.x0, x0 - padding)
    y0 = max(p_rect.y0, y0 - padding)
    x1 = min(p_rect.x1, x1 + padding)
    y1 = min(p_rect.y1, y1 + padding)

    crop_rect = fitz.Rect(x0, y0, x1, y1)
    if crop_rect.is_empty or crop_rect.is_infinite:
        return False

    # Calculate zoom scale based on desired DPI (default PDF is 72 DPI)
    scale = dpi / 72.0
    mat = fitz.Matrix(scale, scale)

    try:
        pix = page.get_pixmap(matrix=mat, clip=crop_rect, alpha=False)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        pix.save(str(output_path))
        return True
    except Exception as e:
        sys.stderr.write(f"Error rendering crop to {output_path}: {e}\n")
        return False


def to_relative_posix(target_path: Path, base_dir: Path) -> str:
    """Format path relative to base_dir using forward slashes."""
    t = target_path.resolve()
    b = base_dir.resolve()
    try:
        return str(t.relative_to(b)).replace("\\", "/")
    except ValueError:
        try:
            return str(t.relative_to(Path.cwd().resolve())).replace("\\", "/")
        except ValueError:
            return str(target_path.name)


def process_markdown_file(
    md_path: Path,
    pdf_path: Path,
    layout_dir: Path,
    out_dir: Path,
    assets_dir: Path,
    dpi: int = 300,
    force: bool = False,
) -> Tuple[int, int, List[Dict[str, Any]]]:
    """Process a single Stage 7 markdown file, crop visual assets, and inject image links."""
    content = md_path.read_text(encoding="utf-8")

    # Determine page number from filename
    m = re.search(r"page_(\d+)", md_path.name, re.IGNORECASE)
    page_num = int(m.group(1)) if m else None

    # Load Stage 5 metadata if available
    page_json = load_page_json(layout_dir, page_num) if page_num else None

    doc = fitz.open(pdf_path)
    if page_num is None or page_num < 1 or page_num > len(doc):
        # Default to first page if undetermined
        pdf_page_idx = 0
    else:
        pdf_page_idx = page_num - 1

    pdf_page = doc[pdf_page_idx]
    page_w = pdf_page.rect.width
    page_h = pdf_page.rect.height

    crop_tag_pattern = re.compile(r"<crop\b([^>]*)/?>", re.IGNORECASE)

    crops_found = 0
    crops_succeeded = 0
    crop_records: List[Dict[str, Any]] = []

    out_md_name = md_path.stem + "-images.md"
    out_md_path = out_dir / out_md_name

    def replace_crop_tag(match: re.Match) -> str:
        nonlocal crops_found, crops_succeeded
        crops_found += 1
        tag_body = match.group(1)
        attrs = parse_tag_attributes(tag_body)

        caption = attrs.get("caption", "").strip()
        box_str = attrs.get("box", "").strip()
        if not box_str and all(k in attrs for k in ("ymin", "xmin", "ymax", "xmax")):
            box_str = f"[{attrs['ymin']}, {attrs['xmin']}, {attrs['ymax']}, {attrs['xmax']}]"
        cid = attrs.get("id", "").strip()

        bbox: Optional[Tuple[float, float, float, float]] = None

        # Priority 1: Direct normalized bounding box [ymin, xmin, ymax, xmax]
        if box_str:
            bbox = descale_box(box_str, page_w, page_h)

        # Priority 2: Fallback to Stage 5 candidate ID
        if bbox is None and cid:
            bbox = find_box_by_id(cid, page_json)

        # Priority 3: Fallback full page if front cover or full page graphic
        if bbox is None and (attrs.get("full_page") == "true" or "cover" in caption.lower()):
            bbox = (0.0, 0.0, page_w, page_h)

        if bbox is None:
            sys.stderr.write(f"Warning: Could not determine bounding box for crop tag in {md_path.name}: {match.group(0)}\n")
            return match.group(0)

        # Calculate normalized box [ymin, xmin, ymax, xmax] (0-1000 scale)
        norm_box: List[int] = [0, 0, 1000, 1000]
        if box_str:
            try:
                cleaned = box_str.strip("[]() \t\r\n")
                norm_box = [int(round(float(p))) for p in cleaned.split(",") if p.strip()]
            except Exception:
                norm_box = [
                    int(round((bbox[1] / page_h) * 1000)),
                    int(round((bbox[0] / page_w) * 1000)),
                    int(round((bbox[3] / page_h) * 1000)),
                    int(round((bbox[2] / page_w) * 1000)),
                ]
        elif bbox:
            norm_box = [
                int(round((bbox[1] / page_h) * 1000)),
                int(round((bbox[0] / page_w) * 1000)),
                int(round((bbox[3] / page_h) * 1000)),
                int(round((bbox[2] / page_w) * 1000)),
            ]

        # Generate asset filename
        p_prefix = f"page_{page_num:04d}" if page_num else "crop"
        if caption:
            slug = sanitize_filename(caption)
            img_filename = f"{p_prefix}_{slug}.png"
        elif cid:
            img_filename = f"{p_prefix}_{sanitize_filename(cid)}.png"
        else:
            img_filename = f"{p_prefix}_crop_{crops_found}.png"

        img_path = assets_dir / img_filename

        if not force and img_path.is_file() and img_path.stat().st_size > 0:
            success = True
        else:
            success = crop_region_to_png(pdf_page, bbox, img_path, dpi=dpi)
        if not success:
            return match.group(0)

        crops_succeeded += 1

        manual_root = pdf_path.parent.parent.resolve() if pdf_path.parent.name == "build" else pdf_path.parent.resolve()
        rel_asset = to_relative_posix(img_path, manual_root)
        rel_md = to_relative_posix(out_md_path, manual_root)
        is_full_page = (norm_box == [0, 0, 1000, 1000])

        crop_records.append({
            "asset_id": Path(img_filename).stem,
            "page_number": page_num or 0,
            "asset": rel_asset,
            "md_file": rel_md,
            "original_asset": rel_asset,
            "active_asset": rel_asset,
            "final_asset": None,
            "status": "eval_ok" if is_full_page else "pending",
            "eval_clip": "ok" if is_full_page else None,
            "detected_type": "image" if is_full_page else None,
            "current_version": 0,
            "active_box": norm_box,
            "versions": [
                {
                    "version": 0,
                    "box": norm_box,
                    "asset_file": rel_asset,
                    "source": "stage8_initial_crop" if not is_full_page else "stage8_full_page_asset",
                    "eval_result": "ok" if is_full_page else None,
                }
            ],
        })

        # Use relative asset path for Markdown
        try:
            rel_asset_path = str(img_path.relative_to(out_dir)).replace("\\", "/")
        except ValueError:
            rel_asset_path = f"assets/{img_filename}"

        img_caption = caption or f"Illustration Page {page_num}"
        return f"![{img_caption}]({rel_asset_path})"

    updated_content = crop_tag_pattern.sub(replace_crop_tag, content)
    doc.close()

    # Determine final markdown filename
    out_md_path.write_text(updated_content, encoding="utf-8")

    print(f"  Processed {md_path.name}: {crops_succeeded}/{crops_found} visual crops extracted -> {out_md_name}")
    return crops_succeeded, crops_found, crop_records


def parse_page_range(range_str: str) -> Set[int]:
    """Parse range string like '1-10', '5', '1,3,5-7'."""
    pages: Set[int] = set()
    for part in range_str.split(","):
        part = part.strip()
        if not part:
            continue
        if "-" in part:
            s_str, e_str = part.split("-", 1)
            pages.update(range(int(s_str.strip()), int(e_str.strip()) + 1))
        else:
            pages.add(int(part))
    return pages


TARGET_MANUALS: List[str] = [
    "68000 Programmer's Reference Manual",
    "68000 User's Manual",
    "A500 A2000 Technical Reference Manual",
    "Hardware Reference Manual",
]


def resolve_manual_targets(manual_input: Optional[str]) -> List[str]:
    """Resolve target manual list. If None, empty, '.', or 'all', discover all existing target manuals."""
    if manual_input and manual_input.strip().lower() not in ("all", "."):
        return [manual_input.strip()]

    # If current working directory is itself a manual directory containing PDF files
    cwd_pdfs = list(Path.cwd().glob("*.pdf"))
    if cwd_pdfs and (not manual_input or manual_input.strip() != "all"):
        return [str(Path.cwd())]

    # Discover which of the four target manuals exist in cwd
    found = [m for m in TARGET_MANUALS if (Path.cwd() / m).is_dir()]
    if found:
        return found

    # Fallback to discovering any subdirectories containing .pdf files
    discovered = []
    for p in sorted(Path.cwd().iterdir()):
        if p.is_dir() and not p.name.startswith(".") and p.name != "build" and not p.name.endswith("-old"):
            if any(p.glob("*.pdf")):
                discovered.append(p.name)
    return discovered if discovered else TARGET_MANUALS


def resolve_pdf_for_target(target: str | Path) -> Tuple[Optional[Path], Optional[Path]]:
    """Resolve (manual_dir, pdf_path) from a path to a PDF file or a manual directory."""
    p = Path(target)
    if not p.is_absolute():
        p = (Path.cwd() / p).resolve()

    if p.is_file() and p.suffix.lower() == ".pdf":
        manual_dir = p.parent.parent if p.parent.name == "build" else p.parent
        return manual_dir, p
    elif p.is_dir():
        manual_dir = p.parent if p.name == "build" else p
        pdf_files = []
        build_dir = manual_dir / "build"
        if build_dir.is_dir():
            pdf_files.extend(list(build_dir.glob("*.pdf")))
        pdf_files.extend(list(manual_dir.glob("*.pdf")))
        ocr_candidates = [f for f in pdf_files if "_ocr" in f.name]
        pdf_path = ocr_candidates[0] if ocr_candidates else (pdf_files[0] if pdf_files else None)
        return manual_dir, pdf_path
    return None, None


def process_manual(
    pdf_path: Path,
    markdown_arg: str = "build/01_page_layout",
    layout_arg: str = "build/01_page_layout",
    out_arg: str = "build/01_page_layout",
    pages: Optional[str] = None,
    file_arg: Optional[str] = None,
    dpi: int = 300,
    force: bool = False,
) -> bool:
    """Execute Stage 8 crop extraction and assets queue bootstrapping for a single manual."""
    if not pdf_path.exists():
        sys.stderr.write(f"Error: PDF not found: {pdf_path}\n")
        return False

    manual_dir = pdf_path.parent.parent.resolve() if pdf_path.parent.name == "build" else pdf_path.parent.resolve()

    def resolve_dir(target_str: str) -> Path:
        p = Path(target_str)
        if p.is_absolute():
            return p
        return (manual_dir / p).resolve()

    md_dir = resolve_dir(markdown_arg)
    layout_dir = resolve_dir(layout_arg)
    out_dir = resolve_dir(out_arg)

    assets_dir = out_dir / "assets"
    assets_dir.mkdir(parents=True, exist_ok=True)

    if file_arg:
        target_file = Path(file_arg)
        if not target_file.is_absolute():
            if (manual_dir / target_file).exists():
                target_file = manual_dir / target_file
            elif (Path.cwd() / target_file).exists():
                target_file = Path.cwd() / target_file
            else:
                target_file = manual_dir / target_file
        if not target_file.exists():
            sys.stderr.write(f"Error: Specified file not found: {target_file}\n")
            return False
        md_files = [target_file]
    else:
        if not md_dir.exists():
            sys.stderr.write(f"Error: Markdown input directory does not exist: {md_dir}\n")
            return False
        md_files = sorted(list(md_dir.glob("*.md")))

    # Exclude derived files (e.g. -images.md, -images-clip_final.md, -embed.md)
    md_files = [f for f in md_files if "-images" not in f.name and "-embed" not in f.name]

    target_pages = parse_page_range(pages) if pages else None
    if target_pages:
        filtered_files = []
        for mf in md_files:
            m = re.search(r"page_(\d+)", mf.name, re.IGNORECASE)
            if m:
                if int(m.group(1)) in target_pages:
                    filtered_files.append(mf)
            else:
                filtered_files.append(mf)
        md_files = filtered_files

    if not md_files:
        print(f"[STAGE 8] [{manual_dir.name}] No matching .md files to process. Target pages: {pages or 'all'}")
        return True

    print(f"\n[STAGE 8] [{manual_dir.name}] Processing {len(md_files)} markdown file(s) (Pages: {pages or 'all'})...")
    print(f"[STAGE 8] [{manual_dir.name}] Assets will be saved to: {to_relative_posix(assets_dir, manual_dir)}")
    print(f"[STAGE 8] [{manual_dir.name}] Final documents saved to: {to_relative_posix(out_dir, manual_dir)}")

    all_crop_records: List[Dict[str, Any]] = []
    for md_file in md_files:
        _, _, records = process_markdown_file(md_file, pdf_path, layout_dir, out_dir, assets_dir, dpi=dpi, force=force)
        all_crop_records.extend(records)

    # Bootstrap or update assets queue JSON in build/ (or fallback to manual root)
    build_dir = manual_dir / "build"
    candidate_q = build_dir / f"{pdf_path.stem}_assets_queue.json"
    if not candidate_q.exists() and (manual_dir / f"{pdf_path.stem}_assets_queue.json").exists():
        queue_file = manual_dir / f"{pdf_path.stem}_assets_queue.json"
    else:
        queue_file = candidate_q
    existing_items: Dict[str, Any] = {}
    if queue_file.is_file():
        try:
            with open(queue_file, "r", encoding="utf-8") as f:
                qdata = json.load(f)
                items_list = qdata.get("queue", []) if isinstance(qdata, dict) else qdata
                for it in items_list:
                    aid = it.get("asset_id") or Path(it.get("asset", "")).stem
                    if aid:
                        existing_items[aid] = it
        except Exception:
            pass

    for rec in all_crop_records:
        aid = rec["asset_id"]
        if aid in existing_items and not force:
            ex = existing_items[aid]
            if ex.get("current_version", 0) > 0 or ex.get("eval_clip") is not None or ex.get("detected_type") is not None:
                rec["active_asset"] = ex.get("active_asset", rec["active_asset"])
                rec["final_asset"] = ex.get("final_asset")
                if ex.get("final_asset"):
                    rec["asset"] = ex.get("final_asset")
                rec["status"] = ex.get("status", "pending")
                rec["eval_clip"] = ex.get("eval_clip")
                rec["detected_type"] = ex.get("detected_type")
                rec["current_version"] = ex.get("current_version", 0)
                rec["active_box"] = ex.get("active_box", rec["active_box"])
                rec["versions"] = ex.get("versions", rec["versions"])
            if ex.get("eval_frame"):
                rec["eval_frame"] = ex.get("eval_frame")
        existing_items[aid] = rec

    # Prune stale assets from existing_items for pages that were reprocessed
    processed_page_nums: Set[int] = set()
    for mf in md_files:
        m = re.search(r"page_(\d+)", mf.name, re.IGNORECASE)
        if m:
            processed_page_nums.add(int(m.group(1)))

    new_asset_ids = {rec["asset_id"] for rec in all_crop_records}
    stale_asset_ids = [
        aid for aid, it in list(existing_items.items())
        if it.get("page_number") in processed_page_nums and aid not in new_asset_ids
    ]
    for aid in stale_asset_ids:
        del existing_items[aid]

    queue_list = sorted(list(existing_items.values()), key=lambda x: (x.get("page_number", 0), x.get("asset_id", "")))

    completed = sum(1 for it in queue_list if it.get("status") == "completed")
    checked = sum(1 for it in queue_list if it.get("status") == "checked")
    pending = sum(1 for it in queue_list if it.get("status") == "pending")
    by_status: Dict[str, int] = {}
    by_type: Dict[str, int] = {}
    by_eval_clip: Dict[str, int] = {}
    for it in queue_list:
        st = it.get("status", "pending")
        by_status[st] = by_status.get(st, 0) + 1
        dt = it.get("detected_type")
        if dt:
            by_type[dt] = by_type.get(dt, 0) + 1
        ev = it.get("eval_clip")
        if ev:
            by_eval_clip[ev] = by_eval_clip.get(ev, 0) + 1

    layout_rel = to_relative_posix(layout_dir, manual_dir)

    queue_data = {
        "pdf_file": pdf_path.name,
        "total_assets": len(queue_list),
        "layout_dir": layout_rel,
        "stats": {
            "total": len(queue_list),
            "completed": completed,
            "checked": checked,
            "pending": pending,
            "by_status": by_status,
            "by_type": by_type,
            "by_eval_clip": by_eval_clip,
        },
        "queue": queue_list,
    }

    with open(queue_file, "w", encoding="utf-8") as f:
        json.dump(queue_data, f, indent=2, ensure_ascii=False)

    print(f"[STAGE 8 SUCCESS] [{manual_dir.name}] Assets queue saved: {queue_file.name} ({len(queue_list)} assets with bounding box metadata)")
    return True


def main() -> None:
    parser = argparse.ArgumentParser(description="Stage 8: Crop Visual Assets and Inject Markdown Links.")
    parser.add_argument(
        "pdf",
        nargs="?",
        default=None,
        help="Path to input PDF file, manual directory, or omit / 'all' to process all books",
    )
    parser.add_argument("--markdown", "-m", default="build/01_page_layout", help="Input directory containing Stage 7 markdown files")
    parser.add_argument("--layout", "-l", default="build/01_page_layout", help="Stage 5 layout directory (for ground-truth JSON)")
    parser.add_argument("--out", "-o", default="build/01_page_layout", help="Final output directory (default: build/01_page_layout)")
    parser.add_argument("--pages", "-p", help="Optional page range to filter (e.g. '1-10', '5')")
    parser.add_argument("--file", "-f", help="Specific markdown file to process instead of entire directory")
    parser.add_argument("--dpi", type=int, default=300, help="Output image DPI (default: 300)")
    parser.add_argument("--force", action="store_true", help="Force re-rendering of existing asset images")

    args = parser.parse_args()

    # If --file is passed and no manual/pdf target is specified, deduce manual directory from file path
    if args.file and not args.pdf:
        target_f = Path(args.file)
        if not target_f.is_absolute():
            target_f = (Path.cwd() / target_f).resolve()
        curr = target_f.parent
        found_target = None
        while curr != curr.parent:
            if any(curr.glob("*.pdf")):
                found_target = curr
                break
            curr = curr.parent
        targets = [str(found_target)] if found_target else resolve_manual_targets(None)
    else:
        targets = resolve_manual_targets(args.pdf)

    all_ok = True
    for target in targets:
        manual_dir, pdf_path = resolve_pdf_for_target(target)
        if not manual_dir or not pdf_path or not pdf_path.exists():
            sys.stderr.write(f"Error: Could not resolve valid PDF file for target '{target}'\n")
            all_ok = False
            continue

        ok = process_manual(
            pdf_path=pdf_path,
            markdown_arg=args.markdown,
            layout_arg=args.layout,
            out_arg=args.out,
            pages=args.pages,
            file_arg=args.file,
            dpi=args.dpi,
            force=args.force,
        )
        if not ok:
            all_ok = False

    if not all_ok:
        sys.exit(1)


if __name__ == "__main__":
    main()

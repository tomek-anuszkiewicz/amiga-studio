"""Stage 6: Prepare Inference Queue.

Scans Stage 5 layout output (page_XXXX.json, page_XXXX.png) and/or source PDF,
and generates or updates a queue JSON in the manual's root directory next to the PDF.
Tracks the inference status ('pending' vs 'completed') of each page file-by-file.

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-stage6-prepare-queue/scripts/stage6_prepare_queue.py "path/to/manual_ocr.pdf"
    python .agents/plugins/pdf-pipeline/skills/pdf-stage6-prepare-queue/scripts/stage6_prepare_queue.py "path/to/manual_ocr.pdf" --pages 1-50
    python .agents/plugins/pdf-pipeline/skills/pdf-stage6-prepare-queue/scripts/stage6_prepare_queue.py "path/to/manual_ocr.pdf" --status
    python .agents/plugins/pdf-pipeline/skills/pdf-stage6-prepare-queue/scripts/stage6_prepare_queue.py "path/to/manual_ocr.pdf" --update-page 5 --set-status completed
"""

from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, List, Optional, Set

import argparse
import json
import re
import sys

# Ensure safe console output on Windows
if sys.stdout.encoding and sys.stdout.encoding.lower() != "utf-8":
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass


def parse_page_range(range_str: str) -> Set[int]:
    """Parse a page range expression like '1-10', '3,5,7', '1-5,8,11-15'."""
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


def get_pdf_page_count(pdf_path: Path) -> Optional[int]:
    """Get total page count from PDF using PyMuPDF if available."""
    try:
        import pymupdf as fitz  # type: ignore
        with fitz.open(pdf_path) as doc:
            return len(doc)
    except Exception:
        return None


def resolve_manual_paths(pdf_input: str, layout_input: str) -> tuple[Path, Path, Path]:
    """Resolve PDF path, manual directory, and layout directory using relative paths."""
    p = Path(pdf_input)
    if not p.is_absolute():
        p = (Path.cwd() / p).resolve()

    if p.is_dir():
        manual_dir = p.parent if p.name == "build" else p
        pdf_candidate = None
        build_dir = manual_dir / "build"
        if build_dir.is_dir():
            for f in build_dir.glob("*.pdf"):
                if f.name.endswith("_ocr.pdf"):
                    pdf_candidate = f
                    break
            if not pdf_candidate:
                for f in build_dir.glob("*.pdf"):
                    if not f.name.endswith("_ocr.pdf"):
                        if pdf_candidate is None or len(f.name) < len(pdf_candidate.name):
                            pdf_candidate = f
        if not pdf_candidate:
            for f in manual_dir.glob("*.pdf"):
                if f.name.endswith("_ocr.pdf"):
                    pdf_candidate = f
                    break
                else:
                    if pdf_candidate is None or len(f.name) < len(pdf_candidate.name):
                        pdf_candidate = f
        pdf_path = pdf_candidate or (build_dir / f"{manual_dir.name}.pdf")
    else:
        pdf_path = p
        manual_dir = pdf_path.parent.parent if pdf_path.parent.name == "build" else pdf_path.parent

    layout_path = Path(layout_input)
    if not layout_path.is_absolute():
        if (manual_dir / layout_path).exists():
            layout_dir = (manual_dir / layout_path).resolve()
        elif (Path.cwd() / layout_path).exists():
            layout_dir = (Path.cwd() / layout_path).resolve()
        else:
            layout_dir = (manual_dir / layout_path).resolve()
    else:
        layout_dir = layout_path

    return pdf_path, manual_dir, layout_dir


def build_or_update_queue(
    pdf_path: Path,
    manual_dir: Path,
    layout_dir: Path,
    output_path: Path,
    pages_filter: Optional[Set[int]] = None,
    reset: bool = False,
) -> Dict[str, Any]:
    """Build or refresh the queue JSON tracking file-by-file progress."""
    existing_data: Optional[Dict[str, Any]] = None
    existing_status_map: Dict[int, str] = {}

    if output_path.exists() and not reset:
        try:
            with open(output_path, "r", encoding="utf-8") as f:
                existing_data = json.load(f)
                for item in existing_data.get("queue", []):
                    p_num = item.get("page_number")
                    if p_num is not None:
                        st = item.get("status", "pending")
                        if st == "done":
                            st = "completed"
                        existing_status_map[p_num] = st
        except Exception as e:
            sys.stderr.write(f"Warning: Failed to load existing queue file ({e}). Starting fresh.\n")

    # Determine total pages
    pdf_pages = get_pdf_page_count(pdf_path) if pdf_path.exists() else None

    # Discover available layout pages in layout_dir
    discovered_page_nums: Set[int] = set()
    if layout_dir.exists():
        for p in layout_dir.glob("page_*.json"):
            m = re.match(r"^page_(\d+)\.json$", p.name)
            if m:
                discovered_page_nums.add(int(m.group(1)))

    if pdf_pages is not None:
        all_pages = set(range(1, pdf_pages + 1))
    elif discovered_page_nums:
        all_pages = discovered_page_nums
    else:
        all_pages = set()

    if pages_filter:
        target_pages = sorted(list(all_pages.intersection(pages_filter)))
    else:
        target_pages = sorted(list(all_pages))

    # Construct queue items
    queue_items: List[Dict[str, Any]] = []
    completed_count = 0
    pending_count = 0

    # Determine layout_dir relative to manual_dir for clean portable paths
    try:
        layout_rel_str = str(layout_dir.relative_to(manual_dir)).replace("\\", "/")
    except ValueError:
        try:
            layout_rel_str = str(layout_dir.relative_to(Path.cwd())).replace("\\", "/")
        except ValueError:
            layout_rel_str = str(layout_dir).replace("\\", "/")

    for p_num in target_pages:
        json_rel = f"{layout_rel_str}/page_{p_num:04d}.json"
        png_rel = f"{layout_rel_str}/page_{p_num:04d}.png"
        md_rel = f"{layout_rel_str}/page_{p_num:04d}.md"

        md_file = layout_dir / f"page_{p_num:04d}.md"
        md_exists_and_ready = md_file.exists() and md_file.stat().st_size > 0

        # Status resolution
        if reset:
            status = "completed" if md_exists_and_ready else "pending"
        elif p_num in existing_status_map:
            # If already marked completed, keep completed. If pending but md was created, mark completed.
            if existing_status_map[p_num] in ("completed", "done"):
                status = "completed"
            elif md_exists_and_ready:
                status = "completed"
            else:
                status = existing_status_map[p_num]
        else:
            status = "completed" if md_exists_and_ready else "pending"

        if status == "completed":
            completed_count += 1
        else:
            pending_count += 1

        queue_items.append({
            "page_number": p_num,
            "json_file": json_rel,
            "png_file": png_rel,
            "md_file": md_rel,
            "status": status,
        })

    queue_data = {
        "pdf_file": pdf_path.name if pdf_path.exists() else str(pdf_path),
        "total_pages": len(target_pages),
        "layout_dir": layout_rel_str,
        "stats": {
            "total": len(target_pages),
            "completed": completed_count,
            "pending": pending_count,
        },
        "queue": queue_items,
    }

    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(queue_data, f, indent=2, ensure_ascii=False)

    return queue_data


def update_pages_status(queue_path: Path, page_nums: Set[int], new_status: str) -> bool:
    """Update status of multiple pages in the queue file atomically with disk validation."""
    if not queue_path.exists():
        sys.stderr.write(f"Error: Queue file not found: {queue_path}\n")
        return False

    with open(queue_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    # When marking pages completed, verify markdown existence and non-zero size on disk
    if new_status == "completed":
        manual_dir = queue_path.parent.parent if queue_path.parent.name == "build" else queue_path.parent
        layout_rel = data.get("layout_dir", "build/01_page_layout")
        layout_dir = (manual_dir / layout_rel).resolve()

        missing_or_empty = []
        for p_num in page_nums:
            md_file = layout_dir / f"page_{p_num:04d}.md"
            if not md_file.exists() or md_file.stat().st_size == 0:
                missing_or_empty.append(f"page_{p_num:04d}.md")

        if missing_or_empty:
            sys.stderr.write(
                f"[STAGE 6 ERROR] Cannot mark completed. Missing or empty markdown on disk ({len(missing_or_empty)} file(s)):\n"
                f"  {missing_or_empty}\n"
            )
            return False

    updated_count = 0
    completed = 0
    pending = 0

    for item in data.get("queue", []):
        p_num = item.get("page_number")
        if p_num in page_nums:
            item["status"] = new_status
            updated_count += 1
        if item.get("status") == "completed":
            completed += 1
        else:
            pending += 1

    if updated_count == 0:
        sys.stderr.write(f"Warning: None of the pages {sorted(list(page_nums))} found in queue file.\n")

    data["stats"] = {
        "total": len(data.get("queue", [])),
        "completed": completed,
        "pending": pending,
    }

    with open(queue_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)

    print(f"[STAGE 6] Verified disk markdown and updated {updated_count} page(s) to '{new_status}'.")
    print(f"[STAGE 6 STATS] Completed: {completed}/{len(data.get('queue', []))} | Pending: {pending}")
    return True


def update_single_page_status(queue_path: Path, page_num: int, new_status: str) -> bool:
    """Update status of a specific page inside an existing queue file."""
    return update_pages_status(queue_path, {page_num}, new_status)


def display_status(queue_path: Path) -> None:
    """Display current queue progress and next pending items."""
    if not queue_path.exists():
        sys.stderr.write(f"Error: Queue file not found: {queue_path}\n")
        sys.exit(1)

    with open(queue_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    stats = data.get("stats", {})
    total = stats.get("total", len(data.get("queue", [])))
    completed = stats.get("completed", 0)
    pending = stats.get("pending", 0)
    pct = (completed / total * 100.0) if total > 0 else 0.0

    print(f"=== Queue Status: {queue_path.name} ===")
    print(f"PDF File:     {data.get('pdf_file')}")
    print(f"Layout Dir:   {data.get('layout_dir')}")
    print(f"Progress:     {completed}/{total} pages completed ({pct:.1f}%)")
    print(f"Pending:      {pending} pages remaining")

    pending_pages = [item["page_number"] for item in data.get("queue", []) if item.get("status") != "completed"]
    if pending_pages:
        next_batch = pending_pages[:10]
        preview_str = ", ".join(str(p) for p in next_batch)
        if len(pending_pages) > 10:
            preview_str += f" ... (+{len(pending_pages) - 10} more)"
        print(f"Next pending: Pages [{preview_str}]")
    else:
        print("All pages completed!")


def main() -> None:
    parser = argparse.ArgumentParser(description="Stage 6: Prepare Inference Queue.")
    parser.add_argument("pdf", help="Path to input PDF file")
    parser.add_argument(
        "--layout",
        "-l",
        default="build/01_page_layout",
        help="Stage 5 layout directory (default: build/01_page_layout relative to PDF)",
    )
    parser.add_argument(
        "--output",
        "-o",
        help="Optional custom output path for queue JSON (default: <pdf_dir>/<pdf_stem>_queue.json)",
    )
    parser.add_argument(
        "--pages",
        "-p",
        help="Optional page range to filter queue (e.g. '1-50', '4-15')",
    )
    parser.add_argument(
        "--reset",
        action="store_true",
        help="Reset statuses based strictly on current file existence on disk",
    )
    parser.add_argument(
        "--status",
        action="store_true",
        help="Display summary status of the queue without modifying it",
    )
    parser.add_argument(
        "--update-page",
        type=int,
        help="Update the status of a specific page number",
    )
    parser.add_argument(
        "--update-pages",
        help="Update the status of multiple pages (e.g. '1,2,3', '1-5')",
    )
    parser.add_argument(
        "--set-status",
        choices=["pending", "completed", "failed"],
        default="completed",
        help="Status value to set with --update-page or --update-pages (default: completed)",
    )

    args = parser.parse_args()

    pdf_path, manual_dir, layout_dir = resolve_manual_paths(args.pdf, args.layout)

    if args.output:
        out_file = Path(args.output)
    else:
        build_dir = manual_dir / "build"
        cand_out = build_dir / f"{pdf_path.stem}_queue.json"
        if not cand_out.exists() and (manual_dir / f"{pdf_path.stem}_queue.json").exists():
            out_file = manual_dir / f"{pdf_path.stem}_queue.json"
        else:
            out_file = cand_out
    if not out_file.is_absolute():
        out_file = (manual_dir / out_file).resolve()

    if args.status:
        display_status(out_file)
        return

    if args.update_pages is not None:
        target_set = parse_page_range(args.update_pages)
        success = update_pages_status(out_file, target_set, args.set_status)
        if not success:
            sys.exit(1)
        return

    if args.update_page is not None:
        success = update_single_page_status(out_file, args.update_page, args.set_status)
        if not success:
            sys.exit(1)
        return

    pages_filter = parse_page_range(args.pages) if args.pages else None

    print(f"[STAGE 6] Scanning '{pdf_path.name}' and layout in '{layout_dir.name}'...")
    queue_data = build_or_update_queue(
        pdf_path=pdf_path,
        manual_dir=manual_dir,
        layout_dir=layout_dir,
        output_path=out_file,
        pages_filter=pages_filter,
        reset=args.reset,
    )

    stats = queue_data["stats"]
    pct = (stats["completed"] / stats["total"] * 100.0) if stats["total"] > 0 else 0.0

    # Display relative path for privacy
    try:
        display_out = str(out_file.relative_to(Path.cwd())).replace("\\", "/")
    except ValueError:
        display_out = out_file.name

    print(f"[STAGE 6 SUCCESS] Queue file generated: {display_out}")
    print(f"[STAGE 6 STATS] Total: {stats['total']} | Completed: {stats['completed']} ({pct:.1f}%) | Pending: {stats['pending']}")


if __name__ == "__main__":
    main()

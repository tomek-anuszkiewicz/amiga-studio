"""Stage 11: Prepare Conversion Queue & Assets Schema.

Initializes and upgrades <manual_dir>/<stem>_assets_queue.json for downstream conversion
(Stages 12, 14), reduction (Stage 13), and embedding (Stage 16).

Key operations:
1. Initializes per-asset conversion metadata non-destructively:
   - conversion_status: 'pending' | 'completed' (auto-detected if file exists on disk)
   - conversion_stage: int | None (12 for Stage 12 HTML tables, 14 for Stage 14 images)
   - generated_markdown: relative path to the active .md fragment for embedding
   - generated_markdown_html: relative path preserving original HTML table (for table_html)
   - generated_txt: relative path to RAG plain text breakdown (for image)
   - reduced_to_markdown: boolean tracking Stage 13 reduction (for table_html)
   - reduction_reason: string explaining reduction decision
   - is_embedded: boolean tracking Stage 16 embedding into page_XXXX-embed.md
   - embedded_file: relative path to page_XXXX-embed.md
2. Maintains aggregate conversion, reduction, and embedding statistics.
3. Generates a clear status report via --status.

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-stage11-prepare-convert/scripts/stage11_prepare_convert.py "path/to/manual_dir"
    python .agents/plugins/pdf-pipeline/skills/pdf-stage11-prepare-convert/scripts/stage11_prepare_convert.py "path/to/manual_dir" --status
    python .agents/plugins/pdf-pipeline/skills/pdf-stage11-prepare-convert/scripts/stage11_prepare_convert.py "path/to/manual_dir" --reset
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

# Safe console output on Windows
if sys.stdout.encoding and sys.stdout.encoding.lower() != "utf-8":
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass


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


def resolve_manual_paths(target: Path) -> Tuple[Path, Optional[Path], Path, Path]:
    """Resolve directory, pdf, layout directory, and queue path."""
    target = target.resolve()
    if target.is_file() and target.suffix.lower() == ".pdf":
        manual_dir = target.parent.parent if target.parent.name == "build" else target.parent
        pdf_path = target
    elif target.is_dir():
        manual_dir = target.parent if target.name == "build" else target
        pdf_files = []
        build_dir = manual_dir / "build"
        if build_dir.is_dir():
            pdf_files.extend(list(build_dir.glob("*.pdf")))
        pdf_files.extend(list(manual_dir.glob("*.pdf")))
        ocr_candidates = [p for p in pdf_files if "_ocr" in p.name]
        pdf_path = ocr_candidates[0] if ocr_candidates else (pdf_files[0] if pdf_files else None)
    else:
        manual_dir = target.parent.parent if target.parent.name == "build" else target.parent
        pdf_path = None

    layout_dir = manual_dir / "build" / "01_page_layout"
    stem = pdf_path.stem if pdf_path else manual_dir.name
    build_dir = manual_dir / "build"
    candidate_q = build_dir / f"{stem}_assets_queue.json"
    if candidate_q.exists():
        queue_path = candidate_q
    elif (manual_dir / f"{stem}_assets_queue.json").exists():
        queue_path = manual_dir / f"{stem}_assets_queue.json"
    else:
        found_q = None
        if build_dir.is_dir():
            for q in build_dir.glob("*_assets_queue.json"):
                found_q = q
                break
        if not found_q:
            for q in manual_dir.glob("*_assets_queue.json"):
                found_q = q
                break
        queue_path = found_q or candidate_q

    return manual_dir, pdf_path, layout_dir, queue_path


def load_queue(queue_path: Path) -> Dict[str, Any]:
    """Load and parse the assets queue JSON."""
    if not queue_path.exists():
        sys.stderr.write(f"Error: Queue file not found: {queue_path}\n")
        sys.exit(1)
    with open(queue_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        data = {"queue": data, "stats": {}}
    return data


def save_queue(queue_path: Path, data: Dict[str, Any]) -> None:
    """Save the updated queue JSON with cleanly formatted indentation."""
    temp_path = queue_path.with_suffix(".tmp")
    with open(temp_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
    temp_path.replace(queue_path)


def refresh_statistics(data: Dict[str, Any], manual_dir: Path) -> None:
    """Recalculate conversion, reduction, comparison, and embedding statistics."""
    items = data.get("queue", [])
    layout_dir = manual_dir / "build" / "01_page_layout"

    conv_stats = {
        "table_markdown": {"total": 0, "converted": 0, "pending": 0},
        "table_html": {"total": 0, "converted": 0, "pending": 0},
        "image": {"total": 0, "converted": 0, "pending": 0},
        "by_stage": {
            "stage12_tables": 0,
            "stage14_images": 0,
            "pending": 0,
        },
    }
    reduc_stats = {
        "total_html_tables": 0,
        "reduced_to_markdown": 0,
        "kept_as_html": 0,
        "pending": 0,
    }
    embed_stats = {
        "total_assets": len(items),
        "embedded_assets": 0,
        "pending_assets": 0,
        "total_pages": 0,
        "embedded_pages": 0,
    }

    all_pages: Set[int] = set()
    embedded_pages: Set[int] = set()

    for it in items:
        dtype = it.get("detected_type")
        page_num = it.get("page_number", 0)
        all_pages.add(page_num)

        # Track conversion stage breakdown
        c_stage = it.get("conversion_stage")
        if c_stage == 12 or (c_stage == 13 and dtype == "table_html"):
            conv_stats["by_stage"]["stage12_tables"] += 1
        elif c_stage == 14 or (c_stage in (15, 16) and dtype == "image"):
            conv_stats["by_stage"]["stage14_images"] += 1
        else:
            conv_stats["by_stage"]["pending"] += 1

        # 1. Conversion stats
        if dtype in conv_stats:
            conv_stats[dtype]["total"] += 1
            if it.get("conversion_status") in ("completed", "converted"):
                conv_stats[dtype]["converted"] += 1
            else:
                conv_stats[dtype]["pending"] += 1

        # 2. Reduction stats
        if dtype == "table_html":
            reduc_stats["total_html_tables"] += 1
            if it.get("reduced_to_markdown") is True:
                reduc_stats["reduced_to_markdown"] += 1
            elif it.get("reduced_to_markdown") is False:
                reduc_stats["kept_as_html"] += 1
            else:
                reduc_stats["pending"] += 1

        # 3. Embed stats
        if it.get("is_embedded") is True:
            embed_stats["embedded_assets"] += 1
        else:
            embed_stats["pending_assets"] += 1

    # Check embedded pages on disk
    for p in all_pages:
        embed_md = layout_dir / f"page_{p:04d}-embed.md"
        if embed_md.exists() and embed_md.stat().st_size > 0:
            embedded_pages.add(p)

    embed_stats["total_pages"] = len(all_pages)
    embed_stats["embedded_pages"] = len(embedded_pages)

    unclass_cnt = sum(1 for it in items if not it.get("detected_type"))
    conv_stats["unclassified"] = unclass_cnt

    data["conversion_stats"] = conv_stats
    data["reduction_stats"] = reduc_stats
    data["embed_stats"] = embed_stats


def upgrade_queue_schema(
    queue_path: Path,
    manual_dir: Path,
    reset: bool = False,
) -> Tuple[int, int]:
    """Upgrade assets queue schema with conversion and embedding fields."""
    data = load_queue(queue_path)
    items = data.get("queue", [])
    layout_dir = manual_dir / "build" / "01_page_layout"
    assets_dir = layout_dir / "assets"

    # Ensure required directories exist
    assets_dir.mkdir(parents=True, exist_ok=True)

    upgraded_count = 0
    already_valid_count = 0

    for it in items:
        asset_id = it.get("asset_id")
        page_num = it.get("page_number", 0)
        dtype = it.get("detected_type")

        # Destination paths relative to manual_dir
        rel_embed_md = to_relative_posix(layout_dir / f"page_{page_num:04d}-embed.md", manual_dir)

        if dtype == "table_html":
            rel_gen_md = to_relative_posix(assets_dir / f"{asset_id}_html.md", manual_dir)
            rel_gen_html = to_relative_posix(assets_dir / f"{asset_id}_html.md", manual_dir)
            rel_gen_txt = None
            
            # Check if reduced markdown exists on disk
            rel_reduced_md = to_relative_posix(assets_dir / f"{asset_id}_reduced.md", manual_dir)
            reduced_exists = (manual_dir / rel_reduced_md).exists() and (manual_dir / rel_reduced_md).stat().st_size > 0
            if reduced_exists:
                rel_gen_md = rel_reduced_md
                default_reduced = True
            else:
                default_reduced = None
        elif dtype == "image":
            rel_gen_md = to_relative_posix(assets_dir / f"{asset_id}.md", manual_dir)
            rel_gen_html = None
            rel_gen_txt = to_relative_posix(assets_dir / f"{asset_id}.txt", manual_dir)
            default_reduced = None
            reduced_exists = False
        else:
            rel_gen_md = to_relative_posix(assets_dir / f"{asset_id}.md", manual_dir)
            rel_gen_html = None
            rel_gen_txt = None
            default_reduced = None
            reduced_exists = False

        # Check existing disk files for conversion status
        full_gen_md = manual_dir / rel_gen_md if rel_gen_md else None
        md_exists = full_gen_md.exists() and full_gen_md.stat().st_size > 0 if full_gen_md else False

        full_gen_txt = manual_dir / rel_gen_txt if rel_gen_txt else None
        txt_exists = full_gen_txt.exists() and full_gen_txt.stat().st_size > 0 if full_gen_txt else False

        if dtype == "image":
            is_converted = md_exists and txt_exists
        else:
            is_converted = md_exists

        # Check existing disk files for embed status
        full_embed = manual_dir / rel_embed_md
        is_embedded = False
        if full_embed.exists() and full_embed.stat().st_size > 0:
            try:
                content = full_embed.read_text(encoding="utf-8", errors="replace")
                if asset_id in content:
                    is_embedded = True
            except Exception:
                pass

        needs_update = False

        if reset:
            it["conversion_status"] = "completed" if is_converted else "pending"
            it["conversion_stage"] = it.get("conversion_stage") if is_converted else None
            it["generated_markdown"] = rel_gen_md
            it["generated_markdown_html"] = rel_gen_html
            it["generated_txt"] = rel_gen_txt
            it["reduced_to_markdown"] = default_reduced
            it["reduction_reason"] = "flat_grid_clean_markdown" if default_reduced else None
            it["is_embedded"] = is_embedded
            it["embedded_file"] = rel_embed_md
            needs_update = True
        else:
            if "conversion_status" not in it:
                it["conversion_status"] = "completed" if is_converted else "pending"
                needs_update = True
            elif it["conversion_status"] == "pending" and is_converted:
                it["conversion_status"] = "completed"
                needs_update = True

            if "conversion_stage" not in it:
                if dtype == "image" and is_converted:
                    it["conversion_stage"] = 14
                elif dtype == "table_html" and is_converted:
                    it["conversion_stage"] = 12
                else:
                    it["conversion_stage"] = None
                needs_update = True
            elif it.get("conversion_stage") == 13:
                it["conversion_stage"] = 12
                needs_update = True
            elif it.get("conversion_stage") in (15, 16):
                it["conversion_stage"] = 14
                needs_update = True

            if "generated_markdown" not in it:
                it["generated_markdown"] = rel_gen_md
                needs_update = True
            elif reduced_exists and not it.get("generated_markdown", "").endswith("_reduced.md"):
                it["generated_markdown"] = rel_gen_md
                needs_update = True

            if "generated_markdown_html" not in it:
                it["generated_markdown_html"] = rel_gen_html
                needs_update = True

            if "generated_txt" not in it:
                it["generated_txt"] = rel_gen_txt
                needs_update = True

            if "reduced_to_markdown" not in it:
                it["reduced_to_markdown"] = default_reduced
                needs_update = True
            elif reduced_exists and it.get("reduced_to_markdown") is not True:
                it["reduced_to_markdown"] = True
                needs_update = True

            if "reduction_reason" not in it:
                it["reduction_reason"] = "flat_grid_clean_markdown" if reduced_exists else None
                needs_update = True
            elif reduced_exists and not it.get("reduction_reason"):
                it["reduction_reason"] = "flat_grid_clean_markdown"
                needs_update = True

            if "is_embedded" not in it:
                it["is_embedded"] = is_embedded
                needs_update = True

            if "embedded_file" not in it:
                it["embedded_file"] = rel_embed_md
                needs_update = True

        if needs_update:
            upgraded_count += 1
        else:
            already_valid_count += 1

    refresh_statistics(data, manual_dir)
    save_queue(queue_path, data)
    return upgraded_count, already_valid_count


def print_status_dashboard(queue_path: Path, manual_dir: Path) -> None:
    """Print comprehensive pipeline status for Stages 11 through 18."""
    data = load_queue(queue_path)
    refresh_statistics(data, manual_dir)

    items = data.get("queue", [])
    total_assets = data.get("total_assets", len(items))
    stats = data.get("stats", {})
    conv = data.get("conversion_stats", {})
    reduc = data.get("reduction_stats", {})
    embed = data.get("embed_stats", {})

    print("=" * 80)
    print(f"PIPELINE ASSET CONVERSION & FINAL EMBED DASHBOARD (Stages 12-16)")
    print(f"Manual Directory: {manual_dir.name}")
    print(f"Queue File:       {queue_path.name}")
    print(f"Total Assets:     {total_assets}")
    print("=" * 80)

    # 1. Classification & Conversion Status (Stage 12)
    print("\n[Stage 12: Asset Classification & Table Conversion Status]")
    print(f"  - HTML Tables     (table_html):     {stats.get('table_html', 0):>4}")
    print(f"  - Images / Figs   (image):          {stats.get('image', 0):>4}")
    unclassified = sum(1 for it in items if not it.get("detected_type"))
    print(f"  - Awaiting Classification/Conversion: {unclassified:>4}")

    # 2. Conversion Status (Stages 12, 14)
    print("\n[Stages 12, 14: Multimodal Asset Conversion Status]")
    html_conv = conv.get("table_html", {})
    img_conv = conv.get("image", {})
    by_stage = conv.get("by_stage", {})

    print(f"  - HTML Tables    (Stage 12): Converted: {by_stage.get('stage12_tables', 0):>4}")
    print(f"  - Images (MD+RAG)(Stage 14): Converted: {by_stage.get('stage14_images', 0):>4} / {img_conv.get('total', 0):<4} (Pending: {img_conv.get('pending', 0)})")

    total_converted = html_conv.get("converted", 0) + img_conv.get("converted", 0)
    total_pending = html_conv.get("pending", 0) + img_conv.get("pending", 0)
    print(f"  -> Total Converted: {total_converted:>4} / {total_assets} ({total_pending} remaining)")

    # 3. Reduction Status (Stage 13)
    print("\n[Stage 13: HTML-to-Markdown Table Reduction Status]")
    print(f"  - Total HTML Candidates: {reduc.get('total_html_tables', 0):>4}")
    print(f"  - Reduced to Markdown:   {reduc.get('reduced_to_markdown', 0):>4}")
    print(f"  - Kept as HTML Table:    {reduc.get('kept_as_html', 0):>4}")
    print(f"  - Pending Evaluation:    {reduc.get('pending', 0):>4}")

    # 4. Embedding Status (Stage 16)
    print("\n[Stage 16: Markdown Embedding Status (page_XXXX-embed.md)]")
    print(f"  - Assets Embedded: {embed.get('embedded_assets', 0):>4} / {total_assets}")
    print(f"  - Pages Embedded:  {embed.get('embedded_pages', 0):>4} / {embed.get('total_pages', 0)}")

    print("=" * 80)


def update_stage_assets(
    queue_path: Path,
    manual_dir: Path,
    stage: int,
    completed: Optional[List[str]] = None,
    classified_images: Optional[List[str]] = None,
    reduced: Optional[List[str]] = None,
) -> int:
    """Deterministically update asset queue records for Stages 12-14."""
    data = load_queue(queue_path)
    items = data.get("queue", [])
    item_map = {it.get("asset_id"): it for it in items if it.get("asset_id")}
    layout_dir = manual_dir / "build" / "01_page_layout"
    assets_dir = layout_dir / "assets"
    updated_count = 0

    if stage == 12:
        if completed:
            missing = [aid for aid in completed if not (assets_dir / f"{aid}_html.md").exists() or (assets_dir / f"{aid}_html.md").stat().st_size == 0]
            if missing:
                sys.stderr.write(f"[STAGE 11 ERROR] Missing or empty Stage 12 table HTML artifact(s) on disk ({len(missing)} file(s)):\n  {missing}\n")
                return -1
            for aid in completed:
                if aid in item_map:
                    it = item_map[aid]
                    it["detected_type"] = "table_html"
                    it["status"] = "checked"
                    it["conversion_status"] = "completed"
                    it["conversion_stage"] = 12
                    it["generated_markdown"] = to_relative_posix(assets_dir / f"{aid}_html.md", manual_dir)
                    it["generated_markdown_html"] = to_relative_posix(assets_dir / f"{aid}_html.md", manual_dir)
                    updated_count += 1
        if classified_images:
            for aid in classified_images:
                if aid in item_map:
                    it = item_map[aid]
                    it["detected_type"] = "image"
                    it["status"] = "checked"
                    it["conversion_status"] = "pending"
                    it["conversion_stage"] = None
                    updated_count += 1

    elif stage == 13 and reduced:
        missing = [aid for aid in reduced if not (assets_dir / f"{aid}_reduced.md").exists() or (assets_dir / f"{aid}_reduced.md").stat().st_size == 0]
        if missing:
            sys.stderr.write(f"[STAGE 11 ERROR] Missing or empty Stage 13 reduced table artifact(s) on disk ({len(missing)} file(s)):\n  {missing}\n")
            return -1
        for aid in reduced:
            if aid in item_map:
                it = item_map[aid]
                it["reduced_to_markdown"] = True
                it["reduction_reason"] = "flat_grid_clean_markdown"
                it["generated_markdown"] = to_relative_posix(assets_dir / f"{aid}_reduced.md", manual_dir)
                updated_count += 1

    elif stage == 14 and completed:
        missing = []
        for aid in completed:
            md_f = assets_dir / f"{aid}.md"
            txt_f = assets_dir / f"{aid}.txt"
            if not md_f.exists() or md_f.stat().st_size == 0 or not txt_f.exists() or txt_f.stat().st_size == 0:
                missing.append(aid)
        if missing:
            sys.stderr.write(f"[STAGE 11 ERROR] Missing or empty Stage 14 image markdown/txt artifact(s) on disk ({len(missing)} file(s)):\n  {missing}\n")
            return -1
        for aid in completed:
            if aid in item_map:
                it = item_map[aid]
                it["detected_type"] = "image"
                it["status"] = "checked"
                it["conversion_status"] = "completed"
                it["conversion_stage"] = 14
                it["generated_markdown"] = to_relative_posix(assets_dir / f"{aid}.md", manual_dir)
                it["generated_txt"] = to_relative_posix(assets_dir / f"{aid}.txt", manual_dir)
                updated_count += 1

    refresh_statistics(data, manual_dir)
    save_queue(queue_path, data)
    return updated_count


def main() -> None:
    parser = argparse.ArgumentParser(description="Stage 11: Prepare Conversion Queue & Asset Schema.")
    parser.add_argument("manual", nargs="?", default=".", help="Path to manual directory or PDF file.")
    parser.add_argument("--status", action="store_true", help="Print asset conversion and embedding status dashboard.")
    parser.add_argument("--reset", action="store_true", help="Reset all conversion and embedding tracking fields.")
    parser.add_argument("--update-stage", type=int, choices=[12, 13, 14], help="Target conversion stage to record")
    parser.add_argument("--completed", nargs="+", help="Asset IDs converted in Stage 12 or 14")
    parser.add_argument("--classified-images", nargs="+", help="Image asset IDs classified in Stage 12")
    parser.add_argument("--reduced", nargs="+", help="Table asset IDs reduced to GFM markdown in Stage 13")
    parser.add_argument("--sync-disk", action="store_true", help="Reconcile queue status with existing files on disk")

    args = parser.parse_args()
    manual_dir, pdf_path, layout_dir, queue_path = resolve_manual_paths(Path(args.manual))

    if args.status:
        print_status_dashboard(queue_path, manual_dir)
        return

    if args.sync_disk:
        upgraded, valid = upgrade_queue_schema(queue_path, manual_dir, reset=False)
        print(f"Stage 11 synced disk artifacts for '{manual_dir.name}': {upgraded} assets updated, {valid} already valid.")
        print_status_dashboard(queue_path, manual_dir)
        return

    if args.update_stage is not None:
        updated = update_stage_assets(
            queue_path=queue_path,
            manual_dir=manual_dir,
            stage=args.update_stage,
            completed=args.completed,
            classified_images=args.classified_images,
            reduced=args.reduced,
        )
        if updated < 0:
            sys.exit(1)
        print(f"[STAGE 11] Verified disk artifacts and updated {updated} asset(s) for Stage {args.update_stage} in '{manual_dir.name}'.")
        return

    upgraded, valid = upgrade_queue_schema(queue_path, manual_dir, reset=args.reset)
    print(f"Stage 11 complete for '{manual_dir.name}': {upgraded} assets updated/initialized, {valid} already valid.")
    print_status_dashboard(queue_path, manual_dir)


if __name__ == "__main__":
    main()

"""Stage Reset & Pipeline Cleanup Tool.

Prepares specified pages for re-running pipeline stages by cleanly removing
downstream artifacts and unwinding queue statuses back to pending.

Supports Stages 7 through 19.
Usage outside this range is explicitly rejected with an error.

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "Hardware Reference Manual" --from-stage 7 --to-stage 19 --pages 73
    python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "Hardware Reference Manual" 7 19 --all
    python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "Hardware Reference Manual" 11 19 --pages 70-80
    python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py 18 19 --all
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

# Import planner from sibling pdf-pipeline skill
script_dir = Path(__file__).resolve().parent
pdf_pipeline_scripts = script_dir.parent.parent / "pdf-pipeline" / "scripts"
if pdf_pipeline_scripts.is_dir() and str(pdf_pipeline_scripts) not in sys.path:
    sys.path.insert(0, str(pdf_pipeline_scripts))

try:
    from planner import generate_execution_plan
except ImportError:
    generate_execution_plan = None

SUPPORTED_MIN_STAGE = 7
SUPPORTED_MAX_STAGE = 19

TARGET_MANUALS: List[str] = [
    "68000 Programmer's Reference Manual",
    "68000 User's Manual",
    "A500 A2000 Technical Reference Manual",
    "Hardware Reference Manual",
    "Test Book example-4567",
]


def resolve_manual_targets(manual_input: Optional[str]) -> List[str]:
    """Resolve target manual list. If None or 'all', discover all existing target manuals."""
    if manual_input and manual_input.strip().lower() != "all":
        return [manual_input.strip()]

    # Discover which of the target manuals exist in cwd
    found = [m for m in TARGET_MANUALS if (Path.cwd() / m).is_dir()]
    if found:
        return found

    # Fallback to discovering any subdirectories containing .pdf files
    discovered = []
    for p in sorted(Path.cwd().iterdir()):
        if p.is_dir() and not p.name.startswith(".") and p.name != "build":
            if any(p.glob("*.pdf")):
                discovered.append(p.name)
    return discovered if discovered else TARGET_MANUALS

def parse_chapter_range(range_str: str) -> Set[int]:
    """Parse chapter range string like '1-3', '5', '1,3,5-7'."""
    chapters: Set[int] = set()
    for part in range_str.split(","):
        part = part.strip()
        if not part:
            continue
        if "-" in part:
            s_str, e_str = part.split("-", 1)
            chapters.update(range(int(s_str.strip()), int(e_str.strip()) + 1))
        else:
            chapters.add(int(part))
    return chapters

# Safe console output on Windows
if sys.stdout.encoding and sys.stdout.encoding.lower() != "utf-8":
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass


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


def resolve_all_pages(manual_dir: Path, layout_dir: Path, pdf_path: Optional[Path]) -> Set[int]:
    """Discover all pages from layout files or PDF page count."""
    pages: Set[int] = set()
    if layout_dir.exists():
        for p in layout_dir.glob("page_*.json"):
            m = re.match(r"^page_(\d+)\.json$", p.name)
            if m:
                pages.add(int(m.group(1)))
    if not pages and pdf_path and pdf_path.exists():
        try:
            import pymupdf as fitz
            with fitz.open(pdf_path) as doc:
                pages = set(range(1, len(doc) + 1))
        except Exception:
            pass
    return pages


def resolve_manual_paths(
    target_input: str,
) -> Tuple[Path, Optional[Path], Path, Optional[Path], Optional[Path], Optional[Path]]:
    """Resolve manual directory, source PDF, layout dir, Stage 6 queue, assets queue, and chapters map."""
    p = Path(target_input)
    if not p.is_absolute():
        p = (Path.cwd() / p).resolve()

    if p.is_file() and p.suffix.lower() == ".pdf":
        manual_dir = p.parent.parent if p.parent.name == "build" else p.parent
        pdf_path: Optional[Path] = p
    elif p.is_dir():
        manual_dir = p.parent if p.name == "build" else p
        pdf_path = None
    else:
        candidate = Path.cwd() / target_input
        if candidate.is_dir():
            manual_dir = candidate.parent.resolve() if candidate.name == "build" else candidate.resolve()
            pdf_path = None
        else:
            manual_dir = p
            pdf_path = None

    if pdf_path is None and manual_dir.exists():
        ocr_candidate = None
        std_candidate = None
        build_dir = manual_dir / "build"
        if build_dir.is_dir():
            for f in build_dir.glob("*.pdf"):
                if f.name.endswith("_ocr.pdf"):
                    ocr_candidate = f
                    break
                elif not f.name.endswith("_ocr.pdf"):
                    if std_candidate is None or len(f.name) < len(std_candidate.name):
                        std_candidate = f
        for f in manual_dir.glob("*.pdf"):
            if f.name.endswith("_ocr.pdf") and not ocr_candidate:
                ocr_candidate = f
                break
            elif not f.name.endswith("_ocr.pdf"):
                if std_candidate is None or len(f.name) < len(std_candidate.name):
                    std_candidate = f
        pdf_path = ocr_candidate or std_candidate

    layout_dir = manual_dir / "build" / "01_page_layout"
    stem = pdf_path.stem if pdf_path else manual_dir.name
    build_dir = manual_dir / "build"

    # Stage 6 queue (<stem>_queue.json)
    stage6_queue_path = build_dir / f"{stem}_queue.json"
    if not stage6_queue_path.exists():
        stage6_queue_path = manual_dir / f"{stem}_queue.json"
    if not stage6_queue_path.exists():
        found_q = None
        if build_dir.is_dir():
            for q in build_dir.glob("*_queue.json"):
                if not q.name.endswith("_assets_queue.json"):
                    found_q = q
                    break
        if not found_q:
            for q in manual_dir.glob("*_queue.json"):
                if not q.name.endswith("_assets_queue.json"):
                    found_q = q
                    break
        stage6_queue_path = found_q or stage6_queue_path

    # Stage 8..12 assets queue (<stem>_assets_queue.json)
    assets_queue_path = build_dir / f"{stem}_assets_queue.json"
    if not assets_queue_path.exists():
        assets_queue_path = manual_dir / f"{stem}_assets_queue.json"
    if not assets_queue_path.exists():
        found_aq = None
        if build_dir.is_dir():
            for aq in build_dir.glob("*_assets_queue.json"):
                found_aq = aq
                break
        if not found_aq:
            for aq in manual_dir.glob("*_assets_queue.json"):
                found_aq = aq
                break
        assets_queue_path = found_aq or assets_queue_path

    # Stage 4 chapters map (<stem>_chapters.json)
    chapters_path = build_dir / f"{stem}_chapters.json"
    if not chapters_path.exists():
        chapters_path = manual_dir / f"{stem}_chapters.json"
    if not chapters_path.exists():
        found_ch = None
        if build_dir.is_dir():
            for ch in build_dir.glob("*_chapters.json"):
                found_ch = ch
                break
        if not found_ch:
            for ch in manual_dir.glob("*_chapters.json"):
                found_ch = ch
                break
        chapters_path = found_ch or chapters_path

    return (
        manual_dir,
        pdf_path,
        layout_dir,
        stage6_queue_path if stage6_queue_path.exists() else None,
        assets_queue_path if assets_queue_path.exists() else None,
        chapters_path if chapters_path.exists() else None,
    )


def reset_stage6_queue(
    queue_path: Path,
    target_pages: Set[int],
) -> Tuple[int, Dict[str, Any]]:
    """Unwind Stage 6 inference queue items for target pages to 'pending'."""
    with open(queue_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    items = data.get("queue", [])
    reset_count = 0

    for item in items:
        p_num = item.get("page_number")
        if p_num in target_pages:
            if item.get("status") != "pending":
                item["status"] = "pending"
                reset_count += 1
        else:
            if item.get("status") == "done":
                item["status"] = "completed"

    completed = sum(1 for it in items if it.get("status") == "completed")
    pending = len(items) - completed

    clean_data = {
        "pdf_file": data.get("pdf_file", ""),
        "total_pages": len(items),
        "layout_dir": data.get("layout_dir", "build/01_page_layout"),
        "stats": {
            "total": len(items),
            "completed": completed,
            "pending": pending,
        },
        "queue": items,
    }

    with open(queue_path, "w", encoding="utf-8") as f:
        json.dump(clean_data, f, indent=2, ensure_ascii=False)

    return reset_count, clean_data["stats"]


def refresh_queue_statistics(
    data: Dict[str, Any],
    items: List[Dict[str, Any]],
    manual_dir: Path,
    layout_dir: Path,
) -> Dict[str, Any]:
    """Recalculate all queue statistics matching Stage 6, 9, and 12 invariants."""
    completed = sum(1 for it in items if it.get("final_asset") is not None or it.get("status") == "completed")
    checked = sum(1 for it in items if it.get("status") == "checked" or it.get("eval_clip") == "ok")
    pending = sum(1 for it in items if it.get("status") == "pending")

    by_status: Dict[str, int] = {}
    by_type: Dict[str, int] = {}
    by_eval_clip: Dict[str, int] = {}
    for it in items:
        st = it.get("status", "pending")
        by_status[st] = by_status.get(st, 0) + 1
        dt = it.get("detected_type")
        if dt:
            by_type[dt] = by_type.get(dt, 0) + 1
        ev = it.get("eval_clip")
        if ev:
            by_eval_clip[ev] = by_eval_clip.get(ev, 0) + 1

    stats = {
        "total": len(items),
        "completed": completed,
        "checked": checked,
        "pending": pending,
        "by_status": by_status,
        "by_type": by_type,
        "by_eval_clip": by_eval_clip,
    }

    # Downstream Stage 12+ statistics
    has_stage12 = any("conversion_status" in it for it in items) or "conversion_stats" in data
    if has_stage12:
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
        for it in items:
            dtype = it.get("detected_type")
            page_num = it.get("page_number", 0)
            if page_num:
                all_pages.add(page_num)

            # Track conversion stage breakdown
            c_stage = it.get("conversion_stage")
            if c_stage == 12:
                conv_stats["by_stage"]["stage12_tables"] += 1
            elif c_stage == 14:
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
        embedded_pages: Set[int] = set()
        for p in all_pages:
            embed_md = layout_dir / f"page_{p:04d}-embed.md"
            if embed_md.exists() and embed_md.stat().st_size > 0:
                embedded_pages.add(p)

        embed_stats["total_pages"] = len(all_pages)
        embed_stats["embedded_pages"] = len(embedded_pages)

        # Track unclassified count for Stage 12 tracking
        unclass_cnt = sum(1 for it in items if not it.get("detected_type"))
        conv_stats["unclassified"] = unclass_cnt

        data["conversion_stats"] = conv_stats
        data["reduction_stats"] = reduc_stats
        data["embed_stats"] = embed_stats

    return stats


def reset_assets_queue(
    queue_path: Path,
    target_pages: Set[int],
    from_stage: int,
    to_stage: int = 19,
    manual_dir: Optional[Path] = None,
    layout_dir: Optional[Path] = None,
) -> Tuple[int, int, Dict[str, Any]]:
    """Reset or prune items in `<stem>_assets_queue.json` for target pages.

    Returns:
        (pruned_count, modified_count, stats)
    """
    with open(queue_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    items = data.get("queue", []) if isinstance(data, dict) else data
    pruned_count = 0
    modified_count = 0

    if from_stage <= 8:
        # If Stage 8 (crops) or Stage 7 (markdown) is reset:
        # Prune all asset entries for the target pages entirely from the queue.
        new_items = []
        for it in items:
            p_num = it.get("page_number")
            if p_num in target_pages:
                pruned_count += 1
            else:
                new_items.append(it)
        items = new_items

        if len(items) == 0:
            try:
                queue_path.unlink()
            except Exception as e:
                sys.stderr.write(f"Warning: Could not remove empty assets queue {queue_path.name}: {e}\n")
            stats = {
                "total": 0,
                "completed": 0,
                "checked": 0,
                "pending": 0,
                "by_status": {},
                "by_type": {},
                "by_eval_clip": {},
            }
            return pruned_count, modified_count, stats

    else:
        # Keep base assets in queue, unwind status/fields according to stage range
        for it in items:
            p_num = it.get("page_number")
            if p_num not in target_pages:
                continue

            modified_count += 1
            is_full_page = (
                it.get("active_box") == [0, 0, 1000, 1000]
                or (it.get("versions") and it["versions"][0].get("source") == "stage8_full_page_asset")
            )
            dtype = it.get("detected_type")

            # -----------------------------------------------------------------
            # Stage 9: Revert recrops, clip_final, eval frames, verdicts
            # -----------------------------------------------------------------
            if from_stage <= 9:
                orig = it.get("original_asset") or it.get("asset")
                it["active_asset"] = orig
                it["asset"] = orig
                it["final_asset"] = None
                it["current_version"] = 0
                if it.get("versions"):
                    v0 = it["versions"][0]
                    v0["eval_result"] = "ok" if is_full_page else None
                    it["versions"] = [v0]
                    it["active_box"] = v0.get("box", it.get("active_box"))
                it["eval_frame"] = None
                it["eval_final_frame"] = None
                it["corrected_box"] = None
                it["defects_found"] = None
                it["margin_defect"] = False
                it["expanded_box"] = None
                it["action"] = None
                it["status"] = "eval_ok" if is_full_page else "pending"
                it["eval_clip"] = "ok" if is_full_page else None
                it["detected_type"] = None
                it["md_file"] = f"build/01_page_layout/page_{p_num:04d}-images.md"

            # -----------------------------------------------------------------
            # Stage 10: Revert evaluation verdicts, recrop actions, clip_final
            # -----------------------------------------------------------------
            elif from_stage == 10:
                if not is_full_page:
                    it["eval_clip"] = None
                    it["status"] = "pending"
                    it["corrected_box"] = None
                    it["defects_found"] = None
                    it["action"] = None
                it["final_asset"] = None
                it["eval_final_frame"] = None
                it["detected_type"] = None
                it["md_file"] = f"build/01_page_layout/page_{p_num:04d}-images.md"

            # -----------------------------------------------------------------
            # -----------------------------------------------------------------
            # Stage 11..19: Conversion, Reduction & Embedding
            # -----------------------------------------------------------------
            if from_stage <= 11 <= to_stage:
                it["conversion_status"] = "pending"
                it["conversion_stage"] = None
                it["generated_markdown"] = None
                it["generated_markdown_html"] = None
                it["generated_txt"] = None
                it["reduced_to_markdown"] = None
                it["reduction_reason"] = None
                it["is_embedded"] = False
                it["embedded_file"] = None

            # Stage 12: Convert Tables (HTML) & Classify Images
            if from_stage <= 12 <= to_stage:
                it["detected_type"] = None
                if it.get("status") == "checked":
                    it["status"] = "eval_ok" if it.get("eval_clip") == "ok" else "pending"
                it["conversion_status"] = "pending"
                it["conversion_stage"] = None
                it["generated_markdown"] = None
                it["generated_markdown_html"] = None
                it["generated_txt"] = None
                it["reduced_to_markdown"] = None
                it["reduction_reason"] = None
                it["is_embedded"] = False
                it["embedded_file"] = None

            # Stage 13: Reduce HTML Tables
            if from_stage <= 13 <= to_stage and dtype == "table_html":
                it["reduced_to_markdown"] = None
                it["reduction_reason"] = None
                if it.get("generated_markdown_html"):
                    it["generated_markdown"] = it.get("generated_markdown_html")

            # Stage 14: Convert Images & RAG Text
            if from_stage <= 14 <= to_stage and dtype == "image":
                it["conversion_status"] = "pending"
                it["conversion_stage"] = None
                it["generated_markdown"] = None
                it["generated_txt"] = None

            # Stage 16: Embed Final Markdown
            if from_stage <= 16 <= to_stage:
                it["is_embedded"] = False
                it["embedded_file"] = None

    if from_stage <= 18 <= to_stage and isinstance(data, dict):
        data.pop("continuation_stats", None)

    # Recalculate and synchronize all stats
    m_dir = manual_dir or queue_path.parent
    l_dir = layout_dir or (m_dir / "build" / "01_page_layout")
    stats = refresh_queue_statistics(data if isinstance(data, dict) else {}, items, m_dir, l_dir)

    if isinstance(data, dict):
        data["total_assets"] = len(items)
        data["queue"] = items
        data["stats"] = stats
    else:
        data = {"queue": items, "stats": stats, "total_assets": len(items)}

    with open(queue_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)

    return pruned_count, modified_count, stats


def collect_files_to_delete(
    manual_dir: Path,
    layout_dir: Path,
    target_pages: Set[int],
    from_stage: int,
    to_stage: int,
    chapters_path: Optional[Path] = None,
    assets_queue_path: Optional[Path] = None,
    is_all: bool = False,
    target_chapters: Optional[Set[int]] = None,
) -> List[Path]:
    """Scan disk and identify all artifacts belonging to target pages and stages (7..20)."""
    files_to_delete: List[Path] = []

    assets_dir = layout_dir / "assets"
    eval_frames_dir = layout_dir / "eval_frames"
    eval_final_dir = layout_dir / "eval_final"
    asset_frames_dir = layout_dir / "asset_frames"
    draft_chapters_dir = manual_dir / "build" / "02_detect_cont_chapters"
    final_chapters_dir = manual_dir / "build" / "02_final_chapters"

    # Build asset id -> detected type & conversion stage map for targeted file deletion
    aid_type_map: Dict[str, str] = {}
    aid_stage_map: Dict[str, Optional[int]] = {}
    if assets_queue_path and assets_queue_path.is_file():
        try:
            with open(assets_queue_path, "r", encoding="utf-8") as f:
                aq_data = json.load(f)
            for it in aq_data.get("queue", []):
                aid = it.get("asset_id")
                dtype = it.get("detected_type")
                c_stg = it.get("conversion_stage")
                if aid:
                    if dtype:
                        aid_type_map[aid] = dtype
                    aid_stage_map[aid] = c_stg
        except Exception:
            pass

    for p in target_pages:
        p_prefix = f"page_{p:04d}"

        # ----------------------------------------------------
        # Stage 7 files: page_XXXX.md
        # ----------------------------------------------------
        if from_stage <= 7 <= to_stage:
            for mf in layout_dir.glob(f"{p_prefix}*.md"):
                if mf.is_file():
                    files_to_delete.append(mf)

        # ----------------------------------------------------
        # Stage 8 files: derived markdown & initial crop assets
        # ----------------------------------------------------
        if from_stage <= 8 <= to_stage:
            if from_stage == 8:
                for mf in layout_dir.glob(f"{p_prefix}-*.md"):
                    if mf.is_file():
                        files_to_delete.append(mf)

            if assets_dir.is_dir():
                for af in assets_dir.glob(f"{p_prefix}_*"):
                    if af.is_file():
                        files_to_delete.append(af)

        # ----------------------------------------------------
        # Stage 9 files: eval frames, recrops, clip_final, finalized markdown
        # ----------------------------------------------------
        if from_stage <= 9 <= to_stage:
            if eval_frames_dir.is_dir():
                for ef in eval_frames_dir.glob(f"{p_prefix}_*"):
                    if ef.is_file():
                        files_to_delete.append(ef)

            if eval_final_dir.is_dir():
                for ff in eval_final_dir.glob(f"{p_prefix}_*"):
                    if ff.is_file():
                        files_to_delete.append(ff)

            if from_stage >= 9:
                for cf_file in layout_dir.glob(f"{p_prefix}*-clip_final.md"):
                    if cf_file.is_file():
                        files_to_delete.append(cf_file)

            if assets_dir.is_dir():
                for rf in assets_dir.glob(f"{p_prefix}_*_v*.png"):
                    if rf.is_file():
                        files_to_delete.append(rf)
                for cf in assets_dir.glob(f"{p_prefix}_*_clip_final.png"):
                    if cf.is_file():
                        files_to_delete.append(cf)

        # ----------------------------------------------------
        # Stage 10 files: rollback clip_final and eval_final frames
        # ----------------------------------------------------
        if from_stage == 10:
            for f_cf in layout_dir.glob(f"{p_prefix}*-clip_final.md"):
                if f_cf.is_file():
                    files_to_delete.append(f_cf)

            if eval_final_dir.is_dir():
                for ff in eval_final_dir.glob(f"{p_prefix}_*"):
                    if ff.is_file():
                        files_to_delete.append(ff)

            if assets_dir.is_dir():
                for cf in assets_dir.glob(f"{p_prefix}_*_clip_final.png"):
                    if cf.is_file():
                        files_to_delete.append(cf)

        # ----------------------------------------------------
        # Stage 12, 13 files: table fragments
        # ----------------------------------------------------
        # Stage 12 files: table fragments (_html.md, _reduced.md)
        # ----------------------------------------------------
        if from_stage <= 12 <= to_stage:
            if assets_dir.is_dir():
                for hf in assets_dir.glob(f"{p_prefix}_*_html.md"):
                    if hf.is_file():
                        files_to_delete.append(hf)
                for rf in assets_dir.glob(f"{p_prefix}_*_reduced.md"):
                    if rf.is_file():
                        files_to_delete.append(rf)

        # ----------------------------------------------------
        # Stage 13 files: reduced table fragments (_reduced.md only)
        # ----------------------------------------------------
        if from_stage <= 13 <= to_stage:
            if assets_dir.is_dir():
                for rf in assets_dir.glob(f"{p_prefix}_*_reduced.md"):
                    if rf.is_file():
                        files_to_delete.append(rf)

        # ----------------------------------------------------
        # Stage 14 files: image fragments (.md and .txt)
        # ----------------------------------------------------
        if from_stage <= 14 <= to_stage:
            if assets_dir.is_dir():
                for xf in assets_dir.glob(f"{p_prefix}_*.txt"):
                    if xf.is_file():
                        files_to_delete.append(xf)
                for im_md in assets_dir.glob(f"{p_prefix}_*.md"):
                    if im_md.is_file() and not im_md.name.endswith("_html.md") and not im_md.name.endswith("_reduced.md"):
                        files_to_delete.append(im_md)

        # ----------------------------------------------------
        # Stage 15 files: asset review frames in asset_frames/
        # ----------------------------------------------------
        if from_stage <= 15 <= to_stage:
            if asset_frames_dir.is_dir():
                for af in asset_frames_dir.glob(f"{p_prefix}_*"):
                    if af.is_file():
                        files_to_delete.append(af)

        # ----------------------------------------------------
        # Stage 16 files: embed document (page_XXXX-embed.md)
        # ----------------------------------------------------
        if from_stage <= 16 <= to_stage:
            f_embed = layout_dir / f"{p_prefix}-embed.md"
            if f_embed.is_file():
                files_to_delete.append(f_embed)

        # ----------------------------------------------------
        # Stage 17 files: proofread document (page_XXXX-proofread.md)
        # ----------------------------------------------------
        if from_stage <= 17 <= to_stage:
            f_proofread = layout_dir / f"{p_prefix}-proofread.md"
            if f_proofread.is_file():
                files_to_delete.append(f_proofread)

    # --------------------------------------------------------
    # Stage 18 & 19 Chapter Files:
    # Stage 18: Draft chapters in build/02_detect_cont_chapters/
    # Stage 19: Final chapters in build/02_final_chapters/
    # --------------------------------------------------------
    reset_stage18 = (from_stage <= 18 and to_stage >= 18) or (from_stage <= 17 and to_stage >= 18)
    reset_stage19 = (from_stage <= 19 and to_stage >= 19) or reset_stage18

    # 1. Reset Stage 18 draft chapters
    if reset_stage18 and draft_chapters_dir.is_dir():
        if is_all:
            for item in draft_chapters_dir.iterdir():
                if item.is_file():
                    files_to_delete.append(item)
                elif item.is_dir():
                    try:
                        shutil.rmtree(item)
                    except Exception as e:
                        sys.stderr.write(f"Warning: Could not remove directory {item.name}: {e}\n")
        elif target_chapters:
            for ch_num in target_chapters:
                for cf in draft_chapters_dir.glob(f"{ch_num:02d}*"):
                    if cf.is_file():
                        files_to_delete.append(cf)
        elif chapters_path and chapters_path.is_file():
            try:
                with open(chapters_path, "r", encoding="utf-8") as f:
                    ch_data = json.load(f)
                ch_list = ch_data.get("chapters", [])
                total_p = ch_data.get("total_pages", 1000)
                for idx, ch in enumerate(ch_list):
                    ch_num = idx + 1
                    sp = ch.get("start_page", 1)
                    np = ch_list[idx + 1].get("start_page", total_p + 1) if (idx + 1) < len(ch_list) else (total_p + 1)
                    ep = np - 1
                    ch_pages = set(range(sp, ep + 1))
                    if ch_pages.intersection(target_pages):
                        for cf in draft_chapters_dir.glob(f"{ch_num:02d}*"):
                            if cf.is_file():
                                files_to_delete.append(cf)
            except Exception as e:
                sys.stderr.write(f"Warning: Could not parse chapters map {chapters_path.name}: {e}\n")
        elif not chapters_path or not chapters_path.is_file():
            sys.stderr.write(
                f"Warning: Chapters map not found for {manual_dir.name}; cannot map target pages to draft chapters.\n"
            )

    # 2. Reset Stage 19 final chapters
    if reset_stage19 and final_chapters_dir.is_dir():
        if is_all:
            for item in final_chapters_dir.iterdir():
                if item.is_file():
                    files_to_delete.append(item)
                elif item.is_dir():
                    try:
                        shutil.rmtree(item)
                    except Exception as e:
                        sys.stderr.write(f"Warning: Could not remove directory {item.name}: {e}\n")
        elif target_chapters:
            for ch_num in target_chapters:
                for cf in final_chapters_dir.glob(f"{ch_num:02d}*"):
                    if cf.is_file():
                        files_to_delete.append(cf)
        elif chapters_path and chapters_path.is_file():
            try:
                with open(chapters_path, "r", encoding="utf-8") as f:
                    ch_data = json.load(f)
                ch_list = ch_data.get("chapters", [])
                total_p = ch_data.get("total_pages", 1000)
                for idx, ch in enumerate(ch_list):
                    ch_num = idx + 1
                    sp = ch.get("start_page", 1)
                    np = ch_list[idx + 1].get("start_page", total_p + 1) if (idx + 1) < len(ch_list) else (total_p + 1)
                    ep = np - 1
                    ch_pages = set(range(sp, ep + 1))
                    if ch_pages.intersection(target_pages):
                        for cf in final_chapters_dir.glob(f"{ch_num:02d}*"):
                            if cf.is_file():
                                files_to_delete.append(cf)
            except Exception as e:
                sys.stderr.write(f"Warning: Could not parse chapters map {chapters_path.name}: {e}\n")
        elif not chapters_path or not chapters_path.is_file():
            sys.stderr.write(
                f"Warning: Chapters map not found for {manual_dir.name}; cannot map target pages to final chapters.\n"
            )

    # Deduplicate while preserving order
    seen: Set[Path] = set()
    unique_files: List[Path] = []
    for f in files_to_delete:
        resolved = f.resolve()
        if resolved not in seen:
            seen.add(resolved)
            unique_files.append(f)

    return unique_files


def reset_stages(
    target_input: str,
    from_stage: int,
    to_stage: int,
    pages_str: Optional[str] = None,
    all_pages: bool = False,
    chapters_str: Optional[str] = None,
    no_plan: bool = False,
) -> bool:
    """Execute clean reset of specified stages for selected pages."""
    # Validation against supported stage boundary
    if from_stage < SUPPORTED_MIN_STAGE or to_stage > SUPPORTED_MAX_STAGE:
        sys.stderr.write(
            f"\n[ERROR] Invalid stage range: Stage {from_stage} -> Stage {to_stage}.\n"
            f"Reset is supported for Stages {SUPPORTED_MIN_STAGE} through {SUPPORTED_MAX_STAGE}.\n\n"
        )
        return False

    if from_stage > to_stage:
        sys.stderr.write(
            f"\n[ERROR] Start stage ({from_stage}) cannot be greater than end stage ({to_stage}).\n\n"
        )
        return False

    manual_dir, pdf_path, layout_dir, stage6_q, assets_q, chapters_p = resolve_manual_paths(target_input)
    if not manual_dir.exists():
        sys.stderr.write(f"\n[ERROR] Manual directory not found: {manual_dir}\n\n")
        return False

    target_chapters: Optional[Set[int]] = None
    if chapters_str:
        target_chapters = parse_chapter_range(chapters_str)

    is_all = all_pages or (pages_str is not None and pages_str.strip().lower() in ("all", "all-pages", "*"))
    if is_all:
        target_pages = resolve_all_pages(manual_dir, layout_dir, pdf_path)
        pages_display = f"all {len(target_pages)} pages"
    elif pages_str:
        target_pages = parse_page_range(pages_str)
        pages_display = pages_str
    elif target_chapters:
        resolved_pages: Set[int] = set()
        if chapters_p and chapters_p.is_file():
            try:
                with open(chapters_p, "r", encoding="utf-8") as f:
                    ch_data = json.load(f)
                ch_list = ch_data.get("chapters", [])
                total_p = ch_data.get("total_pages", 1000)
                for idx, ch in enumerate(ch_list):
                    ch_num = idx + 1
                    if ch_num in target_chapters:
                        sp = ch.get("start_page", 1)
                        np = ch_list[idx + 1].get("start_page", total_p + 1) if (idx + 1) < len(ch_list) else (total_p + 1)
                        resolved_pages.update(range(sp, np))
            except Exception:
                pass
        target_pages = resolved_pages
        ch_list_str = ",".join(str(c) for c in sorted(target_chapters))
        if target_pages:
            pages_display = f"chapter(s) {ch_list_str} (pages {min(target_pages)}-{max(target_pages)})"
        else:
            pages_display = f"chapter(s) {ch_list_str}"
    else:
        sys.stderr.write("\n[ERROR] Either --pages, --chapter, or --all must be specified.\n\n")
        return False

    if not target_pages and not (from_stage >= 19 and target_chapters):
        sys.stderr.write(f"\n[ERROR] No valid pages found in range: '{pages_display}'.\n\n")
        return False

    rel_manual = str(manual_dir.relative_to(Path.cwd())).replace("\\", "/") if manual_dir.is_relative_to(Path.cwd()) else manual_dir.name

    print("\n" + "=" * 65)
    print(f" PIPELINE STAGE RESET & CLEANUP")
    print("=" * 65)
    print(f"Manual:           {rel_manual}")
    print(f"Stage Range:      Stage {from_stage} -> Stage {to_stage}")
    print(f"Target Pages:     {len(target_pages)} page(s) ({pages_display})")
    print("-" * 65)

    # 1. Identify files to delete
    files_to_delete = collect_files_to_delete(
        manual_dir=manual_dir,
        layout_dir=layout_dir,
        target_pages=target_pages,
        from_stage=from_stage,
        to_stage=to_stage,
        chapters_path=chapters_p,
        assets_queue_path=assets_q,
        is_all=is_all,
        target_chapters=target_chapters,
    )

    # 2. Reset Stage 6 queue if from_stage <= 7
    s6_reset_count = 0
    s6_stats: Dict[str, Any] = {}
    if from_stage <= 7 and stage6_q:
        s6_reset_count, s6_stats = reset_stage6_queue(stage6_q, target_pages)

    # 3. Reset/Prune assets queue if exists
    aq_pruned = 0
    aq_modified = 0
    aq_stats: Dict[str, Any] = {}
    if assets_q:
        aq_pruned, aq_modified, aq_stats = reset_assets_queue(
            queue_path=assets_q,
            target_pages=target_pages,
            from_stage=from_stage,
            to_stage=to_stage,
            manual_dir=manual_dir,
            layout_dir=layout_dir,
        )

    # 4. Perform file deletion
    deleted_count = 0
    for f in files_to_delete:
        try:
            if f.is_file():
                f.unlink()
                deleted_count += 1
        except Exception as e:
            sys.stderr.write(f"Warning: Could not delete {f.name}: {e}\n")

    # Clean up empty intermediate dirs if left empty
    for d in [
        layout_dir / "eval_frames",
        layout_dir / "eval_final",
        layout_dir / "asset_frames",
        layout_dir / "assets",
        manual_dir / "build" / "02_detect_cont_chapters",
        manual_dir / "build" / "02_final_chapters",
    ]:
        if d.is_dir() and not any(d.iterdir()):
            try:
                d.rmdir()
            except Exception:
                pass

    # 5. Purge Ahead-Of-Time Execution Plans (invalidated by pipeline stage rollback)
    build_dir = manual_dir / "build"
    purged_plans: List[str] = []
    if build_dir.is_dir():
        for plan_f in sorted(build_dir.glob("*_execution_plan.json")):
            try:
                p_name = plan_f.name
                plan_f.unlink()
                purged_plans.append(p_name)
            except Exception as e:
                sys.stderr.write(f"Warning: Could not delete execution plan {plan_f.name}: {e}\n")

    # 6. Automatically regenerate Ahead-Of-Time Execution Plan for reset scope
    regenerated_plan: Optional[str] = None
    if not no_plan and generate_execution_plan is not None:
        try:
            plan_pages = None if is_all else target_pages
            plan_data = generate_execution_plan(
                manual_dir=manual_dir,
                from_stage=from_stage,
                to_stage=to_stage,
                pages=plan_pages,
                force=False,
                export_json=True,
            )
            regenerated_plan = plan_data.get("plan_file")
        except Exception as e:
            sys.stderr.write(f"Warning: Could not automatically regenerate execution plan: {e}\n")

    # 7. Output Summary Report
    print(f"\n[ACTION SUMMARY]")
    print(f"  Artifact files deleted: {deleted_count}")
    if files_to_delete:
        by_ext: Dict[str, int] = {}
        for f in files_to_delete:
            ext = f.suffix.lower()
            parent = f.parent.name
            key = f"{parent}/*{ext}"
            by_ext[key] = by_ext.get(key, 0) + 1

        for cat, cnt in sorted(by_ext.items()):
            print(f"    - {cat}: {cnt} file(s)")

    if regenerated_plan:
        print(f"  Ahead-Of-Time Execution Plan reset & regenerated:")
        print(f"    - {regenerated_plan} (Scope: Stages {from_stage}..{to_stage}, Pages: {pages_display})")
    elif purged_plans:
        print(f"  Ahead-Of-Time Execution Plans purged:")
        for pp in purged_plans:
            print(f"    - {pp} (Invalidated; regenerate with planner.py or run_pipeline.py --plan)")

    if from_stage <= 7:
        q_name = stage6_q.name if stage6_q else "None"
        print(f"  Stage 6 Queue ({q_name}):")
        print(f"    - Status reset to 'pending': {s6_reset_count} page(s)")
        if s6_stats:
            print(f"    - Updated Stats: {s6_stats.get('completed', 0)} completed, {s6_stats.get('pending', 0)} pending")

    if assets_q:
        print(f"  Assets Queue ({assets_q.name}):")
        if from_stage <= 8:
            print(f"    - Pruned assets: {aq_pruned} item(s)")
        else:
            print(f"    - Rolled back assets: {aq_modified} item(s)")
        if aq_stats:
            conv = aq_stats.get("by_status", {})
            print(f"    - Updated Stats: total={aq_stats.get('total', 0)}, pending={aq_stats.get('pending', 0)}, checked={aq_stats.get('checked', 0)}")

    print(f"\n[RESET SUCCESSFUL] Target scope ({pages_display}) ready for rerun of Stages {from_stage}..{to_stage}.")
    if regenerated_plan:
        print(f"  Execution Plan: '{regenerated_plan}' successfully configured with pending work units.\n")
    elif purged_plans:
        pages_arg = f" --pages {pages_display}" if not is_all and pages_display else ""
        print("  NOTE: Ahead-Of-Time execution plan was invalidated and purged.")
        print("        Regenerate before running orchestrator:")
        print(f"        python .agents/plugins/pdf-pipeline/skills/pdf-pipeline/scripts/planner.py \"{rel_manual}\" --from-stage {from_stage} --to-stage {to_stage}{pages_arg}\n")
    else:
        print()
    print("=" * 65 + "\n")
    return True


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Reset and clean up pipeline stages for selected pages (Stages 7 through 19).",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Reset Stages 18 and 19 across all manuals (re-bundle and re-merge chapters):
  python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py 18 19 --all

  # Reset Stage 19 only across all manuals (re-merge chapters from existing drafts):
  python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py 19 19 --all

  # Reset Stage 18 and 19 for Chapter 2:
  python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "Hardware Reference Manual" 18 19 --chapter 2

  # Reset Stages 11 through 19 across ALL target manuals (manual omitted):
  python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py 11 19 --all

  # Reset Stages 7 through 19 across all manuals for page 73:
  python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py 7 19 --pages 73

  # Reset Stages 7 through 19 for all pages of a specific manual:
  python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "Hardware Reference Manual" 7 19 --all

  # Reset Stages 11 through 19 for pages 70-80:
  python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "Hardware Reference Manual" 11 19 --pages 70-80
        """,
    )

    parser.add_argument(
        "positional_args",
        nargs="*",
        help="Target manual directory or PDF path (optional) and/or stage numbers (e.g. '11 14' or '\"Hardware Reference Manual\" 11 14')",
    )
    parser.add_argument(
        "--manual",
        "-m",
        dest="manual_flag",
        default=None,
        help="Target manual directory or PDF path (default: all target manuals if omitted)",
    )
    parser.add_argument(
        "--from-stage",
        "-from",
        "-s",
        type=int,
        dest="from_stage",
        help=f"Starting stage to reset ({SUPPORTED_MIN_STAGE}..{SUPPORTED_MAX_STAGE})",
    )
    parser.add_argument(
        "--to-stage",
        "-to",
        "-e",
        type=int,
        dest="to_stage",
        help=f"Ending stage to reset ({SUPPORTED_MIN_STAGE}..{SUPPORTED_MAX_STAGE}, default: 19)",
    )
    parser.add_argument(
        "--pages",
        "-p",
        help="Comma-separated pages and/or page ranges (e.g. '73', '70-80', '12,15,18-20') or 'all'",
    )
    parser.add_argument(
        "--chapter",
        "--chapters",
        "-c",
        dest="chapters",
        help="Comma-separated chapter numbers and/or chapter ranges (e.g. '1', '1-3', '2,4')",
    )
    parser.add_argument(
        "--all",
        "--all-pages",
        dest="all_pages",
        action="store_true",
        help="Reset all pages across the manual",
    )
    parser.add_argument(
        "--no-plan",
        action="store_true",
        help="Do not automatically regenerate build/<stem>_execution_plan.json after stage reset",
    )

    args = parser.parse_args()

    # Resolve from_stage, to_stage, and manual_target
    from_stage: Optional[int] = args.from_stage
    to_stage: Optional[int] = args.to_stage
    manual_input: Optional[str] = args.manual_flag

    pos_stage_args: List[int] = []

    for arg in args.positional_args:
        try:
            val = int(arg)
            pos_stage_args.append(val)
            continue
        except ValueError:
            pass

        if arg.lower() in ("all", "all-manuals", "*"):
            manual_input = "all"
        elif manual_input is None:
            manual_input = arg

    if pos_stage_args:
        if len(pos_stage_args) == 1:
            if from_stage is None:
                from_stage = pos_stage_args[0]
            elif to_stage is None:
                to_stage = pos_stage_args[0]
        elif len(pos_stage_args) >= 2:
            if from_stage is None:
                from_stage = pos_stage_args[0]
            if to_stage is None or args.to_stage is None:
                to_stage = pos_stage_args[1]

    if from_stage is None:
        sys.stderr.write("Error: Starting stage is required (e.g. '--from-stage 7' or positional '11 14' or '7 19').\n")
        sys.exit(1)

    if to_stage is None:
        to_stage = 19

    if not args.pages and not args.all_pages and not args.chapters:
        sys.stderr.write("Error: Either --pages (e.g. '--pages 73' or '--pages all'), --chapter (e.g. '--chapter 1'), or --all is required.\n")
        sys.exit(1)

    targets = resolve_manual_targets(manual_input)
    all_success = True
    for target in targets:
        success = reset_stages(
            target_input=target,
            from_stage=from_stage,
            to_stage=to_stage,
            pages_str=args.pages,
            all_pages=args.all_pages,
            chapters_str=args.chapters,
            no_plan=args.no_plan,
        )
        if not success:
            all_success = False

    if not all_success:
        sys.exit(1)


if __name__ == "__main__":
    main()

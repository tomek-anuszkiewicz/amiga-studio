"""Master Workflow Orchestration Script for Technical PDF-to-Markdown Pipeline.



Coordinates and automates the execution of pipeline stages (1 through 6, and 8),

validates stage prerequisites, and displays end-to-end progress.



Usage:

    python .agents/plugins/pdf-pipeline/skills/pdf-pipeline/scripts/run_pipeline.py "68000 Programmer's Reference Manual" --status

    python .agents/plugins/pdf-pipeline/skills/pdf-pipeline/scripts/run_pipeline.py "68000 Programmer's Reference Manual" --prep

    python .agents/plugins/pdf-pipeline/skills/pdf-pipeline/scripts/run_pipeline.py "68000 Programmer's Reference Manual" --stage 5 --pages 1-10

    python .agents/plugins/pdf-pipeline/skills/pdf-pipeline/scripts/run_pipeline.py "68000 Programmer's Reference Manual" --stage 8

"""



from __future__ import annotations



import argparse
import json
import os
import re
import subprocess
import sys

import urllib.parse

from pathlib import Path

from typing import Any, Dict, List, Optional, Set, Tuple



try:

    import pymupdf as fitz

except ImportError:

    try:

        import fitz  # type: ignore

    except ImportError:

        fitz = None

try:
    import psutil
except ImportError:
    psutil = None



# Safe console encoding on Windows

if sys.stdout.encoding and sys.stdout.encoding.lower() != "utf-8":

    try:

        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

        sys.stderr.reconfigure(encoding="utf-8", errors="replace")

    except Exception:

        pass



CONTINUATION_TAG_RE = re.compile(

    r"^<(continuation-(?:paragraph|table|code|index))>\s*\n?", re.IGNORECASE

)


def is_running_in_ide_chat() -> bool:
    """Detect if execution was triggered directly from Antigravity IDE Chat panel.

    Returns True ONLY if:
    1. ANTIGRAVITY_AGENT is active ('1'), AND
    2. The language server is configured for IDE (ANTIGRAVITY_LS_VERSION starts with 'ide-'), AND
    3. The process ancestry does NOT include agy.exe (the Antigravity CLI).

    If the agent was launched from the CLI (agy), or if a human runs python directly
    in their terminal, this returns False.
    """
    is_agent = os.environ.get("ANTIGRAVITY_AGENT") == "1"
    ls_version = os.environ.get("ANTIGRAVITY_LS_VERSION", "")

    if not (is_agent and ls_version.startswith("ide-")):
        return False

    if psutil is not None:
        try:
            proc = psutil.Process()
            while proc:
                name = proc.name().lower()
                if "agy" in name:
                    return False  # Invoked via agy CLI (even if running inside an IDE terminal tab)
                proc = proc.parent()
        except Exception:
            pass

    return True


def verify_runtime_environment(is_execution: bool, allow_ide: bool = False) -> None:
    """Enforce CLI runtime for batch pipeline execution commands."""
    if not is_execution or allow_ide:
        return

    if is_running_in_ide_chat():
        sys.stderr.write(
            "\n"
            "================================================================================\n"
            "[PIPELINE GATEKEEPER ERROR] Execution Blocked in Antigravity IDE Chat Panel!\n"
            "================================================================================\n"
            "Batch pipeline execution cannot be triggered directly from the Antigravity IDE\n"
            "Chat panel. Running multi-stage pipeline workloads inside the IDE Chat causes:\n"
            "  1. Breakdown of hierarchical subagent delegation (Tier 1 -> Tier 2 -> Tier 3)\n"
            "     because 'invoke_subagent' is not exposed in the IDE chat session.\n"
            "  2. Severe context window saturation (>100k tokens) causing hallucination and drift.\n"
            "  3. Extreme UI latency from real-time streaming diffs and syntax highlighting.\n"
            "  4. Repetitive tool permission throttling.\n"
            "\n"
            "HOW TO RUN PROPERLY:\n"
            "Open your terminal (or the IDE integrated terminal) and run the pipeline using\n"
            "the Antigravity CLI (agy):\n"
            "    agy -c --dangerously-skip-permissions\n"
            "\n"
            "Note: Read-only inspection commands (--status, --plan) remain permitted in IDE.\n"
            "(To bypass this check for testing or debugging, pass: --allow-ide)\n"
            "================================================================================\n\n"
        )
        sys.exit(2)






def find_manual_dir_and_pdf(target_input: str) -> Tuple[Path, Optional[Path], Optional[Path]]:

    """Resolve manual directory, source PDF, and OCR PDF."""

    p = Path(target_input)

    if not p.is_absolute():

        p = (Path.cwd() / p).resolve()



    if p.is_file() and p.suffix.lower() == ".pdf":
        if p.parent.name == "build":
            manual_dir = p.parent.parent
        else:
            manual_dir = p.parent
        pdf_file = p
    elif p.is_dir():
        if p.name == "build":
            manual_dir = p.parent
        else:
            manual_dir = p
        pdf_file = None
    else:
        # Check if matching subdirectory under current working directory
        candidate = Path.cwd() / target_input
        if candidate.is_dir():
            manual_dir = candidate.parent.resolve() if candidate.name == "build" else candidate.resolve()
            pdf_file = None
        else:
            manual_dir = p
            pdf_file = None

    # Find candidate PDFs in manual_dir/build and manual_dir
    source_pdf: Optional[Path] = None
    ocr_pdf: Optional[Path] = None

    if manual_dir.exists():
        build_dir = manual_dir / "build"
        if build_dir.is_dir():
            for f in build_dir.glob("*.pdf"):
                if f.name.endswith("_ocr.pdf"):
                    if ocr_pdf is None:
                        ocr_pdf = f
                else:
                    if source_pdf is None or len(f.name) < len(source_pdf.name):
                        source_pdf = f
        for f in manual_dir.glob("*.pdf"):
            if f.name.endswith("_ocr.pdf"):
                if ocr_pdf is None:
                    ocr_pdf = f
            else:
                if source_pdf is None or len(f.name) < len(source_pdf.name):
                    source_pdf = f

    # If specific PDF was passed as argument
    if pdf_file:
        if pdf_file.name.endswith("_ocr.pdf"):
            ocr_pdf = pdf_file
        else:
            source_pdf = pdf_file

    return manual_dir, source_pdf, ocr_pdf


def find_chapters_file(manual_dir: Path, stem: Optional[str] = None) -> Optional[Path]:
    """Find chapters JSON file, checking build/ first then manual_dir."""
    build_dir = manual_dir / "build"
    if stem:
        for d in (build_dir, manual_dir):
            c = d / f"{stem}_chapters.json"
            if c.exists():
                return c
    for d in (build_dir, manual_dir):
        if d.is_dir():
            for c in d.glob("*_chapters.json"):
                return c
    return None


def find_queue_file(manual_dir: Path, stem: Optional[str] = None) -> Optional[Path]:
    """Find Stage 6 inference queue JSON file, checking build/ first then manual_dir."""
    build_dir = manual_dir / "build"
    if stem:
        for d in (build_dir, manual_dir):
            q = d / f"{stem}_queue.json"
            if q.exists():
                return q
    for d in (build_dir, manual_dir):
        if d.is_dir():
            for q in d.glob("*_queue.json"):
                if not q.name.endswith("_assets_queue.json"):
                    return q
    return None


def find_assets_queue_file(manual_dir: Path, stem: Optional[str] = None) -> Optional[Path]:
    """Find Stage 8..11 assets queue JSON file, checking build/ first then manual_dir."""
    build_dir = manual_dir / "build"
    if stem:
        for d in (build_dir, manual_dir):
            aq = d / f"{stem}_assets_queue.json"
            if aq.exists():
                return aq
    for d in (build_dir, manual_dir):
        if d.is_dir():
            for aq in d.glob("*_assets_queue.json"):
                return aq
    return None





def get_pipeline_status(manual_dir: Path, source_pdf: Optional[Path], ocr_pdf: Optional[Path]) -> Dict[str, Any]:

    """Inspect files on disk to build a full, rigorous pipeline status report."""

    status: Dict[str, Any] = {

        "manual_dir": str(manual_dir.relative_to(Path.cwd())).replace("\\", "/") if manual_dir.is_relative_to(Path.cwd()) else manual_dir.name,

        "stages": {},

    }



    effective_pdf = ocr_pdf or source_pdf



    # Determine total pages in the PDF document if available

    total_pdf_pages: Optional[int] = None

    if effective_pdf and effective_pdf.exists() and effective_pdf.stat().st_size > 0:

        if fitz is not None:

            try:

                doc = fitz.open(effective_pdf)

                total_pdf_pages = len(doc)

                doc.close()

            except Exception:

                pass



    # Stage 2: Download / Source PDF

    s2_exists = bool(source_pdf and source_pdf.is_file() and source_pdf.stat().st_size > 0)

    status["stages"]["stage2"] = {

        "name": "Download / Source PDF",

        "ready": s2_exists,

        "file": source_pdf.name if source_pdf else None,

        "missing": [] if s2_exists else (["Source PDF missing or empty"] if source_pdf else ["Source PDF not configured"]),

    }



    # Stage 3: OCR PDF

    s3_exists = bool(ocr_pdf and ocr_pdf.is_file() and ocr_pdf.stat().st_size > 0)

    status["stages"]["stage3"] = {

        "name": "Searchable OCR Layer",

        "ready": s3_exists,

        "file": ocr_pdf.name if ocr_pdf else None,

        "missing": [] if s3_exists else (["OCR PDF missing or empty"] if ocr_pdf else ["OCR PDF not generated"]),

    }



    # Stage 4: Chapters JSON
    stem = effective_pdf.stem if effective_pdf else manual_dir.name
    chapters_file = find_chapters_file(manual_dir, stem)



    chapter_count = 0

    s4_ready = False

    s4_missing: List[str] = []

    if chapters_file and chapters_file.is_file() and chapters_file.stat().st_size > 0:

        try:

            with open(chapters_file, "r", encoding="utf-8") as f:

                ch_data = json.load(f)

                chapters_list = ch_data.get("chapters", [])

                chapter_count = len(chapters_list)

                if chapter_count > 0:

                    s4_ready = True

                else:

                    s4_missing.append("Chapters file exists but contains 0 detected chapters")

        except Exception as e:

            s4_missing.append(f"Invalid chapters JSON file: {e}")

    else:

        s4_missing.append(f"Chapters JSON map missing: {chapters_file.name if chapters_file else 'None'}")



    status["stages"]["stage4"] = {

        "name": "Chapter Detection",

        "ready": s4_ready,

        "file": chapters_file.name if chapters_file else None,

        "chapter_count": chapter_count,

        "missing": s4_missing,

    }



    # Stage 5: Layout Extraction

    layout_dir = manual_dir / "build" / "01_page_layout"



    # Queue file candidate (used as fallback for total pages if fitz was unavailable)
    queue_file = find_queue_file(manual_dir, stem)



    if total_pdf_pages is None and queue_file and queue_file.is_file():

        try:

            with open(queue_file, "r", encoding="utf-8") as f:

                q_head = json.load(f)

                total_pdf_pages = q_head.get("total_pages") or q_head.get("stats", {}).get("total")

        except Exception:

            pass



    status["total_pdf_pages"] = total_pdf_pages



    missing_json: List[str] = []

    missing_png: List[str] = []

    json_count = 0

    png_count = 0



    if total_pdf_pages and total_pdf_pages > 0 and layout_dir.exists():

        for p in range(1, total_pdf_pages + 1):

            jf = layout_dir / f"page_{p:04d}.json"

            if jf.is_file() and jf.stat().st_size > 0:

                json_count += 1

            else:

                missing_json.append(f"page_{p:04d}.json")



            pf = layout_dir / f"page_{p:04d}.png"

            if pf.is_file() and pf.stat().st_size > 0:

                png_count += 1

            else:

                missing_png.append(f"page_{p:04d}.png")

        s5_ready = (len(missing_json) == 0 and len(missing_png) == 0)

    else:

        if layout_dir.exists():

            json_count = len([f for f in layout_dir.glob("page_*.json") if f.stat().st_size > 0])

            png_count = len([f for f in layout_dir.glob("page_*.png") if f.stat().st_size > 0])

            s5_ready = (json_count > 0 and json_count == png_count)

        else:

            s5_ready = False



    status["stages"]["stage5"] = {

        "name": "Page Layout & Previews",

        "ready": s5_ready,

        "json_pages": json_count,

        "png_pages": png_count,

        "total_expected": total_pdf_pages,

        "missing_json": missing_json,

        "missing_png": missing_png,

    }



    # Stage 6: Inference Queue

    q_total = 0

    q_completed = 0

    q_pending = 0

    s6_ready = False

    s6_missing: List[str] = []

    if queue_file and queue_file.is_file() and queue_file.stat().st_size > 0:

        try:

            with open(queue_file, "r", encoding="utf-8") as f:

                q_data = json.load(f)

                queue_items = q_data.get("queue", [])

                if queue_items:

                    q_total = len(queue_items)

                    q_completed = sum(1 for item in queue_items if item.get("status") in ("completed", "done"))

                    q_pending = sum(1 for item in queue_items if item.get("status") not in ("completed", "done"))

                else:

                    q_stats = q_data.get("stats", {})

                    q_total = q_stats.get("total", 0)

                    q_completed = q_stats.get("completed", 0)

                    q_pending = q_stats.get("pending", 0)



                if total_pdf_pages and q_total != total_pdf_pages:

                    s6_missing.append(f"Queue size ({q_total}) does not match total PDF pages ({total_pdf_pages})")



                if q_total > 0 and q_pending == 0 and (total_pdf_pages is None or q_total == total_pdf_pages):

                    s6_ready = True

                elif q_pending > 0:

                    s6_missing.append(f"{q_pending} pending item(s) in queue")

        except Exception as e:

            s6_missing.append(f"Invalid queue JSON: {e}")

    else:

        s6_missing.append("Queue file not initialized")



    status["stages"]["stage6"] = {

        "name": "Inference Queue",

        "ready": s6_ready,

        "file": queue_file.name if queue_file else None,

        "total": q_total,

        "completed": q_completed,

        "pending": q_pending,

        "missing": s6_missing,

    }



    # Stage 7: Markdown Prose

    missing_md: List[str] = []

    md_count = 0

    if total_pdf_pages and total_pdf_pages > 0 and layout_dir.exists():

        for p in range(1, total_pdf_pages + 1):

            mf = layout_dir / f"page_{p:04d}.md"

            if mf.is_file() and mf.stat().st_size > 0:

                md_count += 1

            else:

                missing_md.append(f"page_{p:04d}.md")

        s7_ready = (len(missing_md) == 0 and md_count == total_pdf_pages)

    else:

        if layout_dir.exists():

            md_files = [f for f in layout_dir.glob("page_*.md") if not f.name.endswith("-images.md") and f.stat().st_size > 0]

            md_count = len(md_files)

            s7_ready = (md_count > 0)

        else:

            s7_ready = False



    status["stages"]["stage7"] = {

        "name": "Markdown Prose (LLM)",

        "ready": s7_ready,

        "md_pages": md_count,

        "total_expected": total_pdf_pages,

        "missing_md": missing_md,

        "in_progress": q_pending > 0 and q_completed > 0,

    }



    # Stage 8: Visual Assets & Final Markdown

    assets_dir = layout_dir / "assets"

    asset_files_on_disk = list(assets_dir.glob("*.png")) if assets_dir.exists() else []

    asset_count = len(asset_files_on_disk)



    missing_final_docs: List[str] = []

    final_md_count = 0

    if total_pdf_pages and total_pdf_pages > 0 and layout_dir.exists():

        for p in range(1, total_pdf_pages + 1):

            fmf = layout_dir / f"page_{p:04d}-images.md"

            if fmf.is_file() and fmf.stat().st_size > 0:

                final_md_count += 1

            else:

                missing_final_docs.append(f"page_{p:04d}-images.md")

    elif layout_dir.exists():

        existing_final_md = [f for f in layout_dir.glob("*-images.md") if f.stat().st_size > 0]

        final_md_count = len(existing_final_md)



    # Scan all *-images.md files for referenced assets and verify presence on disk

    missing_assets: List[Dict[str, str]] = []

    unresolved_crops: List[str] = []

    referenced_assets_set: Set[str] = set()



    img_ref_pattern = re.compile(r'!\[.*?\]\(([^)]+)\)|<img\s+[^>]*src=["\']([^"\']+)["\']', re.IGNORECASE)

    crop_tag_pattern = re.compile(r'<crop\b', re.IGNORECASE)



    if layout_dir.exists():

        for fmf in sorted(layout_dir.glob("*-images.md")):

            try:

                txt = fmf.read_text(encoding="utf-8", errors="replace")

            except Exception:

                continue



            if crop_tag_pattern.search(txt):

                unresolved_crops.append(fmf.name)



            for m in img_ref_pattern.finditer(txt):

                raw_ref = m.group(1) or m.group(2)

                if not raw_ref:

                    continue

                clean_ref = raw_ref.split()[0].strip().strip("<>\"'").replace("\\", "/")

                clean_ref = urllib.parse.unquote(clean_ref)

                if clean_ref.startswith("http://") or clean_ref.startswith("https://") or clean_ref.startswith("data:"):

                    continue

                referenced_assets_set.add(clean_ref)

                img_path = layout_dir / clean_ref

                if not img_path.is_file() or img_path.stat().st_size == 0:

                    missing_assets.append({"ref": clean_ref, "doc": fmf.name})



    # Stage 8 & 9 assets processing queue

    assets_queue_file = None

    expected_aq_name = f"{effective_pdf.stem}_assets_queue.json" if effective_pdf else None
    assets_queue_file = find_assets_queue_file(manual_dir, effective_pdf.stem if effective_pdf else None)



    # Determine Stage 8 readiness (must produce final markdown, cropped assets, AND assets queue JSON)

    has_queue = assets_queue_file is not None

    if final_md_count == 0 and asset_count == 0 and not has_queue:

        s8_ready = False

        s8_status = "unstarted"

    elif (

        final_md_count > 0

        and len(missing_final_docs) == 0

        and len(missing_assets) == 0

        and len(unresolved_crops) == 0

        and (total_pdf_pages is None or final_md_count == total_pdf_pages)

        and (asset_count == 0 or has_queue)

    ):

        s8_ready = True

        s8_status = "done"

    elif not has_queue and (asset_count > 0 or final_md_count > 0):

        s8_ready = False

        s8_status = "missing_queue"

    else:

        s8_ready = False

        s8_status = "broken"



    status["stages"]["stage8"] = {

        "name": "Visual Assets & Final Markdown",

        "ready": s8_ready,

        "status_code": s8_status,

        "assets": asset_count,

        "referenced_assets": len(referenced_assets_set),

        "missing_assets": missing_assets,

        "unresolved_crops": unresolved_crops,

        "final_documents": final_md_count,

        "total_expected": total_pdf_pages,

        "missing_final_docs": missing_final_docs,

        "queue_file": assets_queue_file.name if assets_queue_file else None,

        "expected_queue_file": expected_aq_name,

    }



    # Stage 9: Assets Processing Queue



    s9_total = 0

    s9_completed = 0

    s9_checked = 0

    s9_pending = 0

    s9_ready = False

    s9_missing: List[str] = []

    by_status: Dict[str, int] = {}

    by_type: Dict[str, int] = {}

    by_eval_clip: Dict[str, int] = {}



    aq_data: Optional[Dict[str, Any]] = None
    aq_items: List[Dict[str, Any]] = []

    if assets_queue_file and assets_queue_file.is_file() and assets_queue_file.stat().st_size > 0:

        try:

            with open(assets_queue_file, "r", encoding="utf-8") as f:

                aq_data = json.load(f)

                aq_items = aq_data.get("queue", []) if isinstance(aq_data, dict) else aq_data

                s9_total = len(aq_items)

                s9_completed = sum(1 for item in aq_items if str(item.get("status", "")).lower() == "completed")

                s9_checked = sum(1 for item in aq_items if str(item.get("status", "")).lower() == "checked")

                s9_pending = sum(1 for item in aq_items if str(item.get("status", "")).lower() == "pending")

                for it in aq_items:

                    st = it.get("status", "pending")

                    by_status[st] = by_status.get(st, 0) + 1

                    dt = it.get("detected_type")

                    if dt:

                        by_type[dt] = by_type.get(dt, 0) + 1

                    ev = it.get("eval_clip")

                    if ev:

                        by_eval_clip[ev] = by_eval_clip.get(ev, 0) + 1



                missing_queue_assets: List[str] = []

                missing_queue_docs: List[str] = []

                for it in aq_items:

                    asset_rel = it.get("asset")

                    if asset_rel:

                        asset_file = manual_dir / asset_rel

                        if not asset_file.is_file() or asset_file.stat().st_size == 0:

                            missing_queue_assets.append(asset_rel)

                    md_rel = it.get("md_file")

                    if md_rel:

                        md_file_path = manual_dir / md_rel

                        if not md_file_path.is_file() or md_file_path.stat().st_size == 0:

                            missing_queue_docs.append(md_rel)



                if missing_queue_assets:

                    sample = ", ".join(missing_queue_assets[:5])

                    more = f" ... and {len(missing_queue_assets)-5} more" if len(missing_queue_assets) > 5 else ""

                    s9_missing.append(f"Missing {len(missing_queue_assets)} referenced asset file(s) on disk: {sample}{more}")



                if missing_queue_docs:

                    unique_missing_docs = sorted(list(set(missing_queue_docs)))

                    sample = ", ".join(unique_missing_docs[:5])

                    more = f" ... and {len(unique_missing_docs)-5} more" if len(unique_missing_docs) > 5 else ""

                    s9_missing.append(f"Missing {len(unique_missing_docs)} referenced markdown file(s) on disk: {sample}{more}")



                if asset_count > 0:

                    queue_asset_names = set()

                    for it in aq_items:

                        for k in ("asset", "original_asset", "active_asset", "final_asset"):

                            v = it.get(k)

                            if v:

                                queue_asset_names.add(Path(v).name)

                        for v in it.get("versions", []):

                            vf = v.get("asset_file")

                            if vf:

                                queue_asset_names.add(Path(vf).name)

                    missing_from_queue = [f.name for f in sorted(asset_files_on_disk) if f.name not in queue_asset_names]

                    if missing_from_queue:

                        sample = ", ".join(missing_from_queue[:5])

                        more = f" ... and {len(missing_from_queue)-5} more" if len(missing_from_queue) > 5 else ""

                        s9_missing.append(f"Found {len(missing_from_queue)} unmanaged asset file(s) on disk not tracked in queue: {sample}{more}")

                finalized_count = sum(1 for item in aq_items if item.get("final_asset") is not None)

                frames_count = sum(
                    1 for item in aq_items
                    if item.get("eval_frame") is not None
                    or item.get("active_box") == [0, 0, 1000, 1000]
                    or item.get("final_asset") is not None
                    or item.get("eval_clip") == "ok"
                )

                reclip_needed_count = sum(

                    1 for item in aq_items

                    if item.get("corrected_box") is not None

                    or item.get("eval_clip") in {"reclip_needed", "clipped_edge", "severed_text", "bad_bbox", "excess_whitespace"}

                    or str(item.get("status", "")).lower() == "awaiting_reclip"

                )



                # Markdown finalize & clip_final link verification

                missing_md_updates = 0

                for it in aq_items:

                    if it.get("eval_clip") == "ok" or it.get("final_asset") is not None:

                        md_rel = it.get("md_file")

                        if not md_rel or "-clip_final.md" not in md_rel:

                            missing_md_updates += 1

                        else:

                            md_path = manual_dir / md_rel

                            if not md_path.is_file() or md_path.stat().st_size == 0:

                                missing_md_updates += 1



                # Check existence of *-images-clip_final.md documents on disk

                existing_clip_final_md = list(layout_dir.glob("*-images-clip_final.md")) if layout_dir.exists() else []

                clip_final_md_count = len(existing_clip_final_md)

                missing_clip_final_docs: List[str] = []

                if total_pdf_pages and total_pdf_pages > 0 and layout_dir.exists():

                    for p in range(1, total_pdf_pages + 1):

                        cf = layout_dir / f"page_{p:04d}-images-clip_final.md"

                        if not cf.is_file() or cf.stat().st_size == 0:

                            missing_clip_final_docs.append(f"page_{p:04d}-images-clip_final.md")



                # Verify all images referenced inside *-images-clip_final.md are _clip_final.png and exist on disk

                unfinalized_clip_refs: List[Dict[str, str]] = []

                missing_clip_final_assets: List[Dict[str, str]] = []

                if layout_dir.exists():

                    for cf in existing_clip_final_md:

                        try:

                            txt = cf.read_text(encoding="utf-8", errors="replace")

                        except Exception:

                            continue

                        for m in img_ref_pattern.finditer(txt):

                            raw_ref = m.group(1) or m.group(2)

                            if not raw_ref:

                                continue

                            clean_ref = raw_ref.split()[0].strip().strip("<>\"'").replace("\\", "/")

                            clean_ref = urllib.parse.unquote(clean_ref)

                            if clean_ref.startswith("http://") or clean_ref.startswith("https://") or clean_ref.startswith("data:"):

                                continue

                            if not clean_ref.endswith("_clip_final.png"):

                                unfinalized_clip_refs.append({"ref": clean_ref, "doc": cf.name})

                            img_path = layout_dir / clean_ref

                            if not img_path.is_file() or img_path.stat().st_size == 0:

                                missing_clip_final_assets.append({"ref": clean_ref, "doc": cf.name})



                if missing_md_updates > 0:

                    s9_missing.append(f"{missing_md_updates} asset(s) awaiting Markdown finalize / link updates (--finalize)")



                if missing_clip_final_docs:

                    sample = ", ".join(missing_clip_final_docs[:5])

                    more = f" ... and {len(missing_clip_final_docs)-5} more" if len(missing_clip_final_docs) > 5 else ""

                    s9_missing.append(f"Missing {len(missing_clip_final_docs)} final document(s) (*-images-clip_final.md): {sample}{more}")



                if unfinalized_clip_refs:

                    sample = ", ".join([f"{item['ref']} in {item['doc']}" for item in unfinalized_clip_refs[:5]])

                    more = f" ... and {len(unfinalized_clip_refs)-5} more" if len(unfinalized_clip_refs) > 5 else ""

                    s9_missing.append(f"Found {len(unfinalized_clip_refs)} unfinalized asset reference(s) in final markdown (expected _clip_final.png): {sample}{more}")



                if missing_clip_final_assets:

                    sample = ", ".join([f"{item['ref']} in {item['doc']}" for item in missing_clip_final_assets[:5]])

                    more = f" ... and {len(missing_clip_final_assets)-5} more" if len(missing_clip_final_assets) > 5 else ""

                    s9_missing.append(f"Missing {len(missing_clip_final_assets)} referenced final asset image(s) on disk: {sample}{more}")



                s9_ready = (

                    s9_total > 0

                    and s9_pending == 0

                    and finalized_count == s9_total

                    and missing_md_updates == 0

                    and (total_pdf_pages is None or len(missing_clip_final_docs) == 0)

                    and len(unfinalized_clip_refs) == 0

                    and len(missing_clip_final_assets) == 0

                    and len(missing_queue_assets) == 0

                    and len(missing_queue_docs) == 0

                )

        except Exception as e:

            s9_missing.append(f"Invalid assets queue JSON: {e}")

    else:

        s9_missing.append("Assets queue not initialized")

        finalized_count = 0

        frames_count = 0

        reclip_needed_count = 0

        missing_md_updates = 0

        clip_final_md_count = 0

        missing_clip_final_docs = []

        unfinalized_clip_refs = []

        missing_clip_final_assets = []

    loop_completed = sum(1 for it in aq_items if it.get("final_asset") is not None or it.get("eval_clip") == "ok")

    status["stages"]["stage9"] = {

        "name": "Prepare Eval Frames & Recrop",

        "ready": s9_ready,

        "file": assets_queue_file.name if assets_queue_file else None,

        "total": s9_total,

        "loop_completed": loop_completed,

        "finalized": finalized_count,

        "frames_ready": frames_count,

        "reclip_needed": reclip_needed_count,

        "missing_md_updates": missing_md_updates,

        "clip_final_md_count": clip_final_md_count,

        "missing_clip_final_docs": missing_clip_final_docs,

        "unfinalized_clip_refs": unfinalized_clip_refs,

        "missing_clip_final_assets": missing_clip_final_assets,

        "completed": s9_completed,

        "checked": s9_checked,

        "pending": s9_pending,

        "by_status": by_status,

        "by_type": by_type,

        "by_eval_clip": by_eval_clip,

        "missing": s9_missing,

    }



    # Stage 10: Multimodal Crop Quality Evaluation (eval-clip)

    ok_count = by_eval_clip.get("ok", 0)

    rec_count = reclip_needed_count

    eval_count = sum(by_eval_clip.values())

    s10_ready = (s9_total > 0 and ok_count == s9_total)

    done_eval_passes = 0
    for it in aq_items:
        vers = it.get("versions", [])
        if vers:
            for v in vers:
                if v.get("eval_result") is not None or it.get("eval_clip") == "ok":
                    done_eval_passes += 1
        elif it.get("eval_clip") is not None:
            done_eval_passes += 1
    pending_assets_count = sum(1 for it in aq_items if it.get("eval_clip") is None)
    dynamic_total_passes = done_eval_passes + pending_assets_count

    status["stages"]["stage10"] = {

        "name": "Crop Quality Evaluation (eval-clip)",

        "ready": s10_ready,

        "total": s9_total,

        "ok": ok_count,

        "reclip": rec_count,

        "evaluated": eval_count,

        "pending": max(0, s9_total - eval_count),

        "done_passes": done_eval_passes,

        "total_passes": dynamic_total_passes,

    }



    # Downstream Stages 11..18 inspection

    aq_items_list = aq_items if (assets_queue_file and assets_queue_file.is_file()) else []



    # Stage 11: Prepare Conversion Queue

    s11_initialized = sum(1 for it in aq_items_list if "conversion_status" in it)

    s11_ready = (s9_total > 0 and s11_initialized == s9_total)

    status["stages"]["stage11"] = {

        "name": "Prepare Conversion Queue",

        "ready": s11_ready,

        "total": s9_total,

        "initialized": s11_initialized,

        "pending": max(0, s9_total - s11_initialized),

    }



    # Stage 12: Convert Tables & Classify Images
    s12_pool = s9_total
    classified_items = [it for it in aq_items_list if it.get("detected_type")]
    classified_count = len(classified_items)
    unclassified_count = max(0, s12_pool - classified_count)

    html_tables = [it for it in aq_items_list if it.get("detected_type") == "table_html"]
    html_tbl_tot = len(html_tables)
    html_tbl_converted = 0
    missing_html_tbl_files: List[str] = []
    for it in html_tables:
        aid = it.get("asset_id")
        f_html = layout_dir / "assets" / f"{aid}_html.md"
        if f_html.is_file() and f_html.stat().st_size > 0 and it.get("conversion_status") in ("completed", "converted"):
            html_tbl_converted += 1
        else:
            missing_html_tbl_files.append(f"{aid}_html.md")

    s12_all_classified = (s12_pool > 0 and classified_count == s12_pool)
    s12_tables_ok = (html_tbl_tot == 0 or html_tbl_converted == html_tbl_tot)
    s12_ready = (s12_all_classified and s12_tables_ok)

    status["stages"]["stage12"] = {
        "name": "Convert Tables (HTML)",
        "ready": s12_ready,
        "total_assets": s12_pool,
        "pool": s12_pool,
        "classified": classified_count,
        "unclassified": unclassified_count,
        "tables_found": html_tbl_tot,
        "converted_tables": html_tbl_converted,
        "pending_tables": html_tbl_tot - html_tbl_converted,
        "missing_files": missing_html_tbl_files,
    }

    # Stage 13: Reduce HTML Tables
    html_reduced = sum(1 for it in html_tables if it.get("reduced_to_markdown") is True)
    html_kept = sum(1 for it in html_tables if it.get("reduced_to_markdown") is False)
    html_eval_tot = html_reduced + html_kept

    s13_ready = s12_ready and (html_tbl_tot == 0 or html_eval_tot == html_tbl_tot)

    status["stages"]["stage13"] = {
        "name": "Reduce HTML Tables",
        "ready": s13_ready,
        "total": html_tbl_tot,
        "evaluated": html_eval_tot,
        "reduced": html_reduced,
        "kept_html": html_kept,
        "pending": html_tbl_tot - html_eval_tot,
    }

    # Stage 14: Convert Images & RAG Text
    img_assets = [it for it in aq_items_list if it.get("detected_type") == "image"]
    img_tot = len(img_assets)
    img_converted = 0
    missing_img_files: List[str] = []
    for it in img_assets:
        aid = it.get("asset_id")
        f_md = layout_dir / "assets" / f"{aid}.md"
        f_txt = layout_dir / "assets" / f"{aid}.txt"
        if f_md.is_file() and f_md.stat().st_size > 0 and f_txt.is_file() and f_txt.stat().st_size > 0 and it.get("conversion_status") in ("completed", "converted"):
            img_converted += 1
        else:
            missing_img_files.append(f"{aid} (.md/.txt)")

    s14_ready = s12_ready and (img_tot == 0 or img_converted == img_tot)

    status["stages"]["stage14"] = {
        "name": "Convert Images & RAG Text",
        "ready": s14_ready,
        "total": img_tot,
        "converted": img_converted,
        "pending": img_tot - img_converted,
        "missing_files": missing_img_files,
    }

    # Stage 15: Render Asset Frames
    asset_frames_dir = layout_dir / "asset_frames"
    rendered_asset_frames = len(list(asset_frames_dir.glob("page_*_asset_frame.png"))) if asset_frames_dir.is_dir() else 0
    s15_ready = (total_pdf_pages is not None and total_pdf_pages > 0 and rendered_asset_frames >= total_pdf_pages)
    status["stages"]["stage15"] = {
        "name": "Render Asset Frames",
        "ready": s15_ready,
        "rendered": rendered_asset_frames,
        "expected": total_pdf_pages,
    }

    # Stage 16: Embed Markdown
    embed_docs_count = 0
    missing_embed_docs: List[str] = []
    embedded_assets_count = sum(1 for it in aq_items_list if it.get("is_embedded") is True)
    if total_pdf_pages and total_pdf_pages > 0 and layout_dir.exists():
        for p in range(1, total_pdf_pages + 1):
            emf = layout_dir / f"page_{p:04d}-embed.md"
            if emf.is_file() and emf.stat().st_size > 0:
                embed_docs_count += 1
            else:
                missing_embed_docs.append(f"page_{p:04d}-embed.md")
        s16_ready = (len(missing_embed_docs) == 0 and embed_docs_count == total_pdf_pages)
    elif layout_dir.exists():
        existing_embed = list(layout_dir.glob("page_*-embed.md"))
        embed_docs_count = len(existing_embed)
        s16_ready = (embed_docs_count > 0)
    else:
        s16_ready = False

    status["stages"]["stage16"] = {
        "name": "Embed Markdown",
        "ready": s16_ready,
        "embedded_pages": embed_docs_count,
        "total_expected": total_pdf_pages,
        "embedded_assets": embedded_assets_count,
        "total_assets": s9_total,
        "missing_embed_docs": missing_embed_docs,
    }

    # Stage 17: Proofread Page
    proofread_docs_count = 0
    missing_proofread_docs: List[str] = []
    if total_pdf_pages and total_pdf_pages > 0 and layout_dir.exists():
        for p in range(1, total_pdf_pages + 1):
            pmf = layout_dir / f"page_{p:04d}-proofread.md"
            if pmf.is_file() and pmf.stat().st_size > 0:
                proofread_docs_count += 1
            else:
                missing_proofread_docs.append(f"page_{p:04d}-proofread.md")
        s17_ready = (len(missing_proofread_docs) == 0 and proofread_docs_count == total_pdf_pages)
    elif layout_dir.exists():
        existing_proofread = list(layout_dir.glob("page_*-proofread.md"))
        proofread_docs_count = len(existing_proofread)
        s17_ready = (proofread_docs_count > 0 and embed_docs_count > 0 and proofread_docs_count >= embed_docs_count)
    else:
        s17_ready = False

    status["stages"]["stage17"] = {
        "name": "Proofread Page",
        "ready": s17_ready,
        "proofread_pages": proofread_docs_count,
        "total_expected": total_pdf_pages or embed_docs_count,
        "missing_proofread_docs": missing_proofread_docs,
    }

    # Stage 18: Prepare Chapters
    draft_chapters_dir = manual_dir / "build" / "02_detect_cont_chapters"
    draft_chapters: List[str] = []
    if draft_chapters_dir.is_dir():
        for df in sorted(draft_chapters_dir.glob("*.md")):
            if df.is_file() and df.stat().st_size > 0:
                draft_chapters.append(df.name)
    draft_count = len(draft_chapters)
    expected_chapters = status["stages"].get("stage4", {}).get("chapter_count", 0)
    s18_ready = (expected_chapters > 0 and draft_count >= expected_chapters)
    status["stages"]["stage18"] = {
        "name": "Prepare Chapters",
        "ready": s18_ready,
        "draft_count": draft_count,
        "expected_count": expected_chapters,
        "chapters": draft_chapters,
    }

    # Stage 19: Merge Final Chapters
    final_chapters_dir = manual_dir / "build" / "02_final_chapters"
    assembled_chapters: List[str] = []
    unmerged_markers_count = 0
    if final_chapters_dir.is_dir():
        for cf in sorted(final_chapters_dir.glob("*.md")):
            if cf.is_file() and cf.stat().st_size > 0:
                try:
                    txt = cf.read_text(encoding="utf-8", errors="replace")
                    if "<continuation-marker>" in txt:
                        unmerged_markers_count += 1
                    else:
                        assembled_chapters.append(cf.name)
                except Exception:
                    pass
    assembled_count = len(assembled_chapters)
    s19_ready = (expected_chapters > 0 and assembled_count >= expected_chapters and unmerged_markers_count == 0)
    status["stages"]["stage19"] = {
        "name": "Merge Final Chapters",
        "ready": s19_ready,
        "assembled_count": assembled_count,
        "expected_count": expected_chapters,
        "unmerged_markers": unmerged_markers_count,
        "chapters": assembled_chapters,
    }



    return status





ANSI_GREEN = "\033[92m"
ANSI_YELLOW = "\033[93m"
ANSI_RED = "\033[91m"
ANSI_DIM = "\033[90m"
ANSI_BOLD = "\033[1m"
ANSI_RESET = "\033[0m"

# Enable Windows virtual terminal processing
if os.name == "nt":
    try:
        os.system("")
    except Exception:
        pass


def _pad_cell(txt: str, width: int, align: str = "left") -> str:
    """Pad string taking ANSI escape sequences into account for visual length."""
    vis_len = len(re.sub(r"\x1b\[[0-9;]*m", "", txt))
    pad_len = max(0, width - vis_len)
    return txt + (" " * pad_len) if align == "left" else (" " * pad_len) + txt


def _make_progress_bar(
    done: int,
    total: int,
    width: int = 18,
    status: str = "WAITING",
    count_label: Optional[str] = None,
) -> str:
    """Render a colored progress bar with percentage and numeric count."""
    try:
        "█░".encode(sys.stdout.encoding or "utf-8")
        fill_char = "█"
        empty_char = "░"
    except Exception:
        fill_char = "#"
        empty_char = "-"

    if total <= 0:
        pct = 100 if done > 0 or status == "DONE" else 0
        fill = width if pct == 100 else 0
    else:
        if done >= total or status == "DONE":
            pct = 100
            fill = width
        elif done <= 0:
            pct = 0
            fill = 0
        else:
            pct = min(99, max(1, int(done / total * 100)))
            fill = min(width - 1, max(0, int(round(done / total * width))))
    empty = width - fill

    if status == "DONE":
        b = f"{ANSI_GREEN}" + (fill_char * fill) + f"{ANSI_RESET}"
    elif status == "IN PROGRESS":
        b = f"{ANSI_YELLOW}" + (fill_char * fill) + f"{ANSI_DIM}" + (empty_char * empty) + f"{ANSI_RESET}"
    else:
        b = f"{ANSI_DIM}" + (empty_char * width) + f"{ANSI_RESET}"

    if count_label is not None:
        cnt_str = count_label
    elif total <= 0:
        cnt_str = f"({done} done)" if done > 0 else ("(N/A)" if status == "DONE" else "(0/0)")
    else:
        cnt_str = f"({done}/{total})"
    return f"[{b}] {pct:>3}% {cnt_str:>20}"


def _get_stage_row_info(st_num: int, stages: Dict[str, Any], total_pages: int) -> Tuple[str, str, int, int, str]:
    """Return (descriptive_name, status_str, done_count, total_count, count_label) for stage st_num."""
    if st_num == 2:
        s = stages.get("stage2", {})
        ready = s.get("ready", False)
        st = "DONE" if ready else "WAITING"
        cnt = "(1/1 doc)" if ready else "(0/1 doc)"
        return "Download Source PDF", st, 1 if ready else 0, 1, cnt

    if st_num == 3:
        s = stages.get("stage3", {})
        ready = s.get("ready", False)
        st = "DONE" if ready else "WAITING"
        cnt = "(1/1 doc)" if ready else "(0/1 doc)"
        return "Searchable OCR Layer", st, 1 if ready else 0, 1, cnt

    if st_num == 4:
        s = stages.get("stage4", {})
        ready = s.get("ready", False)
        ch_count = s.get("chapter_count", 0)
        tot = max(1, ch_count)
        done = ch_count if ready else 0
        st = "DONE" if ready else "WAITING"
        return "Chapter Detection", st, done, tot, f"({done}/{tot} ch)"

    if st_num == 5:
        s = stages.get("stage5", {})
        tot = s.get("total_expected") or total_pages or 0
        done = min(s.get("json_pages", 0), s.get("png_pages", 0))
        if s.get("ready"):
            status = "DONE"
            done = tot if tot > 0 else done
        elif done > 0 or s.get("json_pages", 0) > 0 or s.get("png_pages", 0) > 0:
            status = "IN PROGRESS"
        else:
            status = "WAITING"
        return "Extract Page Layout", status, done, tot, f"({done}/{tot} p)"

    if st_num == 6:
        s = stages.get("stage6", {})
        tot = s.get("total", 0) or total_pages or 0
        completed = s.get("completed", 0)
        if s.get("ready"):
            status = "DONE"
            done = tot
        elif completed > 0:
            status = "IN PROGRESS"
            done = completed
        else:
            status = "WAITING"
            done = 0
        return "Inference Queue", status, done, tot, f"({done}/{tot} p)"

    if st_num == 7:
        s = stages.get("stage7", {})
        tot = s.get("total_expected") or total_pages or 0
        md_pages = s.get("md_pages", 0)
        if s.get("ready"):
            status = "DONE"
            done = tot
        elif md_pages > 0 or s.get("in_progress"):
            status = "IN PROGRESS"
            done = md_pages
        else:
            status = "WAITING"
            done = 0
        return "Markdown Prose (LLM)", status, done, tot, f"({done}/{tot} p)"

    if st_num == 8:
        s = stages.get("stage8", {})
        tot = s.get("total_expected") or total_pages or 0
        final_docs = s.get("final_documents", 0)
        if s.get("ready"):
            status = "DONE"
            done = tot
        elif final_docs > 0 or s.get("assets", 0) > 0:
            status = "IN PROGRESS"
            done = final_docs
        else:
            status = "WAITING"
            done = 0
        return "Crop Visual Assets", status, done, tot, f"({done}/{tot} p)"

    if st_num == 9:
        s = stages.get("stage9", {})
        tot = s.get("total", 0)
        done = s.get("loop_completed", s.get("finalized", 0))
        if s.get("ready"):
            status = "DONE"
            done = tot
        elif done > 0 or s.get("frames_ready", 0) > 0:
            status = "IN PROGRESS"
        else:
            status = "WAITING"
            done = 0
        return "Prepare Eval Frames", status, done, tot, f"({done}/{tot} ast)"

    if st_num == 10:
        s = stages.get("stage10", {})
        done_passes = s.get("done_passes", s.get("ok", 0))
        tot_passes = s.get("total_passes", s.get("total", 0))
        if s.get("ready"):
            status = "DONE"
            done = tot_passes
            tot = tot_passes
        elif done_passes > 0:
            status = "IN PROGRESS"
            done = done_passes
            tot = tot_passes
        else:
            status = "WAITING"
            done = 0
            tot = tot_passes
        cnt = f"({done}/{tot} passes)"
        return "Crop Quality Eval (LLM)", status, done, tot, cnt

    if st_num == 11:
        s = stages.get("stage11", {})
        tot = s.get("total", 0)
        initialized = s.get("initialized", 0)
        if s.get("ready"):
            status = "DONE"
            done = tot
        elif initialized > 0:
            status = "IN PROGRESS"
            done = initialized
        else:
            status = "WAITING"
            done = 0
        return "Prep Conversion Queue", status, done, tot, f"({done}/{tot} ast)"

    # Stage 12: Convert Tables & Classify Images
    s12 = stages.get("stage12", {})
    s12_ready = s12.get("ready", False)
    tot_assets = s12.get("total_assets", stages.get("stage11", {}).get("total", 0))

    if st_num == 12:
        pool = s12.get("pool", tot_assets)
        classified = s12.get("classified", 0)
        tables = s12.get("tables_found", 0)
        ready = s12.get("ready", False)
        if ready or (pool > 0 and classified >= pool):
            status = "DONE"
            done = pool
            tot = pool
            cnt = f"({pool}/{pool} ast, {tables} tbl)"
        elif classified > 0:
            status = "IN PROGRESS"
            done = classified
            tot = pool
            cnt = f"({classified}/{pool} ast, {tables} tbl)"
        else:
            status = "WAITING"
            done = 0
            tot = pool
            cnt = f"(0/{pool} ast)"
        return "Convert Tables (HTML)", status, done, tot, cnt

    if st_num == 13:
        s13 = stages.get("stage13", {})
        tbl_tot = s13.get("total", 0)
        evaluated = s13.get("evaluated", 0)
        reduced = s13.get("reduced", 0)
        ready = s13.get("ready", False)
        if ready or (tbl_tot > 0 and evaluated >= tbl_tot):
            status = "DONE"
            done = tbl_tot
            tot = tbl_tot
            cnt = f"({tbl_tot}/{tbl_tot} tbl, {reduced} red)"
        elif tbl_tot > 0:
            status = "IN PROGRESS" if evaluated > 0 else "WAITING"
            done = evaluated
            tot = tbl_tot
            cnt = f"({evaluated}/{tbl_tot} tbl, {reduced} red)" if reduced > 0 else f"({evaluated}/{tbl_tot} tbl)"
        else:
            if s12_ready:
                status = "DONE"
                done, tot = 0, 0
                cnt = "(0 tbl)"
            else:
                status = "WAITING"
                done, tot = 0, 0
                cnt = "(awaiting S12)"
        return "Reduce HTML Tables", status, done, tot, cnt

    if st_num == 14:
        s14 = stages.get("stage14", {})
        img_tot = s14.get("total", 0)
        conv = s14.get("converted", 0)
        ready = s14.get("ready", False)
        if ready or (img_tot > 0 and conv >= img_tot):
            status = "DONE"
            done = img_tot
            tot = img_tot
            cnt = f"({img_tot}/{img_tot} img)"
        elif img_tot > 0:
            status = "IN PROGRESS" if conv > 0 else "WAITING"
            done = conv
            tot = img_tot
            cnt = f"({conv}/{img_tot} img)"
        else:
            if s12_ready:
                status = "DONE"
                done, tot = 0, 0
                cnt = "(0 img)"
            else:
                status = "WAITING"
                done, tot = 0, 0
                cnt = "(awaiting S12)"
        return "Convert Images & RAG", status, done, tot, cnt

    if st_num == 15:
        s15 = stages.get("stage15", {})
        tot = s15.get("expected") or total_pages or 0
        rendered = s15.get("rendered", 0)
        if s15.get("ready"):
            status = "DONE"
            done = tot
            cnt = f"({tot}/{tot} p)"
        elif rendered > 0:
            status = "IN PROGRESS"
            done = rendered
            cnt = f"({rendered}/{tot} p)"
        else:
            status = "WAITING"
            done = 0
            cnt = f"(0/{tot} p)"
        return "Render Asset Frames", status, done, tot, cnt

    if st_num == 16:
        s16 = stages.get("stage16", {})
        tot = s16.get("total_expected") or total_pages or 0
        pages = s16.get("embedded_pages", 0)
        if s16.get("ready"):
            status = "DONE"
            done = tot
            cnt = f"({tot}/{tot} p)"
        elif pages > 0:
            status = "IN PROGRESS"
            done = pages
            cnt = f"({pages}/{tot} p)"
        else:
            status = "WAITING"
            done = 0
            cnt = f"(0/{tot} p)"
        return "Embed Markdown", status, done, tot, cnt

    if st_num == 17:
        s17 = stages.get("stage17", {})
        tot = s17.get("total_expected", 0)
        pages = s17.get("proofread_pages", 0)
        if s17.get("ready"):
            status = "DONE"
            done = tot
            cnt = f"({tot}/{tot} p)"
        elif pages > 0:
            status = "IN PROGRESS"
            done = pages
            cnt = f"({pages}/{tot} p)"
        else:
            status = "WAITING"
            done = 0
            cnt = f"(0/{tot} p)"
        return "Proofread Page", status, done, tot, cnt

    if st_num == 18:
        s18 = stages.get("stage18", {})
        tot = s18.get("expected_count", 0)
        drafts = s18.get("draft_count", 0)
        if s18.get("ready"):
            status = "DONE"
            done = tot
            cnt = f"({tot}/{tot} ch)"
        elif drafts > 0:
            status = "IN PROGRESS"
            done = drafts
            cnt = f"({drafts}/{tot} ch)"
        else:
            status = "WAITING"
            done = 0
            cnt = f"(0/{tot} ch)"
        return "Prepare Chapters", status, done, tot, cnt

    if st_num == 19:
        s19 = stages.get("stage19", {})
        tot = s19.get("expected_count", 0)
        assembled = s19.get("assembled_count", 0)
        if s19.get("ready"):
            status = "DONE"
            done = tot
            cnt = f"({tot}/{tot} ch)"
        elif assembled > 0:
            status = "IN PROGRESS"
            done = assembled
            cnt = f"({assembled}/{tot} ch)"
        else:
            status = "WAITING"
            done = 0
            cnt = f"(0/{tot} ch)"
        return "Merge Final Chapters", status, done, tot, cnt

    return f"Stage {st_num}", "WAITING", 0, 0, "(0/0)"


def _print_diagnostic_details(stages: Dict[str, Any]) -> None:
    """Print diagnostic issues and missing artifacts if requested."""
    has_issues = False
    print(f"\n{ANSI_BOLD}--- DIAGNOSTIC DETAILS & ISSUES ---{ANSI_RESET}")
    for st_key, st_val in sorted(stages.items()):
        if not isinstance(st_val, dict):
            continue
        missing = st_val.get("missing", [])
        if missing:
            has_issues = True
            st_name = st_val.get("name", st_key)
            print(f"[{st_name}]:")
            for m in missing:
                print(f"  [!] {m}")
        if st_val.get("missing_final_docs"):
            has_issues = True
            mfd = st_val["missing_final_docs"]
            sample = ", ".join(mfd[:5])
            more = f" ... and {len(mfd)-5} more" if len(mfd) > 5 else ""
            print(f"[{st_val.get('name', st_key)}]: Missing {len(mfd)} final doc(s): {sample}{more}")
        if st_val.get("missing_assets"):
            has_issues = True
            ma = st_val["missing_assets"]
            print(f"[{st_val.get('name', st_key)}]: Missing {len(ma)} referenced asset image(s)")
    if not has_issues:
        print("  No blocking issues found.")
    print()


def print_status_report(status: Dict[str, Any], show_details: bool = False) -> None:
    """Print an eye-friendly, colored columnar dashboard of pipeline status with progress bars."""
    manual_name = status.get("manual_dir", "Unknown Manual")
    stages = status.get("stages", {})
    total_pages = status.get("total_pdf_pages") or 0

    try:
        "┌─┬┐├┼┤└┴┘│".encode(sys.stdout.encoding or "utf-8")
        box_tl, box_tr, box_bl, box_br = "┌", "┐", "└", "┘"
        box_h, box_v = "─", "│"
        box_t, box_b, box_cross = "┬", "┴", "┼"
        box_l, box_r = "├", "┤"
    except Exception:
        box_tl, box_tr, box_bl, box_br = "+", "+", "+", "+"
        box_h, box_v = "-", "|"
        box_t, box_b, box_cross = "+", "+", "+"
        box_l, box_r = "+", "+"

    w_stg, w_nm, w_st, w_prog = 9, 27, 13, 48
    tbl_inner_w = w_stg + w_nm + w_st + w_prog + 9

    print(f"\n" + "=" * tbl_inner_w)
    print(f" PIPELINE STATUS: {ANSI_BOLD}{manual_name}{ANSI_RESET}")
    print("=" * tbl_inner_w)

    # Top border
    print(box_tl + (box_h * (w_stg + 2)) + box_t + (box_h * (w_nm + 2)) + box_t + (box_h * (w_st + 2)) + box_t + (box_h * (w_prog + 2)) + box_tr)
    # Header row
    hdr_stg = _pad_cell(f"{ANSI_BOLD}Stage{ANSI_RESET}", w_stg)
    hdr_nm = _pad_cell(f"{ANSI_BOLD}Name{ANSI_RESET}", w_nm)
    hdr_st = _pad_cell(f"{ANSI_BOLD}Status{ANSI_RESET}", w_st)
    hdr_prog = _pad_cell(f"{ANSI_BOLD}Progress{ANSI_RESET}", w_prog)
    print(f"{box_v} {hdr_stg} {box_v} {hdr_nm} {box_v} {hdr_st} {box_v} {hdr_prog} {box_v}")
    # Header divider
    print(box_l + (box_h * (w_stg + 2)) + box_cross + (box_h * (w_nm + 2)) + box_cross + (box_h * (w_st + 2)) + box_cross + (box_h * (w_prog + 2)) + box_r)

    for st_num in range(2, 20):
        s_lbl = f"Stage {st_num}"
        name, st, done, tot, cnt_lbl = _get_stage_row_info(st_num, stages, total_pages)
        color = ANSI_GREEN if st == "DONE" else (ANSI_YELLOW if st == "IN PROGRESS" else ANSI_RED)
        st_colored = f"{color}{st}{ANSI_RESET}"
        b = _make_progress_bar(done, tot, width=18, status=st, count_label=cnt_lbl)

        cell_stg = _pad_cell(s_lbl, w_stg)
        cell_nm = _pad_cell(name, w_nm)
        cell_st = _pad_cell(st_colored, w_st)
        cell_prog = _pad_cell(b, w_prog)
        print(f"{box_v} {cell_stg} {box_v} {cell_nm} {box_v} {cell_st} {box_v} {cell_prog} {box_v}")

    # Bottom border
    print(box_bl + (box_h * (w_stg + 2)) + box_b + (box_h * (w_nm + 2)) + box_b + (box_h * (w_st + 2)) + box_b + (box_h * (w_prog + 2)) + box_br + "\n")

    if show_details:
        _print_diagnostic_details(stages)





def run_command_live(cmd: List[str]) -> bool:

    """Execute command displaying output in real time."""

    print(f"\n>> Executing: {' '.join(cmd)}")

    result = subprocess.run(cmd)

    return result.returncode == 0





def to_relative_posix(target_path: Path, base_dir: Optional[Path] = None) -> str:
    """Format path relative to base_dir (or cwd) using forward slashes."""
    t = target_path.resolve()
    base = (base_dir or Path.cwd()).resolve()
    try:
        return str(t.relative_to(base)).replace("\\", "/")
    except ValueError:
        try:
            return str(t.relative_to(Path.cwd().resolve())).replace("\\", "/")
        except ValueError:
            return str(target_path.name)


def parse_page_range(range_str: str) -> Set[int]:
    """Parse range string like '1-10', '73', '1,3,5-7'."""
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


def format_page_range(pages: Set[int]) -> str:
    """Format a set of integers into compact range string."""
    if not pages:
        return "none"
    sorted_p = sorted(list(pages))
    ranges: List[str] = []
    i = 0
    while i < len(sorted_p):
        start = sorted_p[i]
        while i + 1 < len(sorted_p) and sorted_p[i + 1] == sorted_p[i] + 1:
            i += 1
        end = sorted_p[i]
        if start == end:
            ranges.append(str(start))
        else:
            ranges.append(f"{start}-{end}")
        i += 1
    return ",".join(ranges)


# Pipeline Stage Metadata Registry (Stages 1 through 19)
STAGE_REGISTRY: Dict[int, Dict[str, Any]] = {
    1: {
        "name": "Initialize Environment",
        "type": "deterministic",
        "skill": "pdf-stage1-initialize",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage1-initialize/scripts/check_env.py",
        "description": "Verifies Python runtime and libraries (PyMuPDF, Pillow, pytesseract).",
        "supports_pages": False,
    },
    2: {
        "name": "Download Documentation",
        "type": "deterministic",
        "skill": "pdf-stage2-download",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage2-download/scripts/download_documentation.ps1",
        "description": "Downloads source technical PDF manuals from archival mirrors.",
        "supports_pages": False,
    },
    3: {
        "name": "Searchable OCR Layer",
        "type": "deterministic",
        "skill": "pdf-stage3-ocr",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage3-ocr/scripts/add_ocr_layer.py",
        "description": "Injects searchable OCR text layer into scanned PDF document.",
        "supports_pages": False,
    },
    4: {
        "name": "Detect Chapters & TOC",
        "type": "deterministic",
        "skill": "pdf-stage4-find-chapters",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage4-find-chapters/scripts/find_chapters.py",
        "description": "Extracts document bookmarks and chapter boundaries to <stem>_chapters.json.",
        "supports_pages": False,
    },
    5: {
        "name": "Extract Layout & Previews",
        "type": "deterministic",
        "skill": "pdf-stage5-extract-layout",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage5-extract-layout/scripts/stage5_extract_layout.py",
        "description": "Extracts per-page text layout blocks (JSON) and rendered preview images (PNG).",
        "supports_pages": True,
    },
    6: {
        "name": "Prepare Inference Queue",
        "type": "deterministic",
        "skill": "pdf-stage6-prepare-queue",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage6-prepare-queue/scripts/stage6_prepare_queue.py",
        "description": "Generates <stem>_queue.json to track transcription status for each page.",
        "supports_pages": True,
    },
    7: {
        "name": "Infer Markdown Prose",
        "type": "inferential",
        "skill": "pdf-stage7-infer-markdown",
        "prompt": ".agents/plugins/pdf-pipeline/skills/pdf-stage7-infer-markdown/prompt.md",
        "description": "Delegated to stage7-worker subagents via stage7-transcription-lead to transcribe layout JSON and PNG into page_XXXX.md with <crop> tags in chunks of <= 5 pages.",
        "supports_pages": True,
    },
    8: {
        "name": "Crop Visual Assets",
        "type": "deterministic",
        "skill": "pdf-stage8-crop-assets",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py",
        "description": "Cuts high-resolution PNG crops, generates page_XXXX-images.md, and bootstraps assets queue.",
        "supports_pages": True,
    },
    9: {
        "name": "Prepare Eval Frames & Recrop",
        "type": "deterministic",
        "skill": "pdf-stage9-prepare-eval-clip",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py",
        "description": "Generates Red/Blue bounding box evaluation frames (eval_frames/) and manages recrops.",
        "supports_pages": True,
    },
    10: {
        "name": "Evaluate & Reclip Assets",
        "type": "inferential",
        "skill": "pdf-stage10-eval-clip",
        "prompt": ".agents/plugins/pdf-pipeline/skills/pdf-stage10-eval-clip/prompt.md",
        "description": "Governed by stage10-eval-lead coordinating Stage 9 CLI and stage10-worker subagents in bounded chunks of <= 10 assets in a loop until confirmed ok.",
        "supports_pages": True,
    },
    11: {
        "name": "Prepare Conversion Queue",
        "type": "deterministic",
        "skill": "pdf-stage11-prepare-convert",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage11-prepare-convert/scripts/stage11_prepare_convert.py",
        "description": "Upgrades assets queue schema with conversion status, fragment paths, and counters.",
        "supports_pages": False,
    },
    12: {
        "name": "Convert Tables & Classify Images",
        "type": "inferential",
        "skill": "pdf-stage12-convert-table-html",
        "prompt": ".agents/plugins/pdf-pipeline/skills/pdf-stage12-convert-table-html/prompt.md",
        "description": "Delegated to stage12-worker subagents via stage12-table-html-lead to convert all tables into HTML tables and classify remaining visual assets directly as images in chunks of <= 10 assets.",
        "supports_pages": True,
    },
    13: {
        "name": "Reduce HTML Tables",
        "type": "inferential",
        "skill": "pdf-stage13-reduce-table-html",
        "prompt": ".agents/plugins/pdf-pipeline/skills/pdf-stage13-reduce-table-html/prompt.md",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage13-reduce-table-html/scripts/stage13_reduce_table_html.py",
        "description": "Deterministic gatekeeper filters HTML tables; reduction candidates are delegated to stage13-worker subagents via stage13-reduction-lead in chunks of <= 10 tables.",
        "supports_pages": True,
    },
    14: {
        "name": "Convert Images & RAG Text",
        "type": "inferential",
        "skill": "pdf-stage14-convert-image",
        "prompt": ".agents/plugins/pdf-pipeline/skills/pdf-stage14-convert-image/prompt.md",
        "description": "Delegated to stage14-worker subagents via stage14-image-lead to create image link + collapsed breakdown (.md) and plain text (.txt) for RAG in chunks of <= 10 images.",
        "supports_pages": True,
    },
    15: {
        "name": "Render Asset Frames",
        "type": "deterministic",
        "skill": "pdf-stage15-render-asset-frames",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage15-render-asset-frames/scripts/stage15_render_asset_frames.py",
        "description": "Renders full-page review frames (build/01_page_layout/asset_frames/page_XXXX_asset_frame.png) with MD, HTML, IMG tags.",
        "supports_pages": True,
    },
    16: {
        "name": "Embed Markdown",
        "type": "deterministic",
        "skill": "pdf-stage16-embed",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage16-embed/scripts/stage16_embed.py",
        "description": "Injects converted asset fragments into page_XXXX-embed.md.",
        "supports_pages": True,
    },
    17: {
        "name": "Proofread Page",
        "type": "inferential",
        "skill": "pdf-stage17-proofread-page",
        "prompt": ".agents/plugins/pdf-pipeline/skills/pdf-stage17-proofread-page/prompt.md",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage17-proofread-page/scripts/stage17_proofread_page.py",
        "description": "Delegated to stage17-worker subagents via stage17-proofread-lead to perform semantic proofreading and retrocomputing syntax normalization in chunks of <= 10 pages.",
        "supports_pages": True,
    },
    18: {
        "name": "Prepare Chapters",
        "type": "deterministic",
        "skill": "pdf-stage18-prepare-chapters",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage18-prepare-chapters/scripts/stage18_prepare_chapters.py",
        "description": "Deterministically bundles page Markdown into chapter drafts (build/02_detect_cont_chapters/<slug>.md) with <continuation-marker>.",
        "supports_pages": True,
    },
    19: {
        "name": "Merge Final Chapters",
        "type": "inferential",
        "skill": "pdf-stage19-merge-chapters",
        "prompt": ".agents/plugins/pdf-pipeline/skills/pdf-stage19-merge-chapters/prompt.md",
        "script": ".agents/plugins/pdf-pipeline/skills/pdf-stage19-merge-chapters/scripts/stage19_merge_chapters.py",
        "description": "Delegated chapter-by-chapter to stage19-worker subagents via stage19-chapter-merge-lead to resolve continuation markers, fuse tables, and consolidate footnotes.",
        "supports_pages": True,
    },
}


def sanitize_filename(name: str) -> str:
    """Sanitize filename by stripping illegal filesystem characters while preserving spaces and hyphens."""
    clean = re.sub(r'[<>:"/\\|?*]', "", name)
    clean = re.sub(r"\s+", " ", clean).strip()
    return clean


def get_chapter_filename(ch: Dict[str, Any], ch_num: int) -> str:
    """Determine clean chapter filename honoring 'filename', 'clean_title', or fallback."""
    if ch.get("filename"):
        raw_name = ch["filename"].strip()
        if not raw_name.lower().endswith(".md"):
            raw_name = f"{raw_name}.md"
        return sanitize_filename(raw_name)

    clean_title = ch.get("clean_title") or ch.get("title", f"Chapter {ch_num}")
    return sanitize_filename(f"{ch_num:02d} - {clean_title}.md")


def inspect_page_status(
    manual_dir: Path,
    source_pdf: Optional[Path],
    ocr_pdf: Optional[Path],
    layout_dir: Path,
    queue_file: Optional[Path],
    assets_queue_file: Optional[Path],
    pages: Optional[Set[int]] = None,
) -> Dict[int, Dict[str, Any]]:
    """Inspect state of target pages across stages 1 through 19."""
    effective_pdf = ocr_pdf or source_pdf
    res: Dict[int, Dict[str, Any]] = {}

    queue_data: Dict[str, Any] = {}
    if queue_file and queue_file.is_file():
        try:
            with open(queue_file, "r", encoding="utf-8") as f:
                queue_data = json.load(f)
        except Exception:
            pass

    page_status_map: Dict[int, str] = {}
    for item in queue_data.get("queue", []):
        p_num = item.get("page_number") or item.get("page")
        if p_num is not None:
            page_status_map[p_num] = item.get("status", "pending")

    assets_data: Dict[str, Any] = {}
    if assets_queue_file and assets_queue_file.is_file():
        try:
            with open(assets_queue_file, "r", encoding="utf-8") as f:
                assets_data = json.load(f)
        except Exception:
            pass

    assets_by_page: Dict[int, List[Dict[str, Any]]] = {}
    for it in assets_data.get("queue", []):
        p_num = it.get("page_number") or it.get("page")
        if p_num is not None:
            assets_by_page.setdefault(p_num, []).append(it)

    if pages:
        target_pages_list = sorted(list(pages))
    else:
        all_pages: Set[int] = set()
        if queue_data.get("queue"):
            for item in queue_data["queue"]:
                pn = item.get("page_number") or item.get("page")
                if isinstance(pn, int):
                    all_pages.add(pn)
        if not all_pages and effective_pdf and effective_pdf.is_file():
            try:
                import fitz  # type: ignore
                doc = fitz.open(effective_pdf)
                all_pages = set(range(1, doc.page_count + 1))
                doc.close()
            except Exception:
                pass
        if not all_pages and layout_dir.is_dir():
            for f in layout_dir.glob("page_*.json"):
                m = re.match(r"^page_(\d+)\.json$", f.name)
                if m:
                    all_pages.add(int(m.group(1)))
        target_pages_list = sorted(list(all_pages)) if all_pages else [1]

    for st_num in range(1, 20):
        if st_num not in STAGE_REGISTRY:
            continue
        info = STAGE_REGISTRY[st_num]
        st_type = info["type"]

        stage_res: Dict[str, Any] = {
            "name": info["name"],
            "type": st_type,
            "ready": False,
            "details": "",
            "pending_count": 0,
            "completed_count": 0,
            "pending_items": [],
        }

        if st_num == 1:
            try:
                import fitz  # type: ignore
                from PIL import Image  # type: ignore
                import pytesseract  # type: ignore
                stage_res["ready"] = True
                stage_res["details"] = "Environment OK (PyMuPDF, Pillow, pytesseract available)"
            except ImportError as e:
                stage_res["ready"] = False
                stage_res["details"] = f"Missing dependency: {e}"

        elif st_num == 2:
            s2_ok = bool(source_pdf and source_pdf.is_file() and source_pdf.stat().st_size > 0)
            stage_res["ready"] = s2_ok
            stage_res["details"] = f"PDF: {source_pdf.name if source_pdf else 'Missing'}"

        elif st_num == 3:
            s3_ok = bool(ocr_pdf and ocr_pdf.is_file() and ocr_pdf.stat().st_size > 0)
            stage_res["ready"] = s3_ok
            stage_res["details"] = f"OCR PDF: {ocr_pdf.name if ocr_pdf else 'None (raw fallback)'}"

        elif st_num == 4:
            stem = effective_pdf.stem if effective_pdf else manual_dir.name
            cand_ch = find_chapters_file(manual_dir, stem)
            s4_ok = bool(cand_ch and cand_ch.is_file() and cand_ch.stat().st_size > 0)
            ch_count = 0
            if s4_ok:
                try:
                    with open(cand_ch, "r", encoding="utf-8") as f:
                        ch_data = json.load(f)
                    ch_count = len(ch_data.get("chapters", []))
                except Exception:
                    ch_count = 1
            stage_res["completed_count"] = ch_count if s4_ok else 0
            stage_res["pending_count"] = 0 if s4_ok else 1
            stage_res["total_chapters"] = ch_count
            stage_res["ready"] = s4_ok
            stage_res["details"] = f"Chapters Map: {cand_ch.name if s4_ok else 'Missing'}"

        elif st_num == 5:
            ok_cnt = 0
            missing_pages = []
            for p in target_pages_list:
                jf = layout_dir / f"page_{p:04d}.json"
                pf = layout_dir / f"page_{p:04d}.png"
                if jf.is_file() and jf.stat().st_size > 0 and pf.is_file() and pf.stat().st_size > 0:
                    ok_cnt += 1
                else:
                    missing_pages.append(p)
            stage_res["completed_count"] = ok_cnt
            stage_res["pending_count"] = len(missing_pages)
            stage_res["pending_items"] = missing_pages
            stage_res["ready"] = (len(missing_pages) == 0)
            stage_res["details"] = f"{ok_cnt}/{len(target_pages_list)} layout files ready (JSON+PNG)"

        elif st_num == 6:
            s6_ok = bool(queue_file and queue_file.is_file() and queue_file.stat().st_size > 0)
            q_items = queue_data.get("queue", [])
            q_tot = len(q_items) if q_items else len(target_pages_list)
            q_comp = sum(1 for item in q_items if item.get("status") in ("completed", "done")) if q_items else (q_tot if s6_ok else 0)
            stage_res["completed_count"] = q_comp
            stage_res["pending_count"] = max(0, q_tot - q_comp)
            stage_res["ready"] = s6_ok and (q_comp == q_tot)
            stage_res["details"] = f"Queue: {queue_file.name if s6_ok else 'Missing'}"

        elif st_num == 7:
            ok_cnt = 0
            pending_p = []
            for p in target_pages_list:
                mf = layout_dir / f"page_{p:04d}.md"
                q_st = page_status_map.get(p, "pending")
                if mf.is_file() and mf.stat().st_size > 0 and q_st in ("completed", "done"):
                    ok_cnt += 1
                else:
                    pending_p.append(p)
            stage_res["completed_count"] = ok_cnt
            stage_res["pending_count"] = len(pending_p)
            stage_res["pending_items"] = pending_p
            stage_res["ready"] = (len(pending_p) == 0)
            stage_res["details"] = f"{ok_cnt}/{len(target_pages_list)} pages transcribed (In-Session LLM)"

        elif st_num == 8:
            ok_cnt = 0
            pending_p = []
            for p in target_pages_list:
                imf = layout_dir / f"page_{p:04d}-images.md"
                if imf.is_file() and imf.stat().st_size > 0:
                    ok_cnt += 1
                else:
                    pending_p.append(p)
            has_aq = bool(assets_queue_file and assets_queue_file.is_file())
            stage_res["completed_count"] = ok_cnt
            stage_res["pending_count"] = len(pending_p)
            stage_res["pending_items"] = pending_p
            stage_res["ready"] = (len(pending_p) == 0 and has_aq)
            stage_res["details"] = f"{ok_cnt}/{len(target_pages_list)} pages cropped with -images.md"

        elif st_num == 9:
            target_assets = []
            for p in target_pages_list:
                target_assets.extend(assets_by_page.get(p, []))
            tot_a = len(target_assets)
            finalized = sum(1 for it in target_assets if it.get("final_asset") is not None)
            frames_ready = sum(1 for it in target_assets if it.get("eval_frame") is not None)
            stage_res["completed_count"] = finalized
            stage_res["pending_count"] = tot_a - finalized
            s8_ready = bool(res.get(8, {}).get("ready"))
            stage_res["ready"] = s8_ready and (tot_a == 0 or finalized == tot_a)
            stage_res["details"] = f"{finalized}/{tot_a} assets locked into _clip_final.png ({frames_ready} frames)"

        elif st_num == 10:
            target_assets = []
            for p in target_pages_list:
                target_assets.extend(assets_by_page.get(p, []))
            tot_a = len(target_assets)
            ok_cnt = sum(1 for it in target_assets if it.get("eval_clip") == "ok")
            pending_items = [it.get("asset_id") for it in target_assets if it.get("eval_clip") != "ok"]
            stage_res["completed_count"] = ok_cnt
            stage_res["pending_count"] = len(pending_items)
            stage_res["pending_items"] = pending_items
            s8_ready = bool(res.get(8, {}).get("ready"))
            stage_res["ready"] = s8_ready and (tot_a == 0 or ok_cnt == tot_a)
            stage_res["details"] = f"{ok_cnt}/{tot_a} assets confirmed 'ok' (In-Session LLM)"

        elif st_num == 11:
            target_assets = []
            for p in target_pages_list:
                target_assets.extend(assets_by_page.get(p, []))
            tot_a = len(target_assets)
            initialized = sum(1 for it in target_assets if "conversion_status" in it)
            stage_res["completed_count"] = initialized
            stage_res["pending_count"] = tot_a - initialized
            stage_res["total_assets"] = tot_a
            stage_res["ready"] = (tot_a == 0 or initialized == tot_a)
            stage_res["details"] = f"{initialized}/{tot_a} assets initialized with Stage 11 schema"

        elif st_num == 12:
            target_assets = []
            for p in target_pages_list:
                target_assets.extend(assets_by_page.get(p, []))
            tot_a = len(target_assets)
            converted = 0
            pending_ids = []
            html_cnt = 0
            img_cnt = 0
            for it in target_assets:
                aid = it.get("asset_id")
                dtype = it.get("detected_type")
                c_status = it.get("conversion_status")
                c_stage = it.get("conversion_stage")
                if dtype == "table_html":
                    hf = layout_dir / "assets" / f"{aid}_html.md"
                    if c_status in ("completed", "converted") and hf.is_file() and hf.stat().st_size > 0:
                        converted += 1
                        html_cnt += 1
                    else:
                        pending_ids.append(aid)
                elif dtype == "image" or c_status in ("completed", "converted") or dtype in ("table_markdown", "md") or c_stage in (13, 14):
                    converted += 1
                    if dtype == "image":
                        img_cnt += 1
                else:
                    pending_ids.append(aid)
            stage_res["completed_count"] = converted
            stage_res["pending_count"] = len(pending_ids)
            stage_res["pending_items"] = pending_ids
            stage_res["total_assets"] = tot_a
            stage_res["converted_tables"] = html_cnt
            stage_res["classified_images"] = img_cnt
            stage_res["ready"] = (tot_a == 0 or converted == tot_a)
            if pending_ids:
                stage_res["details"] = f"{html_cnt} HTML tables converted, {img_cnt} images classified ({len(pending_ids)} pending)"
            else:
                stage_res["details"] = f"{html_cnt} HTML tables converted, {img_cnt} visual assets classified as images"

        elif st_num == 13:
            s12_ready = bool(res.get(12, {}).get("ready"))
            html_tables = [it for p in target_pages_list for it in assets_by_page.get(p, []) if it.get("detected_type") == "table_html"]
            tot_t = len(html_tables)
            evaluated = sum(1 for it in html_tables if it.get("reduced_to_markdown") is not None)
            reduced = sum(1 for it in html_tables if it.get("reduced_to_markdown") is True)
            pending_ids = [it.get("asset_id") for it in html_tables if it.get("reduced_to_markdown") is None]
            stage_res["completed_count"] = evaluated
            stage_res["pending_count"] = len(pending_ids)
            stage_res["pending_items"] = pending_ids
            stage_res["total_tables"] = tot_t
            stage_res["ready"] = s12_ready and (tot_t == 0 or evaluated == tot_t)
            stage_res["details"] = f"{evaluated}/{tot_t} HTML tables evaluated ({reduced} reduced to MD, {len(pending_ids)} pending Agent reduction)"

        elif st_num == 14:
            s12_ready = bool(res.get(12, {}).get("ready"))
            images = [it for p in target_pages_list for it in assets_by_page.get(p, []) if it.get("detected_type") == "image"]
            tot_img = len(images)
            converted = 0
            pending_ids = []
            for it in images:
                aid = it.get("asset_id")
                mf = layout_dir / "assets" / f"{aid}.md"
                tf = layout_dir / "assets" / f"{aid}.txt"
                if mf.is_file() and mf.stat().st_size > 0 and tf.is_file() and tf.stat().st_size > 0 and it.get("conversion_status") in ("completed", "converted"):
                    converted += 1
                else:
                    pending_ids.append(aid)
            stage_res["completed_count"] = converted
            stage_res["pending_count"] = len(pending_ids)
            stage_res["pending_items"] = pending_ids
            stage_res["total_images"] = tot_img
            stage_res["ready"] = s12_ready and (tot_img == 0 or converted == tot_img)
            stage_res["details"] = f"{converted}/{tot_img} images converted (In-Session LLM)"

        elif st_num == 15:
            asset_frames_dir = layout_dir / "asset_frames"
            ok_cnt = 0
            pending_p = []
            for p in target_pages_list:
                aff = asset_frames_dir / f"page_{p:04d}_asset_frame.png"
                if aff.is_file() and aff.stat().st_size > 0:
                    ok_cnt += 1
                else:
                    pending_p.append(p)
            stage_res["completed_count"] = ok_cnt
            stage_res["pending_count"] = len(pending_p)
            stage_res["pending_items"] = pending_p
            stage_res["ready"] = (len(pending_p) == 0)
            stage_res["details"] = f"{ok_cnt}/{len(target_pages_list)} classified asset review frames rendered"

        elif st_num == 16:
            ok_cnt = 0
            pending_p = []
            for p in target_pages_list:
                emf = layout_dir / f"page_{p:04d}-embed.md"
                if emf.is_file() and emf.stat().st_size > 0:
                    ok_cnt += 1
                else:
                    pending_p.append(p)
            stage_res["completed_count"] = ok_cnt
            stage_res["pending_count"] = len(pending_p)
            stage_res["pending_items"] = pending_p
            stage_res["ready"] = (len(pending_p) == 0)
            stage_res["details"] = f"{ok_cnt}/{len(target_pages_list)} pages embedded into page_XXXX-embed.md"

        elif st_num == 17:
            ok_cnt = 0
            pending_p = []
            for p in target_pages_list:
                pmf = layout_dir / f"page_{p:04d}-proofread.md"
                if pmf.is_file() and pmf.stat().st_size > 0:
                    ok_cnt += 1
                else:
                    pending_p.append(p)
            stage_res["completed_count"] = ok_cnt
            stage_res["pending_count"] = len(pending_p)
            stage_res["pending_items"] = pending_p
            stage_res["ready"] = (len(pending_p) == 0)
            stage_res["details"] = f"{ok_cnt}/{len(target_pages_list)} pages proofread into page_XXXX-proofread.md"

        elif st_num == 18:
            draft_dir = manual_dir / "build" / "02_detect_cont_chapters"
            cand_ch = find_chapters_file(manual_dir, stem)

            expected_ch = 0
            ch_list: List[Dict[str, Any]] = []
            if cand_ch and cand_ch.exists():
                try:
                    with open(cand_ch, "r", encoding="utf-8") as f:
                        ch_data = json.load(f)
                    ch_list = ch_data.get("chapters", [])
                    expected_ch = len(ch_list)
                except Exception:
                    pass

            target_ch_indices = []
            if cand_ch and cand_ch.exists() and pages:
                tot_p = ch_data.get("total_pages", 1000)
                for idx, ch in enumerate(ch_list):
                    sp = ch.get("start_page", 1)
                    np = ch_list[idx + 1].get("start_page", tot_p + 1) if (idx + 1) < len(ch_list) else (tot_p + 1)
                    ep = np - 1
                    if set(range(sp, ep + 1)).intersection(pages):
                        target_ch_indices.append(idx + 1)
            else:
                target_ch_indices = list(range(1, expected_ch + 1))

            prepared_count = 0
            pending_chapters = []
            for ch_idx in target_ch_indices:
                expected_fn = None
                if 0 <= (ch_idx - 1) < len(ch_list):
                    expected_fn = get_chapter_filename(ch_list[ch_idx - 1], ch_idx)

                found = []
                if draft_dir.exists():
                    if expected_fn and (draft_dir / expected_fn).is_file():
                        found = [draft_dir / expected_fn]
                    else:
                        found = [f for f in sorted(draft_dir.glob(f"{ch_idx:02d}*.md")) if f.is_file()]

                if found and found[0].stat().st_size > 0:
                    prepared_count += 1
                else:
                    pending_chapters.append(ch_idx)

            stage_res["completed_count"] = prepared_count
            stage_res["pending_count"] = len(pending_chapters)
            stage_res["pending_items"] = pending_chapters
            stage_res["total_chapters"] = len(target_ch_indices)
            stage_res["ready"] = (len(target_ch_indices) > 0 and prepared_count == len(target_ch_indices))
            stage_res["details"] = f"{prepared_count}/{len(target_ch_indices)} chapter drafts prepared in build/02_detect_cont_chapters/"

        elif st_num == 19:
            draft_dir = manual_dir / "build" / "02_detect_cont_chapters"
            final_dir = manual_dir / "build" / "02_final_chapters"
            cand_ch = find_chapters_file(manual_dir, stem)

            expected_ch = 0
            ch_list: List[Dict[str, Any]] = []
            if cand_ch and cand_ch.exists():
                try:
                    with open(cand_ch, "r", encoding="utf-8") as f:
                        ch_data = json.load(f)
                    ch_list = ch_data.get("chapters", [])
                    expected_ch = len(ch_list)
                except Exception:
                    pass

            target_ch_indices = []
            if cand_ch and cand_ch.exists() and pages:
                tot_p = ch_data.get("total_pages", 1000)
                for idx, ch in enumerate(ch_list):
                    sp = ch.get("start_page", 1)
                    np = ch_list[idx + 1].get("start_page", tot_p + 1) if (idx + 1) < len(ch_list) else (tot_p + 1)
                    ep = np - 1
                    if set(range(sp, ep + 1)).intersection(pages):
                        target_ch_indices.append(idx + 1)
            else:
                target_ch_indices = list(range(1, expected_ch + 1))

            finalized_count = 0
            pending_chapters = []
            for ch_idx in target_ch_indices:
                expected_fn = None
                if 0 <= (ch_idx - 1) < len(ch_list):
                    expected_fn = get_chapter_filename(ch_list[ch_idx - 1], ch_idx)

                found_draft = []
                if draft_dir.exists():
                    if expected_fn:
                        if (draft_dir / expected_fn).is_file():
                            found_draft = [draft_dir / expected_fn]
                    else:
                        found_draft = [f for f in sorted(draft_dir.glob(f"{ch_idx:02d}*.md")) if f.is_file()]

                draft_name = found_draft[0].name if found_draft else (expected_fn or f"{ch_idx:02d}_chapter.md")

                found_final = []
                if final_dir.exists():
                    if expected_fn:
                        if (final_dir / expected_fn).is_file():
                            found_final = [final_dir / expected_fn]
                    else:
                        found_final = [f for f in sorted(final_dir.glob(f"{ch_idx:02d}*.md")) if f.is_file()]

                if found_final and found_final[0].stat().st_size > 0:
                    try:
                        txt = found_final[0].read_text(encoding="utf-8", errors="replace")
                        if "<continuation-marker>" not in txt:
                            finalized_count += 1
                        else:
                            pending_chapters.append(draft_name)
                    except Exception:
                        pending_chapters.append(draft_name)
                else:
                    pending_chapters.append(draft_name)

            stage_res["completed_count"] = finalized_count
            stage_res["pending_count"] = len(pending_chapters)
            stage_res["pending_items"] = pending_chapters
            stage_res["total_chapters"] = len(target_ch_indices)
            stage_res["ready"] = (len(target_ch_indices) > 0 and finalized_count == len(target_ch_indices))
            stage_res["details"] = f"{finalized_count}/{len(target_ch_indices)} chapter documents finalized in build/02_final_chapters/"

        res[st_num] = stage_res

    return res


def print_execution_plan(
    manual_dir: Path,
    from_stage: int,
    to_stage: int,
    pages: Optional[Set[int]],
    stage_status: Dict[int, Dict[str, Any]],
) -> None:
    """Display the detailed execution plan mapping deterministic vs inferential steps."""
    p_str = format_page_range(pages) if pages else "all"
    print("\n" + "=" * 85)
    print(f" EXECUTION PLAN: Stages {from_stage} -> {to_stage} for '{manual_dir.name}' (Pages: {p_str})")
    print("=" * 85)

    step = 1
    for st in range(from_stage, to_stage + 1):
        if st not in STAGE_REGISTRY:
            continue
        info = STAGE_REGISTRY[st]
        st_res = stage_status.get(st, {})
        is_inferential = (info["type"] == "inferential")
        ready_tag = "[ALREADY DONE]" if st_res.get("ready") else "[NEEDS EXECUTION]"

        print(f"\nStep {step:02d}: Stage {st:02d} - {info['name']} {ready_tag}")
        print(f"  * Type:        {'INFERENTIAL (In-Session Multimodal Agent)' if is_inferential else 'DETERMINISTIC (Python CLI Script)'}")
        print(f"  * Purpose:     {info['description']}")

        if is_inferential:
            print(f"  * Skill/Prompt: {info.get('skill', '')} -> {info.get('prompt', '')}")
            print(f"  * Action:      Agent multimodal inspection using `view_file` and workspace tools.")
            if st_res.get("pending_items"):
                items = st_res["pending_items"]
                sample = ", ".join(str(x) for x in items[:8])
                more = f" ... and {len(items)-8} more" if len(items) > 8 else ""
                print(f"  * Scope:       Pending targets: {sample}{more}")
        else:
            p_arg = f" --pages {p_str}" if info["supports_pages"] and pages else ""
            print(f"  * Script:      {info['script']}")
            print(f"  * Command:     python {info['script']} \"{manual_dir.name}\"{p_arg}")

        step += 1

    print("\n" + "=" * 85 + "\n")


def resolve_stage_item_details(st: int, item: Any, layout_dir: Path) -> Dict[str, str]:
    """Resolve file input, expected output, and action for a pending item at an inferential stage."""
    res: Dict[str, str] = {"item": str(item), "input": "", "output": "", "action": ""}
    if st == 7:
        try:
            p_num = int(item)
            p_json = layout_dir / f"page_{p_num:04d}.json"
            p_png = layout_dir / f"page_{p_num:04d}.png"
            p_md = layout_dir / f"page_{p_num:04d}.md"
            res["input"] = f"{to_relative_posix(p_json)}, {to_relative_posix(p_png)}"
            res["output"] = to_relative_posix(p_md)
            res["action"] = f"Transcribe page {p_num} layout into clean Markdown prose with <crop> tags."
        except Exception:
            pass
    elif st == 10:
        aid = str(item)
        eval_frame = layout_dir / "eval_frames" / f"{aid}_eval.png"
        raw_crop = layout_dir / "assets" / f"{aid}.png"
        inp = eval_frame if eval_frame.is_file() else raw_crop
        res["input"] = to_relative_posix(inp)
        res["output"] = "<stem>_assets_queue.json"
        res["action"] = f"Inspect bounding box perimeter for {aid}. Mark 'eval_clip: ok' or assign corrected_box."
    elif st == 12:
        aid = str(item)
        final_crop = layout_dir / "assets" / f"{aid}_clip_final.png"
        raw_crop = layout_dir / "assets" / f"{aid}.png"
        inp = final_crop if final_crop.is_file() else raw_crop
        res["input"] = to_relative_posix(inp)
        res["output"] = to_relative_posix(layout_dir / "assets" / f"{aid}_html.md")
        res["action"] = f"If table, synthesize HTML table into {aid}_html.md. Otherwise classify as image for Stage 14."
    elif st == 13:
        aid = str(item)
        final_crop = layout_dir / "assets" / f"{aid}_clip_final.png"
        raw_crop = layout_dir / "assets" / f"{aid}.png"
        inp = final_crop if final_crop.is_file() else raw_crop
        res["input"] = to_relative_posix(inp)
        res["output"] = to_relative_posix(layout_dir / "assets" / f"{aid}_reduced.md")
        res["action"] = f"Transcribe visual crop into clean GFM Markdown with native LaTeX math into {aid}_reduced.md."
    elif st == 14:
        aid = str(item)
        final_crop = layout_dir / "assets" / f"{aid}_clip_final.png"
        raw_crop = layout_dir / "assets" / f"{aid}.png"
        inp = final_crop if final_crop.is_file() else raw_crop
        res["input"] = to_relative_posix(inp)
        res["output"] = f"{to_relative_posix(layout_dir / 'assets' / f'{aid}.md')}, {to_relative_posix(layout_dir / 'assets' / f'{aid}.txt')}"
        res["action"] = f"Synthesize image link with architectural breakdown (.md) and RAG plain text (.txt)."
    elif st == 17:
        p_num = int(item)
        inp = layout_dir / f"page_{p_num:04d}-embed.md"
        out = layout_dir / f"page_{p_num:04d}-proofread.md"
        res["input"] = to_relative_posix(inp)
        res["output"] = to_relative_posix(out)
        res["action"] = f"Proofread and normalize retrocomputing syntax into page_{p_num:04d}-proofread.md using sliding window context."
    elif st == 19:
        manual_dir = layout_dir.parent.parent
        draft_dir = manual_dir / "build" / "02_detect_cont_chapters"
        final_dir = manual_dir / "build" / "02_final_chapters"
        ch_name = str(item)
        if not ch_name.endswith(".md"):
            ch_name = f"{ch_name}.md"
        draft_f = draft_dir / ch_name
        final_f = final_dir / ch_name
        res["input"] = to_relative_posix(draft_f)
        res["output"] = to_relative_posix(final_f)
        res["action"] = f"Intelligently resolve <continuation-marker>, fuse multi-page tables/notes, and assemble {final_f.name}."
    return res


def execute_deterministic_slice(
    manual_dir: Path,
    source_pdf: Optional[Path],
    ocr_pdf: Optional[Path],
    from_stage: int,
    to_stage: int,
    pages: Optional[Set[int]],
    force: bool = False,
    dpi: int = 300,
    chunk_size: int = 1,
) -> bool:
    """Execute deterministic stages in sequence, yielding cleanly at inferential stages."""
    python_cmd = sys.executable
    effective_pdf = ocr_pdf or source_pdf
    pdf_rel = str(effective_pdf.relative_to(Path.cwd())).replace("\\", "/") if effective_pdf and effective_pdf.is_relative_to(Path.cwd()) else (str(effective_pdf) if effective_pdf else "")
    manual_rel = str(manual_dir.relative_to(Path.cwd())).replace("\\", "/") if manual_dir.is_relative_to(Path.cwd()) else str(manual_dir)
    stem = effective_pdf.stem if effective_pdf else manual_dir.name
    p_str = format_page_range(pages) if pages else None

    for st in range(from_stage, to_stage + 1):
        if st not in STAGE_REGISTRY:
            continue
        info = STAGE_REGISTRY[st]
        st_type = info["type"]

        # Dynamically refresh PDF references in case upstream stages just generated them
        _, source_pdf, ocr_pdf = find_manual_dir_and_pdf(str(manual_dir))
        effective_pdf = ocr_pdf or source_pdf
        pdf_rel = str(effective_pdf.relative_to(Path.cwd())).replace("\\", "/") if effective_pdf and effective_pdf.is_relative_to(Path.cwd()) else (str(effective_pdf) if effective_pdf else "")
        stem = effective_pdf.stem if effective_pdf else manual_dir.name

        layout_dir = manual_dir / "build" / "01_page_layout"
        q_file = find_queue_file(manual_dir, stem) or (manual_dir / "build" / f"{stem}_queue.json")
        aq_file = find_assets_queue_file(manual_dir, stem) or (manual_dir / "build" / f"{stem}_assets_queue.json")
        st_status = inspect_page_status(manual_dir, source_pdf, ocr_pdf, layout_dir, q_file, aq_file, pages).get(st, {})

        if st_type == "inferential":
            if st == 13 and "script" in info and not st_status.get("ready"):
                cmd = [python_cmd, info["script"], manual_rel]
                if p_str:
                    cmd.extend(["--pages", p_str])
                run_command_live(cmd)
                st_status = inspect_page_status(manual_dir, source_pdf, ocr_pdf, layout_dir, q_file, aq_file, pages).get(st, {})

            if st_status.get("ready") and not force:
                print(f"\n[STAGE {st}: {info['name']}] [PASS] Already completed for target pages. Continuing...")
                continue
            else:
                pending = st_status.get("pending_items", [])
                sample = ", ".join(str(x) for x in pending[:8])
                more = f" ... and {len(pending)-8} more" if len(pending) > 8 else ""
                print("\n" + "#" * 80)
                print(f"# [INFERENTIAL HANDOFF] >>> STAGE {st}: {info['name']}")
                print(f"# Skill:      {info['skill']}")
                print(f"# Prompt:     {info.get('prompt', '')}")
                print(f"# Pages:      {p_str or 'All'}")
                print(f"# Pending:    {len(pending)} item(s): {sample}{more}")
                print(f"# Chunk Size: {chunk_size} (Strict Context Isolation: process 1 item per inspection turn)")
                print("#" * 80)
                if pending:
                    chunk_items = pending[:chunk_size]
                    print(f"# NEXT CHUNK TO PROCESS ({len(chunk_items)} of {len(pending)} remaining):")
                    for idx, itm in enumerate(chunk_items, 1):
                        det = resolve_stage_item_details(st, itm, layout_dir)
                        print(f"# [{idx}/{len(chunk_items)}] Item:   {det['item']}")
                        print(f"#     Input:  {det['input']}")
                        print(f"#     Output: {det['output']}")
                        print(f"#     Action: {det['action']}")
                    print("#" * 80)
                print(f"Execute Stage {st} in the active Agent session following {info['skill']}/SKILL.md.")
                print(f"ISOLATED CONTEXT: Inspect and transcribe each pending item individually.")
                print(f"Do NOT load multiple pages or multiple assets in parallel.")
                print(f"Once Stage {st} is complete for all targets, resume with: --from-stage {st + 1} --to-stage {to_stage}\n")
                return True

        # Deterministic stage execution
        print(f"\n=== [STAGE {st}: {info['name']}] ===")

        if st == 1:
            cmd = [python_cmd, info["script"]]
            if not run_command_live(cmd):
                sys.stderr.write("Stage 1 failed.\n")
                return False

        elif st == 2:
            if st_status.get("ready") and not force:
                print("Stage 2: Source PDF already present. Skipped.")
            else:
                cmd = ["powershell", "-ExecutionPolicy", "Bypass", "-File", info["script"], "-Manual", manual_dir.name]
                if force:
                    cmd.append("-Force")
                if not run_command_live(cmd):
                    sys.stderr.write("Stage 2 failed.\n")
                    return False

        elif st == 3:
            if st_status.get("ready") and not force:
                print("Stage 3: OCR PDF already present. Skipped.")
            else:
                src_rel = str(source_pdf.relative_to(Path.cwd())).replace("\\", "/") if source_pdf and source_pdf.is_relative_to(Path.cwd()) else (str(source_pdf) if source_pdf else "")
                cmd = [python_cmd, info["script"], src_rel]
                if force:
                    cmd.append("--force")
                if not run_command_live(cmd):
                    sys.stderr.write("Stage 3 failed.\n")
                    return False

        elif st == 4:
            if st_status.get("ready") and not force:
                print("Stage 4: Chapters map already detected. Skipped.")
            else:
                cmd = [python_cmd, info["script"], pdf_rel]
                if not run_command_live(cmd):
                    sys.stderr.write("Stage 4 failed.\n")
                    return False

        elif st == 5:
            if st_status.get("ready") and not force:
                print("Stage 5: Layout JSON and PNG already extracted. Skipped.")
            else:
                cmd = [python_cmd, info["script"], pdf_rel]
                if p_str:
                    cmd.extend(["--pages", p_str])
                if force:
                    cmd.append("--force")
                if not run_command_live(cmd):
                    sys.stderr.write("Stage 5 failed.\n")
                    return False

        elif st == 6:
            if st_status.get("ready") and not force:
                print("Stage 6: Queue already initialized. Skipped.")
            else:
                cmd = [python_cmd, info["script"], pdf_rel]
                if p_str:
                    cmd.extend(["--pages", p_str])
                if force:
                    cmd.append("--reset")
                if not run_command_live(cmd):
                    sys.stderr.write("Stage 6 failed.\n")
                    return False

        elif st == 8:
            cmd = [python_cmd, info["script"], pdf_rel, "--dpi", str(dpi)]
            if p_str:
                cmd.extend(["--pages", p_str])
            if not run_command_live(cmd):
                sys.stderr.write("Stage 8 failed.\n")
                return False

        elif st == 9:
            cmd = [python_cmd, info["script"], manual_rel, "--prep-frames"]
            if p_str:
                cmd.extend(["--pages", p_str])
            if force:
                cmd.append("--force")
            if not run_command_live(cmd):
                sys.stderr.write("Stage 9 failed.\n")
                return False

        elif st == 11:
            cmd = [python_cmd, info["script"], manual_rel]
            if force:
                cmd.append("--reset")
            if not run_command_live(cmd):
                sys.stderr.write("Stage 11 failed.\n")
                return False

        elif st == 15:
            cmd = [python_cmd, info["script"], manual_rel]
            if p_str:
                cmd.extend(["--pages", p_str])
            if force:
                cmd.append("--force")
            if not run_command_live(cmd):
                sys.stderr.write("Stage 15 failed.\n")
                return False

        elif st == 16:
            cmd = [python_cmd, info["script"], manual_rel]
            if p_str:
                cmd.extend(["--pages", p_str])
            if force:
                cmd.append("--force")
            if not run_command_live(cmd):
                sys.stderr.write("Stage 16 failed.\n")
                return False

        elif st == 18:
            cmd = [python_cmd, info["script"], manual_rel]
            if p_str:
                cmd.extend(["--pages", p_str])
            if force:
                cmd.append("--force")
            if not run_command_live(cmd):
                sys.stderr.write("Stage 18 failed.\n")
                return False

    print("\n[SUCCESS] Completed deterministic slice execution.")
    return True


def execute_pipeline(
    manual_dir: Path,
    source_pdf: Optional[Path],
    ocr_pdf: Optional[Path],
    from_stage: int,
    to_stage: int,
    pages: Optional[str] = None,
    force: bool = False,
    dpi: int = 300,
    chunk_size: int = 1,
    plan: bool = False,
) -> bool:
    """Execute stages sequentially with support for execution planning and horizontal slice runs."""
    parsed_pages = parse_page_range(pages) if pages else None
    effective_pdf = ocr_pdf or source_pdf
    stem = effective_pdf.stem if effective_pdf else manual_dir.name
    layout_dir = manual_dir / "build" / "01_page_layout"
    q_file = find_queue_file(manual_dir, stem) or (manual_dir / "build" / f"{stem}_queue.json")
    aq_file = find_assets_queue_file(manual_dir, stem) or (manual_dir / "build" / f"{stem}_assets_queue.json")

    if plan:
        from planner import generate_execution_plan, print_execution_plan_dashboard
        custom_quotas = {st: chunk_size for st in range(1, 20)} if chunk_size > 1 else None
        plan_data = generate_execution_plan(
            manual_dir=manual_dir,
            from_stage=from_stage,
            to_stage=to_stage,
            pages=parsed_pages,
            force=force,
            custom_quotas=custom_quotas,
            export_json=True,
        )
        print_execution_plan_dashboard(plan_data)
        return True

    return execute_deterministic_slice(
        manual_dir=manual_dir,
        source_pdf=source_pdf,
        ocr_pdf=ocr_pdf,
        from_stage=from_stage,
        to_stage=to_stage,
        pages=parsed_pages,
        force=force,
        dpi=dpi,
        chunk_size=chunk_size,
    )






TARGET_MANUALS: List[str] = [

    "68000 Programmer's Reference Manual",

    "68000 User's Manual",

    "A500 A2000 Technical Reference Manual",

    "Hardware Reference Manual",

    "Test Book example-4567",

]





def main() -> None:

    parser = argparse.ArgumentParser(description="Master Pipeline Workflow Runner.")

    parser.add_argument(

        "manual",

        nargs="?",

        default=None,

        help="Target manual directory or PDF path (default: all target manuals)",

    )

    parser.add_argument("--status", action="store_true", help="Display status report across all stages")

    parser.add_argument("--prep", action="store_true", help="Run automated preparation stages (Chapters, Layout, Queue: Stages 4-6)")

    parser.add_argument("--stage", type=int, choices=range(1, 20), help="Run a single specific pipeline stage (1-19)")

    parser.add_argument("--from-stage", "-from", type=int, choices=range(1, 20), help="Starting stage number (1-19)")

    parser.add_argument("--to-stage", "-to", type=int, choices=range(1, 20), help="Ending stage number (1-19)")

    parser.add_argument("--pages", "-p", help="Page range filter for applicable stages (e.g. '1-10', '4-15')")

    parser.add_argument("--force", action="store_true", help="Force re-generation of stages even if artifacts exist")

    parser.add_argument("--dpi", type=int, default=300, help="DPI for Stage 8 asset cropping (default: 300)")

    parser.add_argument("--chunk-size", "-c", type=int, default=1, dest="chunk_size", help="Chunk size (work quota) for inferential handoff tracking (default: 1)")
    parser.add_argument("--plan", action="store_true", help="Display execution plan for the requested slice without running")
    parser.add_argument("--reset-stages", action="store_true", help="Reset artifacts and unwind queue statuses for specified stages and pages prior to execution (Stages 7-19)")

    parser.add_argument("--reset-only", action="store_true", help="Perform reset without executing the pipeline afterwards")
    parser.add_argument("--all", "--all-pages", dest="all_pages", action="store_true", help="Apply to all pages across the manual(s)")
    parser.add_argument("--details", action="store_true", help="Display verbose diagnostic file lists below the status table")
    parser.add_argument("--allow-ide", action="store_true", help="Bypass gatekeeper check when running inside Antigravity IDE Chat panel")

    args = parser.parse_args()



    # Determine execution scope

    from_stage: Optional[int] = None

    to_stage: Optional[int] = None



    if args.stage is not None:

        from_stage = args.stage

        to_stage = args.stage

    elif args.prep:

        from_stage = 4

        to_stage = 6

    elif args.from_stage is not None or args.to_stage is not None:

        from_stage = args.from_stage if args.from_stage is not None else 1

        to_stage = args.to_stage if args.to_stage is not None else 19

        if from_stage > to_stage:

            sys.stderr.write(f"Error: --from-stage ({from_stage}) cannot be greater than --to-stage ({to_stage})\n")

            sys.exit(1)

    is_execution = bool(
        args.prep
        or args.stage is not None
        or args.from_stage is not None
        or args.to_stage is not None
        or args.reset_stages
        or args.reset_only
    ) and not args.plan

    verify_runtime_environment(is_execution=is_execution, allow_ide=args.allow_ide)



    if args.manual and args.manual.lower() != "all":

        manual_targets = [args.manual]

    else:

        manual_targets = TARGET_MANUALS



    all_success = True

    for target in manual_targets:

        manual_dir, source_pdf, ocr_pdf = find_manual_dir_and_pdf(target)



        if not manual_dir.exists():

            sys.stderr.write(f"Error: Manual directory not found: {manual_dir}\n")

            all_success = False

            continue



        if args.status or (from_stage is None and to_stage is None and not args.reset_stages and not args.reset_only):
            status = get_pipeline_status(manual_dir, source_pdf, ocr_pdf)
            print_status_report(status, show_details=args.details)
            continue



        if args.reset_stages or args.reset_only:

            reset_script = ".agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py"

            manual_rel = str(manual_dir.relative_to(Path.cwd())).replace("\\", "/") if manual_dir.is_relative_to(Path.cwd()) else str(manual_dir)

            rst_from = from_stage if from_stage is not None else 7

            rst_to = to_stage if to_stage is not None else 20

            is_all = getattr(args, "all_pages", False) or (args.pages and args.pages.strip().lower() in ("all", "all-pages", "*"))

            if not args.pages and not is_all:

                sys.stderr.write("Error: --pages (or --all) is required when using --reset-stages or --reset-only.\n")

                all_success = False

                continue

            reset_cmd = [

                sys.executable,

                reset_script,

                manual_rel,

                "--from-stage", str(rst_from),

                "--to-stage", str(rst_to),

            ]

            if is_all:

                reset_cmd.append("--all")

            else:

                reset_cmd.extend(["--pages", args.pages])

            if not run_command_live(reset_cmd):

                all_success = False

                continue

            if args.reset_only:

                continue



        success = execute_pipeline(
            manual_dir=manual_dir,
            source_pdf=source_pdf,
            ocr_pdf=ocr_pdf,
            from_stage=from_stage,
            to_stage=to_stage,
            pages=args.pages,
            force=args.force,
            dpi=args.dpi,
            chunk_size=args.chunk_size,
            plan=args.plan,
        )

        if not success:

            all_success = False



    if not all_success:

        sys.exit(1)





if __name__ == "__main__":

    main()


"""Dispatcher and Execution Bridge for the 3-Tier Multi-Agent PDF Pipeline.

Bridges the Ahead-Of-Time (AOT) execution plan (build/<stem>_execution_plan.json)
with deterministic CLI runners and chunked subagent execution, validating
disk artifacts at each stage boundary.

Usage:
    python .agents/plugins/pdf-pipeline/skills/pdf-pipeline/scripts/dispatcher.py "Hardware Reference Manual" --status
    python .agents/plugins/pdf-pipeline/skills/pdf-pipeline/scripts/dispatcher.py "Hardware Reference Manual" --next
    python .agents/plugins/pdf-pipeline/skills/pdf-pipeline/scripts/dispatcher.py "Hardware Reference Manual" --run-deterministic
    python .agents/plugins/pdf-pipeline/skills/pdf-pipeline/scripts/dispatcher.py "Hardware Reference Manual" --verify-step 2
    python .agents/plugins/pdf-pipeline/skills/pdf-pipeline/scripts/dispatcher.py "Hardware Reference Manual" --verify-all
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

# Sibling imports
script_dir = Path(__file__).resolve().parent
if str(script_dir) not in sys.path:
    sys.path.insert(0, str(script_dir))

from run_pipeline import (
    find_assets_queue_file,
    find_chapters_file,
    find_manual_dir_and_pdf,
    find_queue_file,
    parse_page_range,
    run_command_live,
    to_relative_posix,
    verify_runtime_environment,
)
from planner import generate_execution_plan


def find_plan_file(manual_dir: Path) -> Optional[Path]:
    """Find existing execution_plan.json in build/."""
    build_dir = manual_dir / "build"
    if not build_dir.exists():
        return None
    for p in build_dir.glob("*_execution_plan.json"):
        return p
    return None


def load_or_create_plan(
    manual_dir: Path,
    from_stage: int = 1,
    to_stage: int = 19,
    pages: Optional[Set[int]] = None,
    force: bool = False,
    chunk_size: Optional[int] = None,
) -> Tuple[Dict[str, Any], Path]:
    """Load an existing execution plan or generate a new one."""
    plan_file = find_plan_file(manual_dir)
    if plan_file and plan_file.exists() and not force:
        try:
            with open(plan_file, "r", encoding="utf-8") as f:
                data = json.load(f)
            return data, plan_file
        except Exception:
            pass

    custom_quotas = {st: chunk_size for st in range(1, 20)} if chunk_size else None
    data = generate_execution_plan(
        manual_dir=manual_dir,
        from_stage=from_stage,
        to_stage=to_stage,
        pages=pages,
        force=force,
        custom_quotas=custom_quotas,
        export_json=True,
    )
    plan_path = manual_dir / Path(data.get("plan_file", "build/execution_plan.json"))
    return data, plan_path


def verify_chunk_on_disk(
    stage: int,
    chunk: Dict[str, Any],
    manual_dir: Path,
    layout_dir: Path,
    assets_queue: Optional[Dict[str, Any]] = None,
) -> Tuple[bool, List[str]]:
    """Verify whether a single chunk's target disk artifacts exist and are valid."""
    missing: List[str] = []
    items = chunk.get("items", [])

    if stage == 7:
        for itm in items:
            p_num = int(itm)
            target_md = layout_dir / f"page_{p_num:04d}.md"
            if not target_md.exists() or target_md.stat().st_size == 0:
                missing.append(to_relative_posix(target_md))

    elif stage == 10:
        # Check assets queue for eval_clip == ok
        aq = assets_queue or {}
        entries = aq.get("assets", {})
        if not entries and isinstance(aq.get("queue"), list):
            entries = {it.get("asset_id"): it for it in aq.get("queue") if isinstance(it, dict)}
        for itm in items:
            entry = entries.get(itm, {})
            if entry.get("eval_clip") != "ok":
                missing.append(f"{itm} (eval_clip != ok)")

    elif stage == 12:
        aq = assets_queue or {}
        entries = aq.get("assets", {})
        if not entries and isinstance(aq.get("queue"), list):
            entries = {it.get("asset_id"): it for it in aq.get("queue") if isinstance(it, dict)}
        for itm in items:
            entry = entries.get(itm, {})
            det_type = entry.get("detected_type")
            target_md = layout_dir / "assets" / f"{itm}_html.md"
            if det_type == "image":
                # Validly classified as image for Stage 14
                continue
            elif not target_md.exists() or target_md.stat().st_size == 0:
                missing.append(to_relative_posix(target_md))

    elif stage == 13:
        for itm in items:
            target_md = layout_dir / "assets" / f"{itm}_reduced.md"
            if not target_md.exists() or target_md.stat().st_size == 0:
                missing.append(to_relative_posix(target_md))

    elif stage == 14:
        for itm in items:
            target_md = layout_dir / "assets" / f"{itm}.md"
            target_txt = layout_dir / "assets" / f"{itm}.txt"
            if not target_md.exists() or target_md.stat().st_size == 0:
                missing.append(to_relative_posix(target_md))
            if not target_txt.exists() or target_txt.stat().st_size == 0:
                missing.append(to_relative_posix(target_txt))

    elif stage == 17:
        for itm in items:
            p_num = int(itm)
            target_md = layout_dir / f"page_{p_num:04d}-proofread.md"
            if not target_md.exists() or target_md.stat().st_size == 0:
                missing.append(to_relative_posix(target_md))

    elif stage == 19:
        for itm in items:
            slug = Path(itm).stem
            target_md = manual_dir / "build" / "02_final_chapters" / f"{slug}.md"
            if not target_md.exists() or target_md.stat().st_size == 0:
                missing.append(to_relative_posix(target_md))
            else:
                try:
                    content = target_md.read_text(encoding="utf-8")
                    if "<continuation-marker>" in content:
                        missing.append(f"{slug}.md (contains unresolved <continuation-marker>)")
                except Exception as e:
                    missing.append(f"{slug}.md (read error: {e})")

    return (len(missing) == 0, missing)


def verify_step(
    step: Dict[str, Any],
    manual_dir: Path,
    layout_dir: Path,
    stem: str,
) -> Dict[str, Any]:
    """Verify whether a plan step is completed on disk."""
    st_type = step.get("type")
    st_num = step.get("stage", 0)

    if st_type == "skipped":
        return {
            "step": step.get("step"),
            "stage": st_num,
            "status": "completed",
            "reason": step.get("reason", "Skipped (already complete)"),
            "pending_chunks": 0,
        }

    if st_type == "deterministic":
        # Check basic completion marker for deterministic stages
        cmd = step.get("command", "")
        # Heuristic check on disk
        is_done = False
        base_stem = stem[:-4] if stem.endswith("_ocr") else stem
        if st_num == 1:
            is_done = True
        elif st_num == 2:
            src_f = manual_dir / f"{base_stem}.pdf"
            is_done = src_f.exists()
        elif st_num == 3:
            build_dir = manual_dir / "build"
            ocr_candidates = list(build_dir.glob("*_ocr.pdf")) + list(manual_dir.glob("*_ocr.pdf"))
            is_done = len(ocr_candidates) > 0
        elif st_num == 4:
            ch_f = find_chapters_file(manual_dir, base_stem)
            is_done = ch_f is not None and ch_f.exists()
        elif st_num == 5:
            # Layout dir has pages
            is_done = any(layout_dir.glob("page_*.json"))
        elif st_num == 6:
            q_f = find_queue_file(manual_dir, base_stem)
            is_done = q_f is not None and q_f.exists()
        elif st_num == 8:
            is_done = any(layout_dir.glob("assets/*.png"))
        elif st_num == 9:
            is_done = any((layout_dir / "eval_frames").glob("*.png"))
        elif st_num == 11:
            aq_f = find_assets_queue_file(manual_dir, base_stem)
            is_done = aq_f is not None and aq_f.exists()
        elif st_num == 15:
            is_done = any((layout_dir / "asset_frames").glob("page_*_asset_frame.png"))
        elif st_num == 16:
            is_done = any(layout_dir.glob("page_*-embed.md"))
        elif st_num == 18:
            is_done = any((manual_dir / "build" / "02_detect_cont_chapters").glob("*.md"))
        else:
            is_done = False

        return {
            "step": step.get("step"),
            "stage": st_num,
            "status": "completed" if is_done else "pending",
            "type": "deterministic",
            "command": cmd,
            "pending_chunks": 0 if is_done else 1,
        }

    # Inferential stage: check chunks
    aq_file = find_assets_queue_file(manual_dir, stem) or (manual_dir / "build" / f"{stem}_assets_queue.json")
    aq_data = None
    if aq_file and aq_file.exists():
        try:
            with open(aq_file, "r", encoding="utf-8") as f:
                aq_data = json.load(f)
        except Exception:
            pass

    pending_chunks = []
    completed_chunks = []

    for c in step.get("chunks", []):
        ok, missing = verify_chunk_on_disk(st_num, c, manual_dir, layout_dir, aq_data)
        if ok:
            completed_chunks.append(c.get("chunk_id"))
        else:
            pending_chunks.append({
                "chunk_id": c.get("chunk_id"),
                "item_count": c.get("item_count"),
                "missing_items": missing,
                "items": c.get("items"),
                "work_units": c.get("work_units"),
            })

    is_complete = (len(pending_chunks) == 0 and len(step.get("chunks", [])) > 0)
    return {
        "step": step.get("step"),
        "stage": st_num,
        "name": step.get("name"),
        "status": "completed" if is_complete else "pending",
        "type": "inferential",
        "tier2_lead": step.get("tier2_lead"),
        "tier3_worker": step.get("tier3_worker"),
        "total_chunks": len(step.get("chunks", [])),
        "completed_chunks": len(completed_chunks),
        "pending_chunks_count": len(pending_chunks),
        "pending_chunks": pending_chunks,
    }


def get_next_action(
    plan_data: Dict[str, Any],
    manual_dir: Path,
) -> Dict[str, Any]:
    """Inspect the plan against disk and determine the single next actionable unit."""
    manual_dir, source_pdf, ocr_pdf = find_manual_dir_and_pdf(str(manual_dir))
    effective_pdf = ocr_pdf or source_pdf
    stem = effective_pdf.stem if effective_pdf else manual_dir.name
    layout_dir = manual_dir / "build" / "01_page_layout"

    plan = plan_data.get("plan", [])

    for s in plan:
        res = verify_step(s, manual_dir, layout_dir, stem)
        if res.get("status") == "completed":
            continue

        if s.get("type") == "deterministic":
            return {
                "action": "run_deterministic",
                "step": s.get("step"),
                "stage": s.get("stage"),
                "name": s.get("name"),
                "command": s.get("command"),
                "description": s.get("description", ""),
            }

        elif s.get("type") == "inferential":
            pending_chunks = res.get("pending_chunks", [])
            if not pending_chunks:
                continue

            next_c = pending_chunks[0]
            return {
                "action": "dispatch_worker",
                "step": s.get("step"),
                "stage": s.get("stage"),
                "name": s.get("name"),
                "lead": s.get("tier2_lead"),
                "worker": s.get("tier3_worker"),
                "skill": s.get("skill"),
                "prompt": s.get("prompt"),
                "chunk": next_c,
                "remaining_chunks": len(pending_chunks),
                "total_chunks": res.get("total_chunks"),
            }

    return {
        "action": "all_completed",
        "message": "All execution plan steps verified successfully on disk.",
    }


def run_deterministic_sequence(
    plan_data: Dict[str, Any],
    manual_dir: Path,
) -> bool:
    """Execute consecutive deterministic steps until reaching an inferential step or plan completion."""
    manual_dir, source_pdf, ocr_pdf = find_manual_dir_and_pdf(str(manual_dir))
    effective_pdf = ocr_pdf or source_pdf
    stem = effective_pdf.stem if effective_pdf else manual_dir.name
    layout_dir = manual_dir / "build" / "01_page_layout"

    plan = plan_data.get("plan", [])

    for s in plan:
        res = verify_step(s, manual_dir, layout_dir, stem)
        if res.get("status") == "completed":
            print(f"[Step {s.get('step'):02d}] Stage {s.get('stage'):02d}: {s.get('name')} -> PASS (Already Completed)")
            continue

        if s.get("type") == "deterministic":
            cmd_str = s.get("command", "")
            print(f"\n{'='*70}")
            print(f"Executing [Step {s.get('step'):02d}] Stage {s.get('stage'):02d}: {s.get('name')}")
            print(f"Command: {cmd_str}")
            print(f"{'='*70}")

            import shlex
            cmd_args = shlex.split(cmd_str)
            if not run_command_live(cmd_args):
                sys.stderr.write(f"\n[ERROR] Step {s.get('step')} failed.\n")
                return False

            print(f"[SUCCESS] Step {s.get('step'):02d} completed.\n")

        elif s.get("type") == "inferential":
            print(f"\n[HALT AT INFERENTIAL BOUNDARY] Next step is inferential:")
            print(f"  Step {s.get('step'):02d}: Stage {s.get('stage'):02d} ({s.get('name')})")
            print(f"  Lead Agent:   {s.get('tier2_lead')}")
            print(f"  Worker Agent: {s.get('tier3_worker')}")
            print(f"  Pending:      {res.get('pending_chunks_count')} chunk(s) remaining.")
            print(f"Run `dispatcher.py --next` to retrieve the next chunk payload.")
            return True

    print("\n[SUCCESS] All steps in the execution plan completed.")
    return True


def print_dispatcher_status(plan_data: Dict[str, Any], manual_dir: Path) -> None:
    """Print status of all execution plan steps verified against disk."""
    manual_dir, source_pdf, ocr_pdf = find_manual_dir_and_pdf(str(manual_dir))
    effective_pdf = ocr_pdf or source_pdf
    stem = effective_pdf.stem if effective_pdf else manual_dir.name
    layout_dir = manual_dir / "build" / "01_page_layout"

    plan = plan_data.get("plan", [])
    print("\n" + "=" * 80)
    print(f" DISPATCHER VERIFICATION: '{manual_dir.name}'")
    print("=" * 80)

    for s in plan:
        res = verify_step(s, manual_dir, layout_dir, stem)
        st_num = s.get("stage", 0)
        st_name = s.get("name", "")
        status = res.get("status", "").upper()

        if s.get("type") == "skipped":
            print(f"[Step {s.get('step'):02d}] Stage {st_num:02d}: {st_name:<30} -> PASS (Skipped)")
        elif s.get("type") == "deterministic":
            tag = "DONE" if status == "COMPLETED" else "PENDING"
            print(f"[Step {s.get('step'):02d}] Stage {st_num:02d}: {st_name:<30} -> [{tag}] (CLI)")
        else:
            done_c = res.get("completed_chunks", 0)
            tot_c = res.get("total_chunks", 0)
            tag = "DONE" if status == "COMPLETED" else f"PENDING ({done_c}/{tot_c} chunks)"
            lead = s.get("tier2_lead", "")
            print(f"[Step {s.get('step'):02d}] Stage {st_num:02d}: {st_name:<30} -> [{tag}] via {lead}")

    print("=" * 80 + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(description="3-Tier Pipeline Dispatcher & Bridge")
    parser.add_argument("manual", nargs="?", default="Hardware Reference Manual", help="Target manual directory")
    parser.add_argument("--status", action="store_true", help="Print verified status of all plan steps on disk")
    parser.add_argument("--next", action="store_true", help="Print next actionable unit (CLI command or chunk payload) in JSON")
    parser.add_argument("--run-deterministic", action="store_true", help="Execute deterministic steps until next inferential stage")
    parser.add_argument("--verify-step", type=int, help="Verify a specific plan step number against disk")
    parser.add_argument("--verify-all", action="store_true", help="Verify all steps and exit with 0 if complete, 1 if pending")
    parser.add_argument("--allow-ide", action="store_true", help="Bypass gatekeeper check when running inside Antigravity IDE Chat panel")

    args = parser.parse_args()
    manual_dir = Path(args.manual)
    manual_dir, source_pdf, ocr_pdf = find_manual_dir_and_pdf(str(manual_dir))

    plan_data, plan_file = load_or_create_plan(manual_dir)

    if args.status:
        print_dispatcher_status(plan_data, manual_dir)
        return

    if args.next:
        action = get_next_action(plan_data, manual_dir)
        print(json.dumps(action, indent=2, ensure_ascii=False))
        return

    if args.run_deterministic:
        verify_runtime_environment(is_execution=True, allow_ide=args.allow_ide)
        ok = run_deterministic_sequence(plan_data, manual_dir)
        if not ok:
            sys.exit(1)
        return

    if args.verify_step is not None:
        effective_pdf = ocr_pdf or source_pdf
        stem = effective_pdf.stem if effective_pdf else manual_dir.name
        layout_dir = manual_dir / "build" / "01_page_layout"
        plan = plan_data.get("plan", [])
        matched = [s for s in plan if s.get("step") == args.verify_step]
        if not matched:
            sys.stderr.write(f"Error: Step {args.verify_step} not found in execution plan.\n")
            sys.exit(1)
        res = verify_step(matched[0], manual_dir, layout_dir, stem)
        print(json.dumps(res, indent=2, ensure_ascii=False))
        if res.get("status") != "completed":
            sys.exit(1)
        return

    if args.verify_all:
        effective_pdf = ocr_pdf or source_pdf
        stem = effective_pdf.stem if effective_pdf else manual_dir.name
        layout_dir = manual_dir / "build" / "01_page_layout"
        plan = plan_data.get("plan", [])
        all_ok = True
        for s in plan:
            res = verify_step(s, manual_dir, layout_dir, stem)
            if res.get("status") != "completed":
                all_ok = False
                break
        if all_ok:
            print(json.dumps({"status": "completed", "message": "All steps verified."}, indent=2))
            sys.exit(0)
        else:
            print(json.dumps({"status": "pending", "message": "Steps still pending."}, indent=2))
            sys.exit(1)

    # Default action if no flag specified: show status
    print_dispatcher_status(plan_data, manual_dir)


if __name__ == "__main__":
    main()

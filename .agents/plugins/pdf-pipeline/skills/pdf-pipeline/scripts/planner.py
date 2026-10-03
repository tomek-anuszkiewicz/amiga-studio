"""Ahead-Of-Time (AOT) Execution Plan Generator for Technical PDF-to-Markdown Pipeline.

Pre-computes deterministic execution graphs, slicing inferential stages into
stage-calibrated chunks for Tier 2 Stage Leads and Tier 3 Leaf Workers, while
routing deterministic stages to single-shot CLI commands.

Outputs: build/<stem>_execution_plan.json
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Set

# Relative import from sibling module
script_dir = Path(__file__).resolve().parent
if str(script_dir) not in sys.path:
    sys.path.insert(0, str(script_dir))

from run_pipeline import (
    STAGE_REGISTRY,
    find_assets_queue_file,
    find_manual_dir_and_pdf,
    find_queue_file,
    format_page_range,
    inspect_page_status,
    parse_page_range,
    resolve_stage_item_details,
    to_relative_posix,
)

# Calibrated stage-aware chunk quotas
STAGE_QUOTAS: Dict[int, int] = {
    7: 5,   # Page transcription (heavy multimodal)
    10: 10, # Eval clip frames
    12: 10, # Convert tables & classify images
    13: 10, # Table reduction to Markdown
    14: 10, # Image diagram breakdowns
    17: 10, # Page proofreading (N-1 sliding window)
    19: 1,  # Final chapter merger
}

# Global concurrency ceiling per stage wave
DEFAULT_MAX_CONCURRENCY = 3

# Dedicated Tier 2 Stage Leads
TIER2_STAGE_LEADS: Dict[int, str] = {
    7: "stage7-transcription-lead",
    10: "stage10-eval-lead",
    12: "stage12-table-html-lead",
    13: "stage13-reduction-lead",
    14: "stage14-image-lead",
    17: "stage17-proofread-lead",
    19: "stage19-chapter-merge-lead",
}

# Dedicated Tier 3 Leaf Workers
TIER3_LEAF_WORKERS: Dict[int, str] = {
    7: "stage7-worker",
    10: "stage10-worker",
    12: "stage12-worker",
    13: "stage13-worker",
    14: "stage14-worker",
    17: "stage17-worker",
    19: "stage19-worker",
}


def generate_execution_plan(
    manual_dir: Path,
    from_stage: int = 1,
    to_stage: int = 19,
    pages: Optional[Set[int]] = None,
    force: bool = False,
    custom_quotas: Optional[Dict[int, int]] = None,
    max_concurrency: int = DEFAULT_MAX_CONCURRENCY,
    export_json: bool = True,
) -> Dict[str, Any]:
    """Generate an immutable AOT Execution Plan mapping stages to deterministic CLI or chunked leaf workers."""
    manual_dir, source_pdf, ocr_pdf = find_manual_dir_and_pdf(str(manual_dir))
    effective_pdf = ocr_pdf or source_pdf
    stem = effective_pdf.stem if effective_pdf else manual_dir.name
    layout_dir = manual_dir / "build" / "01_page_layout"

    q_file = find_queue_file(manual_dir, stem) or (manual_dir / "build" / f"{stem}_queue.json")
    aq_file = find_assets_queue_file(manual_dir, stem) or (manual_dir / "build" / f"{stem}_assets_queue.json")

    stage_status = inspect_page_status(
        manual_dir=manual_dir,
        source_pdf=source_pdf,
        ocr_pdf=ocr_pdf,
        layout_dir=layout_dir,
        queue_file=q_file,
        assets_queue_file=aq_file,
        pages=pages,
    )

    p_str = format_page_range(pages) if pages else "all"
    quotas = dict(STAGE_QUOTAS)
    if custom_quotas:
        quotas.update(custom_quotas)

    plan_steps: List[Dict[str, Any]] = []
    deterministic_count = 0
    inferential_count = 0
    total_chunks = 0
    total_batches = 0

    step_counter = 1
    for st in range(from_stage, to_stage + 1):
        if st not in STAGE_REGISTRY:
            continue
        info = STAGE_REGISTRY[st]
        st_res = stage_status.get(st, {})
        is_inferential = (info["type"] == "inferential")
        is_ready = bool(st_res.get("ready"))

        if is_ready and not force:
            plan_steps.append({
                "step": step_counter,
                "stage": st,
                "name": info["name"],
                "type": "skipped",
                "status": "already_completed",
                "reason": st_res.get("details", "All target items already completed."),
            })
            step_counter += 1
            continue

        if not is_inferential:
            deterministic_count += 1
            p_arg = f" --pages {p_str}" if info.get("supports_pages") and pages else ""
            script_path = info.get("script", "")
            manual_rel = to_relative_posix(manual_dir)
            pdf_rel = to_relative_posix(effective_pdf) if effective_pdf else manual_rel
            src_rel = to_relative_posix(source_pdf) if source_pdf else manual_rel

            if st == 1:
                cmd = f"python {script_path}"
            elif st == 2:
                cmd = f"powershell -ExecutionPolicy Bypass -File {script_path}"
            elif st == 3:
                cmd = f"python {script_path} \"{src_rel}\""
            elif st in (4, 5, 6):
                cmd = f"python {script_path} \"{pdf_rel}\"{p_arg}"
            else:
                cmd = f"python {script_path} \"{manual_rel}\"{p_arg}"

            if force:
                cmd += " -Force" if st == 2 else " --force"

            plan_steps.append({
                "step": step_counter,
                "stage": st,
                "name": info["name"],
                "type": "deterministic",
                "execution_mode": "bulk_cli",
                "skill": info.get("skill", ""),
                "command": cmd,
                "description": info.get("description", ""),
                "pending_count": st_res.get("pending_count", 0),
            })
            step_counter += 1

        else:
            inferential_count += 1
            pending_items = st_res.get("pending_items", [])
            quota = quotas.get(st, 5)
            lead_name = TIER2_STAGE_LEADS.get(st, f"stage{st}-lead")
            worker_name = TIER3_LEAF_WORKERS.get(st, f"stage{st}-worker")

            # Slice into chunks
            chunks: List[Dict[str, Any]] = []
            for chunk_idx in range(0, max(len(pending_items), 1), quota):
                chunk_items = pending_items[chunk_idx : chunk_idx + quota]
                if not chunk_items and pending_items:
                    continue
                c_num = (chunk_idx // quota) + 1
                chunk_details = [
                    resolve_stage_item_details(st, itm, layout_dir)
                    for itm in chunk_items
                ]
                chunks.append({
                    "chunk_id": c_num,
                    "item_count": len(chunk_items),
                    "items": [str(x) for x in chunk_items],
                    "work_units": chunk_details,
                })

            # Group chunks into parallel batch waves (capped by max_concurrency)
            batches: List[Dict[str, Any]] = []
            for b_idx in range(0, max(len(chunks), 1), max_concurrency):
                b_chunks = chunks[b_idx : b_idx + max_concurrency]
                if not b_chunks and chunks:
                    continue
                b_num = (b_idx // max_concurrency) + 1
                batches.append({
                    "batch_id": b_num,
                    "chunk_count": len(b_chunks),
                    "chunk_ids": [c["chunk_id"] for c in b_chunks],
                    "chunks": b_chunks,
                })

            total_chunks += len(chunks)
            total_batches += len(batches)

            step_entry: Dict[str, Any] = {
                "step": step_counter,
                "stage": st,
                "name": info["name"],
                "type": "inferential",
                "tier2_lead": lead_name,
                "tier3_worker": worker_name,
                "skill": info.get("skill", ""),
                "prompt": info.get("prompt", ""),
                "calibrated_chunk_size": quota,
                "max_concurrency": max_concurrency,
                "pending_items_count": len(pending_items),
                "chunk_count": len(chunks),
                "batch_count": len(batches),
                "batches": batches,
                "chunks": chunks,
            }

            # Special case: Stage 9 <-> Stage 10 loop encapsulation
            if st == 10:
                step_entry["loop_encapsulation"] = {
                    "is_recursive_loop": True,
                    "governor_stage": 9,
                    "governor_script": ".agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py",
                    "prep_frames_cmd": f"python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py \"{to_relative_posix(manual_dir)}\" --prep-frames" + (f" --pages {p_str}" if pages else ""),
                    "apply_recrops_cmd": f"python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py \"{to_relative_posix(manual_dir)}\" --apply-recrops",
                    "finalize_cmd": f"python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py \"{to_relative_posix(manual_dir)}\" --finalize" + (f" --pages {p_str}" if pages else ""),
                }

            plan_steps.append(step_entry)
            step_counter += 1

    plan_data: Dict[str, Any] = {
        "manual": manual_dir.name,
        "manual_dir": to_relative_posix(manual_dir),
        "source_pdf": to_relative_posix(source_pdf) if source_pdf else None,
        "ocr_pdf": to_relative_posix(ocr_pdf) if ocr_pdf else None,
        "scope": {
            "from_stage": from_stage,
            "to_stage": to_stage,
            "pages": p_str,
            "target_pages_count": len(pages) if pages else "all",
            "force": force,
        },
        "quotas": quotas,
        "max_concurrency": max_concurrency,
        "summary": {
            "total_steps": len(plan_steps),
            "skipped_steps": sum(1 for s in plan_steps if s["type"] == "skipped"),
            "deterministic_steps": deterministic_count,
            "inferential_steps": inferential_count,
            "total_inferential_chunks": total_chunks,
            "total_inferential_batches": total_batches,
            "max_concurrency": max_concurrency,
        },
        "plan": plan_steps,
    }

    if export_json:
        build_dir = manual_dir / "build"
        build_dir.mkdir(parents=True, exist_ok=True)
        plan_out = build_dir / f"{stem}_execution_plan.json"
        with open(plan_out, "w", encoding="utf-8") as f:
            json.dump(plan_data, f, indent=2, ensure_ascii=False)
        plan_data["plan_file"] = to_relative_posix(plan_out)

    return plan_data


def print_execution_plan_dashboard(plan_data: Dict[str, Any]) -> None:
    """Print an ASCII dashboard of the generated Ahead-Of-Time execution plan."""
    scope = plan_data.get("scope", {})
    summary = plan_data.get("summary", {})
    plan = plan_data.get("plan", [])

    print("\n" + "=" * 90)
    print(f" AHEAD-OF-TIME (AOT) EXECUTION PLAN: '{plan_data.get('manual')}'")
    print(f" Scope: Stages {scope.get('from_stage')} -> {scope.get('to_stage')} | Pages: {scope.get('pages')}")
    print(f" Summary: {summary.get('deterministic_steps')} Deterministic (CLI) | {summary.get('inferential_steps')} Inferential ({summary.get('total_inferential_chunks')} Chunks)")
    if "plan_file" in plan_data:
        print(f" Plan File: {plan_data['plan_file']}")
    print("=" * 90)

    for s in plan:
        st_num = s.get("stage")
        st_name = s.get("name")
        st_type = s.get("type")

        if st_type == "skipped":
            print(f"\n[Step {s.get('step'):02d}] Stage {st_num:02d}: {st_name} -> [ALREADY COMPLETE] (Pass)")
            continue

        if st_type == "deterministic":
            print(f"\n[Step {s.get('step'):02d}] Stage {st_num:02d}: {st_name} -> [DETERMINISTIC CLI]")
            print(f"  * Action:      Bulk single-shot execution across target slice")
            print(f"  * Command:     {s.get('command')}")

        elif st_type == "inferential":
            lead = s.get("tier2_lead")
            worker = s.get("tier3_worker")
            quota = s.get("calibrated_chunk_size")
            n_chunks = s.get("chunk_count")
            n_pending = s.get("pending_items_count")
            loop_info = s.get("loop_encapsulation")
            n_batches = s.get("batch_count", len(s.get("batches", [])))
            max_c = s.get("max_concurrency", DEFAULT_MAX_CONCURRENCY)

            print(f"\n[Step {s.get('step'):02d}] Stage {st_num:02d}: {st_name} -> [INFERENTIAL]")
            print(f"  * Tier 2 Lead:   {lead}")
            print(f"  * Tier 3 Worker: {worker} (Chunk Quota: {quota}, Max Concurrency: {max_c})")
            print(f"  * Workload:      {n_pending} item(s) mapped into {n_chunks} chunk(s) across {n_batches} parallel batch(es)")

            if loop_info:
                print(f"  * Loop Encapsulation: Bidirectional Stage 9 <-> 10 feedback governed by {lead}")

            for b in s.get("batches", []):
                bid = b.get("batch_id")
                b_cids = b.get("chunk_ids", [])
                print(f"      - Batch {bid:02d}: [Chunks {b_cids}] -> {b.get('chunk_count')} concurrent worker(s)")

    print("\n" + "=" * 90 + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(description="Ahead-Of-Time (AOT) Execution Plan Generator")
    parser.add_argument("manual", nargs="?", default="Hardware Reference Manual", help="Target manual directory")
    parser.add_argument("--from-stage", "-from", type=int, default=1, choices=range(1, 20), help="Starting stage (1-19)")
    parser.add_argument("--to-stage", "-to", type=int, default=19, choices=range(1, 20), help="Ending stage (1-19)")
    parser.add_argument("--pages", "-p", help="Page range filter (e.g. '1-10', '4-15')")
    parser.add_argument("--force", action="store_true", help="Force planning even for completed stages")
    parser.add_argument("--chunk-size", "-c", type=int, help="Override chunk size across all inferential stages")
    parser.add_argument("--max-concurrency", "-mc", type=int, default=DEFAULT_MAX_CONCURRENCY, help="Maximum concurrent workers per batch wave (default: 3)")
    parser.add_argument("--no-export", action="store_true", help="Do not write execution_plan.json to disk")

    args = parser.parse_args()
    parsed_pages = parse_page_range(args.pages) if args.pages else None

    custom_quotas = None
    if args.chunk_size:
        custom_quotas = {st: args.chunk_size for st in STAGE_QUOTAS}

    plan_data = generate_execution_plan(
        manual_dir=Path(args.manual),
        from_stage=args.from_stage,
        to_stage=args.to_stage,
        pages=parsed_pages,
        force=args.force,
        custom_quotas=custom_quotas,
        max_concurrency=args.max_concurrency,
        export_json=not args.no_export,
    )

    print_execution_plan_dashboard(plan_data)


if __name__ == "__main__":
    main()

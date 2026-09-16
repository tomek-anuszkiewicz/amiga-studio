#!/usr/bin/env python3
"""
pipeline.py: Master CLI Orchestrator for the 12-Stage PDF-to-Markdown Pipeline.
"""

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Optional
import yaml


STAGE_DEFINITIONS = [
    ("01", "01_preprocess", "preprocess.py", "Deconstruct PDF into pages, PNGs, and text blocks"),
    ("01b", "01b_ocr", "detect_and_ocr.py", "Detect scan/empty pages and extract OCR blocks via Gemini Vision"),
    ("02", "02_page_segmentation", "segment_page.py", "Vertical banding & zone classification"),
    ("03", "03_build_raw_stream", "build_stream.py", "Build raw stream & extract initial assets"),
    ("04", "04_stream_reduction", "reduce_stream.py", "Normalize stream: weld prose & de-hyphenate"),
    ("05", "05_chapter_partition", "partition_chapters.py", "Partition stream into numbered sections"),
    ("06", "06_detect_continuations", "detect_continuations.py", "Detect multi-page table/graphic continuations"),
    ("07", "07_transform_tables", "transform_tables.py", "Transform table nodes (GFM vs HTML table)"),
    ("08", "08_transform_graphics", "transform_graphics.py", "Transform graphics (Mermaid vs SVG + RAG sidecars)"),
    ("09", "09_transform_prose", "format_prose.py", "Format prose/code and tag TOC with TOC34534"),
    ("10", "10_proofread_stream", "proofread_stream.py", "Proofread chapter streams & manifest with LLM"),
    ("11", "11_emit_markdown", "emit_markdown.py", "Emit per-section Markdown files (suppressing toc_header)"),
    ("12", "12_refine_first_chapter_name", "refine_name.py", "Refine canonical name of first chapter"),
    ("13", "13_link_toc", "link_toc.py", "Fuzzy header matching & TOC wikilink conversion"),
]


def load_config(config_path: Path) -> dict:
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            return yaml.safe_load(f) or {}
    return {}


def update_status(
    status_file: Path,
    stage_num: str,
    status: str,
    details: str = "",
    duration_seconds: Optional[float] = None,
    llm_calls: Optional[int] = None,
):
    data = {}
    if status_file.exists():
        try:
            with open(status_file, "r", encoding="utf-8") as f:
                data = json.load(f)
        except Exception:
            data = {}

    entry = data.get(stage_num, {})
    entry["status"] = status
    if details or "details" not in entry:
        entry["details"] = details

    if duration_seconds is not None:
        entry["duration_seconds"] = duration_seconds
    elif "duration_seconds" not in entry:
        entry["duration_seconds"] = None

    if llm_calls is not None:
        entry["llm_calls"] = llm_calls
    elif "llm_calls" not in entry:
        entry["llm_calls"] = 0

    data[stage_num] = entry
    status_file.parent.mkdir(parents=True, exist_ok=True)
    with open(status_file, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)


def get_last_completed_stage_idx(status_file: Path) -> int:
    if not status_file.exists():
        return -1
    try:
        with open(status_file, "r", encoding="utf-8") as f:
            data = json.load(f)
        last_idx = -1
        for idx, (s_id, _, _, _) in enumerate(STAGE_DEFINITIONS):
            if data.get(s_id, {}).get("status") == "success":
                last_idx = idx
            else:
                break
        return last_idx
    except Exception:
        return -1


def resolve_stage_idx(arg_val: str) -> Optional[int]:
    if not arg_val:
        return None
    val = str(arg_val).strip().lower()
    for idx, (s_id, s_dir, _, _) in enumerate(STAGE_DEFINITIONS):
        if val == s_id.lower() or val == s_dir.lower():
            return idx
        if val.isdigit() and s_id.isdigit() and int(val) == int(s_id):
            return idx
        if val in (s_id.lstrip('0').lower(), f"{s_id.lstrip('0')}b"):
            return idx
    return None


def run_stage(
    stage_idx: int,
    stage_info: tuple,
    skill_dir: Path,
    workspace_dir: Path,
    pdf_path: Path,
    output_dir: Path,
    config_path: Path,
    verbose: bool,
    max_pages: int = None,
    page_range: str = None,
    start_page: int = None,
    end_page: int = None,
) -> bool:
    stage_num, stage_dir_name, script_name, desc = stage_info
    script_path = skill_dir / "stages" / stage_dir_name / script_name

    print(f"\n==================================================")
    print(f"[*] Stage {stage_num}: {stage_dir_name} ({desc})")
    print(f"==================================================")

    if not script_path.exists():
        print(f"[!] Error: Script not found: {script_path}", file=sys.stderr)
        return False

    cmd = [
        sys.executable,
        str(script_path),
        "--workspace", str(workspace_dir),
        "--config", str(config_path),
    ]

    # Add stage-specific flags if needed
    if stage_num in ("01", "01b"):
        if stage_num == "01":
            cmd.extend(["--pdf", str(pdf_path)])
        if page_range:
            cmd.extend(["--page-range", str(page_range)])
        elif start_page or end_page:
            if start_page:
                cmd.extend(["--start-page", str(start_page)])
            if end_page:
                cmd.extend(["--end-page", str(end_page)])
        elif max_pages:
            cmd.extend(["--max-pages", str(max_pages)])
    elif stage_num == "10":
        cmd.extend([
            "--input-dir", str(workspace_dir / "09_transform_prose"),
            "--output-dir", str(workspace_dir / "10_proofread_stream"),
        ])
    elif stage_num == "11":
        cmd.extend(["--output-dir", str(workspace_dir / "11_emit_markdown")])
    elif stage_num == "12":
        cmd.extend([
            "--input-dir", str(workspace_dir / "11_emit_markdown"),
            "--output-dir", str(workspace_dir / "12_refine_first_chapter_name"),
        ])
    elif stage_num == "13":
        dest_dir = output_dir if output_dir else (workspace_dir / "13_link_toc")
        cmd.extend([
            "--input-dir", str(workspace_dir / "12_refine_first_chapter_name"),
            "--output-dir", str(dest_dir),
        ])

    if verbose:
        print(f"[CMD] {' '.join(cmd)}")

    status_file = workspace_dir / "stage_status.json"
    stage_metrics_file = workspace_dir / f".stage_{stage_num}_metrics.json"

    # Reset metrics file for this stage run
    try:
        if stage_metrics_file.exists():
            stage_metrics_file.unlink()
        stage_metrics_file.parent.mkdir(parents=True, exist_ok=True)
        with open(stage_metrics_file, "w", encoding="utf-8") as f:
            json.dump({"llm_calls": 0}, f)
    except Exception:
        pass

    env = os.environ.copy()
    env["LLM_STAGE_METRICS_FILE"] = str(stage_metrics_file)

    def read_llm_calls() -> int:
        if stage_metrics_file.exists():
            try:
                with open(stage_metrics_file, "r", encoding="utf-8") as f:
                    return json.load(f).get("llm_calls", 0)
            except Exception:
                return 0
        return 0

    update_status(status_file, stage_num, "running")

    start_time = time.time()
    try:
        result = subprocess.run(cmd, env=env, check=True)
        duration = round(time.time() - start_time, 2)
        calls = read_llm_calls()
        update_status(status_file, stage_num, "success", duration_seconds=duration, llm_calls=calls)
        print(f"[*] Stage {stage_num} finished in {duration:.2f}s with {calls} LLM call(s).")
        return True
    except subprocess.CalledProcessError as e:
        duration = round(time.time() - start_time, 2)
        calls = read_llm_calls()
        print(f"[!] Stage {stage_num} failed with return code {e.returncode} ({duration:.2f}s, {calls} LLM calls)", file=sys.stderr)
        update_status(status_file, stage_num, "failed", f"Exit code {e.returncode}", duration_seconds=duration, llm_calls=calls)
        return False
    except Exception as e:
        duration = round(time.time() - start_time, 2)
        calls = read_llm_calls()
        print(f"[!] Stage {stage_num} encountered exception: {e} ({duration:.2f}s, {calls} LLM calls)", file=sys.stderr)
        update_status(status_file, stage_num, "failed", str(e), duration_seconds=duration, llm_calls=calls)
        return False


def print_pipeline_status(workspace_dir: Path, output_dir: Path):
    print("\n==================================================")
    print("         PDF-to-Markdown Pipeline Status          ")
    print("==================================================")

    # 1. 01_preprocess (01)
    p1 = workspace_dir / "01_preprocess" if (workspace_dir / "01_preprocess").exists() else (workspace_dir / "01_pages")
    p1_count = len(list(p1.glob("page_*.png"))) if p1.exists() else 0
    print(f"[*] 01_preprocess                 : {p1_count} rendered PNGs")

    # 1b. 01b_ocr (01b)
    p1_json_count = len(list(p1.glob("page_*.json"))) if p1.exists() else 0
    print(f"[*] 01b_ocr                       : {p1_json_count} page JSON text streams inspected")

    # 2. 02_page_segmentation (02)
    p2 = workspace_dir / "02_page_segmentation" if (workspace_dir / "02_page_segmentation").exists() else (workspace_dir / "02_segments")
    p2_count = len(list(p2.glob("page_*_segments.json"))) if p2.exists() else 0
    print(f"[*] 02_page_segmentation          : {p2_count} segment JSON files")

    # 3. 03_build_raw_stream (03)
    p3 = (workspace_dir / "03_build_raw_stream" / "raw_stream.json") if (workspace_dir / "03_build_raw_stream").exists() else (workspace_dir / "03_raw_stream" / "raw_stream.json")
    print(f"[*] 03_build_raw_stream           : {'OK (' + str(p3.stat().st_size) + ' B)' if p3.exists() else 'Missing'}")

    # 4. 04_stream_reduction (04)
    p4 = (workspace_dir / "04_stream_reduction" / "reduced_stream.json") if (workspace_dir / "04_stream_reduction").exists() else (workspace_dir / "04_reduced_stream" / "reduced_stream.json")
    print(f"[*] 04_stream_reduction           : {'OK (' + str(p4.stat().st_size) + ' B)' if p4.exists() else 'Missing'}")

    # 5. 05_chapter_partition (05)
    p5 = workspace_dir / "05_chapter_partition" if (workspace_dir / "05_chapter_partition").exists() else (workspace_dir / "05_chapters_raw")
    p5_count = len(list(p5.glob("*.json"))) if p5.exists() else 0
    print(f"[*] 05_chapter_partition          : {p5_count} chapter stream files")

    # 6. 06_detect_continuations (06)
    p6 = workspace_dir / "06_detect_continuations" if (workspace_dir / "06_detect_continuations").exists() else (workspace_dir / "06_chapters_continuations")
    p6_count = len(list(p6.glob("*.json"))) if p6.exists() else 0
    print(f"[*] 06_detect_continuations       : {p6_count} chapter stream files")

    # 7. 07_transform_tables (07)
    p7 = workspace_dir / "07_transform_tables" if (workspace_dir / "07_transform_tables").exists() else (workspace_dir / "07_chapters_tables")
    p7_count = len(list(p7.glob("*.json"))) if p7.exists() else 0
    print(f"[*] 07_transform_tables           : {p7_count} chapter stream files")

    # 8. 08_transform_graphics (08)
    p8 = workspace_dir / "08_transform_graphics" if (workspace_dir / "08_transform_graphics").exists() else (workspace_dir / "08_chapters_graphics")
    p8_count = len(list(p8.glob("*.json"))) if p8.exists() else 0
    print(f"[*] 08_transform_graphics         : {p8_count} chapter stream files")

    # 9. 09_transform_prose (09)
    p9 = workspace_dir / "09_transform_prose" if (workspace_dir / "09_transform_prose").exists() else (workspace_dir / "09_chapters_formatted")
    p9_count = len(list(p9.glob("*.json"))) if p9.exists() else 0
    print(f"[*] 09_transform_prose            : {p9_count} chapter stream files")

    # 10. 10_proofread_stream (10)
    p10 = workspace_dir / "10_proofread_stream"
    p10_count = len(list(p10.glob("*.json"))) if p10.exists() else 0
    print(f"[*] 10_proofread_stream           : {p10_count} chapter stream files")

    # 11. 11_emit_markdown (11)
    p11 = workspace_dir / "11_emit_markdown" if (workspace_dir / "11_emit_markdown").exists() else (workspace_dir / "10_emit_markdown")
    p11_count = len(list(p11.glob("*.md"))) if p11.exists() else 0
    print(f"[*] 11_emit_markdown              : {p11_count} files")

    # 12. 12_refine_first_chapter_name (12)
    p12 = workspace_dir / "12_refine_first_chapter_name" if (workspace_dir / "12_refine_first_chapter_name").exists() else (workspace_dir / "12_canonical_markdown")
    p12_count = len(list(p12.glob("*.md"))) if p12.exists() else 0
    print(f"[*] 12_refine_first_chapter_name  : {p12_count} files")

    # 13. 13_link_toc (13)
    p13 = workspace_dir / "13_link_toc" if (workspace_dir / "13_link_toc").exists() else (workspace_dir / "11_link_toc")
    p13_count = len(list(p13.glob("*.md"))) if p13.exists() else 0
    print(f"[*] 13_link_toc                   : {p13_count} files")

    # Final Output Markdown (if custom output_dir used)
    if output_dir:
        md_count = len(list(output_dir.glob("*.md"))) if output_dir.exists() else 0
        print(f"[*] Custom Output Dir             : {md_count} files in {output_dir.name}/")

    status_file = workspace_dir / "stage_status.json"
    if status_file.exists():
        try:
            with open(status_file, "r", encoding="utf-8") as f:
                status_data = json.load(f)
            if status_data:
                print("\n---------------- Stage Statistics ----------------")
                total_duration = 0.0
                total_calls = 0
                for num, name, _, _ in STAGE_DEFINITIONS:
                    if num in status_data:
                        st = status_data[num]
                        status = st.get("status", "unknown")
                        dur = st.get("duration_seconds")
                        dur_str = f"{dur:.2f}s" if dur is not None else "-"
                        if dur is not None:
                            total_duration += dur
                        calls = st.get("llm_calls", 0)
                        total_calls += calls
                        print(f"Stage {num} ({name:<28}): {status:<8} | Time: {dur_str:>8} | LLM: {calls:>4} call(s)")
                print(f"Total Measured Time: {total_duration:.2f}s | Total LLM Calls: {total_calls}")
                print("--------------------------------------------------")
        except Exception:
            pass

    print("==================================================\n")


STAGE_OUTPUT_TARGETS = {
    "01": ["01_preprocess", "01_pages", "pages", "pages_manifest.json", "manifest.json"],
    "01b": [],
    "02": ["02_page_segmentation", "02_segments", "segments"],
    "03": ["03_build_raw_stream", "03_raw_stream", "raw_stream.json", "assets"],
    "04": ["04_stream_reduction", "04_reduced_stream", "reduced_stream.json"],
    "05": ["05_chapter_partition", "05_chapters_raw", "chapters", "chapters_manifest.json"],
    "06": ["06_detect_continuations", "06_chapters_continuations", "tasks/continuations"],
    "07": ["07_transform_tables", "07_chapters_tables", "tasks/tables"],
    "08": ["08_transform_graphics", "08_chapters_graphics", "tasks/graphics", "__ASSETS_SIDECARS__"],
    "09": ["09_transform_prose", "09_chapters_formatted", "tasks/prose"],
    "10": ["10_proofread_stream"],
    "11": ["11_emit_markdown", "10_emit_markdown", "10_markdown_raw"],
    "12": ["12_refine_first_chapter_name", "12_canonical_markdown"],
    "13": ["13_link_toc", "11_link_toc", "13_proofread_markdown", "__OUTPUT_DIR__"],
}


def clean_downstream_stages(workspace_dir: Path, output_dir: Path, start_idx: int, status_file: Path):
    """
    Cleans all intermediate artifacts and output directories for all stages >= start_idx.
    Guarantees that re-running from stage N starts completely fresh without stale downstream files.
    """
    import shutil
    start_info = STAGE_DEFINITIONS[start_idx]
    stages_to_clean = [s[0] for s in STAGE_DEFINITIONS[start_idx:]]
    print(f"[*] Invalidation: Wiping intermediate and output artifacts from Stage {start_info[0]} onwards...")
    for s_id in stages_to_clean:
        m_file = workspace_dir / f".stage_{s_id}_metrics.json"
        if m_file.exists():
            try:
                m_file.unlink()
            except Exception:
                pass
        targets = STAGE_OUTPUT_TARGETS.get(s_id, [])
        for target in targets:
            if target == "__OUTPUT_DIR__":
                if output_dir and output_dir.exists():
                    for f in output_dir.glob("*.md"):
                        try:
                            f.unlink()
                        except Exception:
                            pass
                    assets = output_dir / "assets"
                    if assets.exists():
                        shutil.rmtree(assets, ignore_errors=True)
            elif target == "__ASSETS_SIDECARS__":
                for a_dir in [workspace_dir / "08_chapters_graphics" / "assets", workspace_dir / "04_reduced_stream" / "assets", workspace_dir / "assets"]:
                    if a_dir.exists():
                        for txt_file in a_dir.glob("*.png.txt"):
                            try:
                                txt_file.unlink()
                            except Exception:
                                pass
            else:
                p = workspace_dir / target
                if p.is_dir():
                    shutil.rmtree(p, ignore_errors=True)
                elif p.is_file():
                    try:
                        p.unlink(missing_ok=True)
                    except Exception:
                        pass

    # Reset stage status entries for stages being cleaned
    if status_file.exists():
        try:
            with open(status_file, "r", encoding="utf-8") as f:
                data = json.load(f)
            updated = {k: v for k, v in data.items() if k not in stages_to_clean}
            with open(status_file, "w", encoding="utf-8") as f:
                json.dump(updated, f, indent=2)
        except Exception:
            pass


def main():
    parser = argparse.ArgumentParser(description="Master 12-Stage PDF-to-Markdown Pipeline Orchestrator")
    parser.add_argument("--pdf", type=str, help="Path to input technical PDF document")
    parser.add_argument("--workspace", type=str, default=None, help="Workspace directory for intermediate data (defaults to <book_dir>/workspace)")
    parser.add_argument("--output-dir", type=str, default=None, help="Output directory for generated Markdown files (defaults to <book_dir>/output_markdown)")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")
    parser.add_argument("--stage", type=str, help="Run single stage by number (e.g. 01, 06)")
    parser.add_argument("--from-stage", type=str, help="Start pipeline from stage number (e.g. 03)")
    parser.add_argument("--to-stage", type=str, help="End pipeline at stage number (e.g. 08)")
    parser.add_argument("--max-pages", type=int, help="Limit number of pages processed in Stage 01")
    parser.add_argument("--page-range", type=str, help="Page range to process in Stage 01 (e.g. 173-178 or 173..178)")
    parser.add_argument("--pages", dest="pages_alt", type=str, default=None, help="Discrete pages or ranges (e.g. '16,17,18, 32,37,50,73')")
    parser.add_argument("--start-page", type=int, help="Start page number for Stage 01 (1-indexed)")
    parser.add_argument("--end-page", type=int, help="End page number for Stage 01 (1-indexed)")
    parser.add_argument("--resume", action="store_true", help="Resume from last successfully completed stage")
    parser.add_argument("--verbose", action="store_true", help="Enable verbose command printing")
    parser.add_argument("--status", action="store_true", help="Display summary status of workspace and task items")
    parser.add_argument("--prepare-stage", type=str, help="Prepare task items for cognitive stage (06, 07, 08, 09)")
    parser.add_argument("--apply-stage", type=str, help="Apply Agent's edited task items for cognitive stage (06, 07, 08, 09)")
    parser.add_argument("--run-deterministic", action="store_true", help="Run deterministic stages (01, 03, 04, 05, 10, 11)")

    args = parser.parse_args()

    skill_dir = Path(__file__).resolve().parent
    config_path = Path(args.config)
    if not config_path.is_absolute():
        config_path = skill_dir / config_path

    config = load_config(config_path)

    pdf_path = Path(args.pdf) if args.pdf else None
    if pdf_path and not pdf_path.is_absolute():
        pdf_path = Path.cwd() / pdf_path
    book_dir = pdf_path.parent if pdf_path else None

    # Default workspace and output directories to the book's directory if PDF is provided
    if args.workspace:
        workspace_dir = Path(args.workspace)
        if not workspace_dir.is_absolute():
            workspace_dir = Path.cwd() / workspace_dir
    else:
        workspace_dir = (book_dir / "workspace") if book_dir else (Path.cwd() / "workspace")
    workspace_dir.mkdir(parents=True, exist_ok=True)

    output_dir = None
    if args.output_dir:
        output_dir = Path(args.output_dir)
        if not output_dir.is_absolute():
            output_dir = Path.cwd() / output_dir
        output_dir.mkdir(parents=True, exist_ok=True)

    if args.status:
        print_pipeline_status(workspace_dir, output_dir)
        return

    # Handle prepare / apply stage shortcuts
    if args.prepare_stage:
        target_idx = resolve_stage_idx(args.prepare_stage)
        target_info = STAGE_DEFINITIONS[target_idx] if target_idx is not None else None
        if not target_info:
            print(f"[!] Unknown stage: {args.prepare_stage}", file=sys.stderr)
            sys.exit(1)
        script = skill_dir / "stages" / target_info[1] / target_info[2]
        cmd = [sys.executable, str(script), "--workspace", str(workspace_dir), "--config", str(config_path), "--prepare"]
        subprocess.run(cmd, check=True)
        return

    if args.apply_stage:
        target_idx = resolve_stage_idx(args.apply_stage)
        target_info = STAGE_DEFINITIONS[target_idx] if target_idx is not None else None
        if not target_info:
            print(f"[!] Unknown stage: {args.apply_stage}", file=sys.stderr)
            sys.exit(1)
        script = skill_dir / "stages" / target_info[1] / target_info[2]
        cmd = [sys.executable, str(script), "--workspace", str(workspace_dir), "--config", str(config_path), "--apply"]
        subprocess.run(cmd, check=True)
        return

    status_file = workspace_dir / "stage_status.json"

    # Determine stages to run (as list of 0-based indices into STAGE_DEFINITIONS)
    if args.run_deterministic:
        stages_to_run = [i for i, s in enumerate(STAGE_DEFINITIONS) if s[0] in ("01", "01b", "03", "04", "05", "10", "11")]
    elif args.stage:
        target_idx = resolve_stage_idx(args.stage)
        if target_idx is None:
            print(f"[!] Unknown stage: {args.stage}", file=sys.stderr)
            sys.exit(1)
        stages_to_run = [target_idx]
    elif args.from_stage or args.to_stage:
        s_start = resolve_stage_idx(args.from_stage) if args.from_stage else 0
        s_end = resolve_stage_idx(args.to_stage) if args.to_stage else (len(STAGE_DEFINITIONS) - 1)
        if s_start is None or s_end is None:
            print(f"[!] Invalid stage range: {args.from_stage} to {args.to_stage}", file=sys.stderr)
            sys.exit(1)
        stages_to_run = list(range(s_start, s_end + 1))
    elif args.resume:
        last_completed_idx = get_last_completed_stage_idx(status_file)
        start_idx = last_completed_idx + 1
        if start_idx >= len(STAGE_DEFINITIONS):
            print("[*] All pipeline stages are already completed successfully.")
            return
        stages_to_run = list(range(start_idx, len(STAGE_DEFINITIONS)))
        print(f"[*] Resuming from Stage {STAGE_DEFINITIONS[start_idx][0]} ({STAGE_DEFINITIONS[start_idx][1]})")
    else:
        stages_to_run = list(range(len(STAGE_DEFINITIONS)))

    # Clean and invalidate all downstream intermediate and output stages from stages_to_run[0] onwards
    clean_downstream_stages(workspace_dir, output_dir, stages_to_run[0], status_file)

    if 0 in stages_to_run and not pdf_path:
        # Check if pages already exist
        pages_exist = bool(list((workspace_dir / "01_preprocess").glob("page_*.png"))) or bool(list((workspace_dir / "01_pages").glob("page_*.png"))) or bool(list((workspace_dir / "pages").glob("page_*.png")))
        if not pages_exist:
            print("[!] Error: --pdf is required when running Stage 01 without existing preprocessed pages.", file=sys.stderr)
            sys.exit(1)
        else:
            print("[*] Note: Existing preprocessed pages found in workspace.")

    print(f"[*] PDF-to-Markdown Pipeline executing stages: {[STAGE_DEFINITIONS[i][0] for i in stages_to_run]}")
    print(f"    Workspace  : {workspace_dir}")
    if output_dir:
        print(f"    Output Dir : {output_dir}")
    if pdf_path:
        print(f"    Source PDF : {pdf_path}")

    for idx in stages_to_run:
        stage_info = STAGE_DEFINITIONS[idx]
        success = run_stage(
            stage_idx=idx + 1,
            stage_info=stage_info,
            skill_dir=skill_dir,
            workspace_dir=workspace_dir,
            pdf_path=pdf_path,
            output_dir=output_dir,
            config_path=config_path,
            verbose=args.verbose,
            max_pages=args.max_pages,
            page_range=args.page_range or args.pages_alt,
            start_page=args.start_page,
            end_page=args.end_page,
        )

        if not success:
            print(f"\n[!] Pipeline halted at Stage {stage_info[0]} due to failure.", file=sys.stderr)
            sys.exit(1)

    print("\n[+] Selected pipeline stages completed successfully!")


if __name__ == "__main__":
    main()

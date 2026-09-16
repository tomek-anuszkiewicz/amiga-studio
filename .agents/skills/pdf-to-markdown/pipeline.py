#!/usr/bin/env python3
"""
pipeline.py: Master CLI Orchestrator for the 12-Stage PDF-to-Markdown Pipeline.
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
import yaml


STAGE_DEFINITIONS = [
    ("01", "01_preprocess", "preprocess.py", "Deconstruct PDF into pages, PNGs, and text blocks"),
    ("02", "02_page_segmentation", "segment_page.py", "Vertical banding & zone classification"),
    ("03", "03_build_raw_stream", "build_stream.py", "Build raw stream & extract initial assets"),
    ("04", "04_stream_reduction", "reduce_stream.py", "Normalize stream: weld prose & de-hyphenate"),
    ("05", "05_chapter_partition", "partition_chapters.py", "Partition stream into numbered sections"),
    ("06", "06_detect_continuations", "detect_continuations.py", "Detect multi-page table/graphic continuations"),
    ("07", "07_transform_tables", "transform_tables.py", "Transform table nodes (GFM vs HTML table)"),
    ("08", "08_transform_graphics", "transform_graphics.py", "Transform graphics (Mermaid vs SVG + RAG sidecars)"),
    ("09", "09_transform_prose", "format_prose.py", "Format prose/code and tag TOC with TOC34534"),
    ("10", "10_emit_markdown", "emit_markdown.py", "Emit per-section Markdown files (suppressing toc_header)"),
    ("11", "11_link_toc", "link_toc.py", "Fuzzy header matching & TOC wikilink conversion"),
    ("12", "12_refine_first_chapter_name", "refine_name.py", "Refine canonical name of first chapter"),
    ("13", "13_proofread_markdown", "proofread_markdown.py", "Proofread Markdown for OCR glitches & typos"),
]


def load_config(config_path: Path) -> dict:
    if config_path.exists():
        with open(config_path, "r", encoding="utf-8") as f:
            return yaml.safe_load(f) or {}
    return {}


def update_status(status_file: Path, stage_num: str, status: str, details: str = ""):
    data = {}
    if status_file.exists():
        try:
            with open(status_file, "r", encoding="utf-8") as f:
                data = json.load(f)
        except Exception:
            data = {}
    data[stage_num] = {"status": status, "details": details}
    status_file.parent.mkdir(parents=True, exist_ok=True)
    with open(status_file, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)


def get_last_completed_stage(status_file: Path) -> int:
    if not status_file.exists():
        return 0
    try:
        with open(status_file, "r", encoding="utf-8") as f:
            data = json.load(f)
        completed = [int(k) for k, v in data.items() if v.get("status") == "success" and k.isdigit()]
        return max(completed) if completed else 0
    except Exception:
        return 0


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
        cmd.extend(["--output-dir", str(workspace_dir / "10_emit_markdown")])
    elif stage_num == "11":
        cmd.extend([
            "--input-dir", str(workspace_dir / "10_emit_markdown"),
            "--output-dir", str(workspace_dir / "11_link_toc"),
        ])
    elif stage_num == "12":
        cmd.extend([
            "--input-dir", str(workspace_dir / "11_link_toc"),
            "--output-dir", str(workspace_dir / "12_refine_first_chapter_name"),
        ])
    elif stage_num == "13":
        dest_dir = output_dir if output_dir else (workspace_dir / "13_proofread_markdown")
        cmd.extend([
            "--input-dir", str(workspace_dir / "12_refine_first_chapter_name"),
            "--output-dir", str(dest_dir),
        ])

    if verbose:
        print(f"[CMD] {' '.join(cmd)}")

    status_file = workspace_dir / "stage_status.json"
    update_status(status_file, stage_num, "running")

    try:
        result = subprocess.run(cmd, check=True)
        update_status(status_file, stage_num, "success")
        return True
    except subprocess.CalledProcessError as e:
        print(f"[!] Stage {stage_num} failed with return code {e.returncode}", file=sys.stderr)
        update_status(status_file, stage_num, "failed", f"Exit code {e.returncode}")
        return False
    except Exception as e:
        print(f"[!] Stage {stage_num} encountered exception: {e}", file=sys.stderr)
        update_status(status_file, stage_num, "failed", str(e))
        return False


def print_pipeline_status(workspace_dir: Path, output_dir: Path):
    print("\n==================================================")
    print("         PDF-to-Markdown Pipeline Status          ")
    print("==================================================")

    # 1. 01_preprocess (01)
    p1 = workspace_dir / "01_preprocess" if (workspace_dir / "01_preprocess").exists() else (workspace_dir / "01_pages")
    p1_count = len(list(p1.glob("page_*.png"))) if p1.exists() else 0
    print(f"[*] 01_preprocess                 : {p1_count} rendered PNGs")

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

    # 10. 10_emit_markdown (10)
    p10 = workspace_dir / "10_emit_markdown" if (workspace_dir / "10_emit_markdown").exists() else (workspace_dir / "10_markdown_raw")
    p10_count = len(list(p10.glob("*.md"))) if p10.exists() else 0
    print(f"[*] 10_emit_markdown              : {p10_count} files")

    # 11. 11_link_toc (11)
    p11 = workspace_dir / "11_link_toc" if (workspace_dir / "11_link_toc").exists() else (workspace_dir / "11_markdown_linked")
    p11_count = len(list(p11.glob("*.md"))) if p11.exists() else 0
    print(f"[*] 11_link_toc                   : {p11_count} files")

    # 12. 12_refine_first_chapter_name (12)
    p12 = workspace_dir / "12_refine_first_chapter_name" if (workspace_dir / "12_refine_first_chapter_name").exists() else (workspace_dir / "12_canonical_markdown")
    p12_count = len(list(p12.glob("*.md"))) if p12.exists() else 0
    print(f"[*] 12_refine_first_chapter_name  : {p12_count} files")

    # 13. 13_proofread_markdown (13)
    p13 = workspace_dir / "13_proofread_markdown"
    p13_count = len(list(p13.glob("*.md"))) if p13.exists() else 0
    print(f"[*] 13_proofread_markdown         : {p13_count} files")

    # Final Output Markdown (if custom output_dir used)
    if output_dir:
        md_count = len(list(output_dir.glob("*.md"))) if output_dir.exists() else 0
        print(f"[*] Custom Output Dir             : {md_count} files in {output_dir.name}/")
    print("==================================================\n")


STAGE_OUTPUT_TARGETS = {
    1: ["01_preprocess", "01_pages", "pages", "pages_manifest.json", "manifest.json"],
    2: ["02_page_segmentation", "02_segments", "segments"],
    3: ["03_build_raw_stream", "03_raw_stream", "raw_stream.json", "assets"],
    4: ["04_stream_reduction", "04_reduced_stream", "reduced_stream.json"],
    5: ["05_chapter_partition", "05_chapters_raw", "chapters", "chapters_manifest.json"],
    6: ["06_detect_continuations", "06_chapters_continuations", "tasks/continuations"],
    7: ["07_transform_tables", "07_chapters_tables", "tasks/tables"],
    8: ["08_transform_graphics", "08_chapters_graphics", "tasks/graphics", "__ASSETS_SIDECARS__"],
    9: ["09_transform_prose", "09_chapters_formatted", "tasks/prose"],
    10: ["10_emit_markdown", "10_markdown_raw"],
    11: ["11_link_toc", "11_markdown_linked"],
    12: ["12_refine_first_chapter_name", "12_canonical_markdown"],
    13: ["13_proofread_markdown"],
}


def clean_downstream_stages(workspace_dir: Path, output_dir: Path, start_stage: int, status_file: Path):
    """
    Cleans all intermediate artifacts and output directories for all stages >= start_stage.
    Guarantees that re-running from stage N starts completely fresh without stale downstream files.
    """
    import shutil
    print(f"[*] Invalidation: Wiping intermediate and output artifacts for stages {start_stage:02d} to 13...")
    for s in range(start_stage, 14):
        targets = STAGE_OUTPUT_TARGETS.get(s, [])
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

    # Reset stage status entries for stages >= start_stage
    if status_file.exists():
        try:
            with open(status_file, "r", encoding="utf-8") as f:
                data = json.load(f)
            updated = {k: v for k, v in data.items() if not (k.isdigit() and int(k) >= start_stage)}
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
        stg = f"{int(args.prepare_stage):02d}"
        target_info = next((s for s in STAGE_DEFINITIONS if s[0] == stg), None)
        if not target_info:
            print(f"[!] Unknown stage: {args.prepare_stage}", file=sys.stderr)
            sys.exit(1)
        script = skill_dir / "stages" / target_info[1] / target_info[2]
        cmd = [sys.executable, str(script), "--workspace", str(workspace_dir), "--config", str(config_path), "--prepare"]
        subprocess.run(cmd, check=True)
        return

    if args.apply_stage:
        stg = f"{int(args.apply_stage):02d}"
        target_info = next((s for s in STAGE_DEFINITIONS if s[0] == stg), None)
        if not target_info:
            print(f"[!] Unknown stage: {args.apply_stage}", file=sys.stderr)
            sys.exit(1)
        script = skill_dir / "stages" / target_info[1] / target_info[2]
        cmd = [sys.executable, str(script), "--workspace", str(workspace_dir), "--config", str(config_path), "--apply"]
        subprocess.run(cmd, check=True)
        return

    status_file = workspace_dir / "stage_status.json"

    # Determine stages to run
    if args.run_deterministic:
        stages_to_run = [1, 3, 4, 5, 10, 11]
    elif args.stage:
        target = int(args.stage)
        stages_to_run = [target]
    elif args.from_stage or args.to_stage:
        s_start = int(args.from_stage) if args.from_stage else 1
        s_end = int(args.to_stage) if args.to_stage else len(STAGE_DEFINITIONS)
        stages_to_run = list(range(s_start, s_end + 1))
    elif args.resume:
        last_completed = get_last_completed_stage(status_file)
        stages_to_run = list(range(min(last_completed + 1, len(STAGE_DEFINITIONS)), len(STAGE_DEFINITIONS) + 1))
        print(f"[*] Resuming from Stage {stages_to_run[0]:02d}")
    else:
        stages_to_run = list(range(1, len(STAGE_DEFINITIONS) + 1))

    # Clean and invalidate all downstream intermediate and output stages from min(stages_to_run) onwards
    clean_downstream_stages(workspace_dir, output_dir, stages_to_run[0], status_file)

    if 1 in stages_to_run and not pdf_path:
        # Check if pages already exist
        pages_exist = bool(list((workspace_dir / "01_pages").glob("page_*.png"))) or bool(list((workspace_dir / "pages").glob("page_*.png")))
        if not pages_exist:
            print("[!] Error: --pdf is required when running Stage 01 without existing preprocessed pages.", file=sys.stderr)
            sys.exit(1)
        else:
            print("[*] Note: Existing preprocessed pages found in workspace.")

    print(f"[*] PDF-to-Markdown Pipeline executing stages: {[f'{s:02d}' for s in stages_to_run]}")
    print(f"    Workspace  : {workspace_dir}")
    if output_dir:
        print(f"    Output Dir : {output_dir}")
    if pdf_path:
        print(f"    Source PDF : {pdf_path}")

    for idx, stage_info in enumerate(STAGE_DEFINITIONS, start=1):
        if idx not in stages_to_run:
            continue

        success = run_stage(
            stage_idx=idx,
            stage_info=stage_info,
            skill_dir=skill_dir,
            workspace_dir=workspace_dir,
            pdf_path=pdf_path,
            output_dir=output_dir,
            config_path=config_path,
            verbose=args.verbose,
            max_pages=args.max_pages,
            page_range=args.page_range,
            start_page=args.start_page,
            end_page=args.end_page,
        )

        if not success:
            print(f"\n[!] Pipeline halted at Stage {stage_info[0]} due to failure.", file=sys.stderr)
            sys.exit(1)

    print("\n[+] Selected pipeline stages completed successfully!")


if __name__ == "__main__":
    main()

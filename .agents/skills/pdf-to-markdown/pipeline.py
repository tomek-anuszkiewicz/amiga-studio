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
        if max_pages:
            cmd.extend(["--max-pages", str(max_pages)])
    if stage_num in ("10", "11", "12"):
        cmd.extend(["--output-dir", str(output_dir)])

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


def main():
    parser = argparse.ArgumentParser(description="Master 12-Stage PDF-to-Markdown Pipeline Orchestrator")
    parser.add_argument("--pdf", type=str, help="Path to input technical PDF document")
    parser.add_argument("--workspace", type=str, default="workspace", help="Workspace directory for intermediate data")
    parser.add_argument("--output-dir", type=str, default="output_markdown", help="Output directory for generated Markdown files")
    parser.add_argument("--config", type=str, default="config.yaml", help="Path to config.yaml")
    parser.add_argument("--stage", type=str, help="Run single stage by number (e.g. 01, 06)")
    parser.add_argument("--from-stage", type=str, help="Start pipeline from stage number (e.g. 03)")
    parser.add_argument("--to-stage", type=str, help="End pipeline at stage number (e.g. 08)")
    parser.add_argument("--max-pages", type=int, help="Limit number of pages processed in Stage 01")
    parser.add_argument("--resume", action="store_true", help="Resume from last successfully completed stage")
    parser.add_argument("--verbose", action="store_true", help="Enable verbose command printing")

    args = parser.parse_args()

    skill_dir = Path(__file__).resolve().parent
    config_path = Path(args.config)
    if not config_path.is_absolute():
        config_path = skill_dir / config_path

    config = load_config(config_path)

    workspace_dir = Path(args.workspace)
    if not workspace_dir.is_absolute():
        workspace_dir = skill_dir / workspace_dir
    workspace_dir.mkdir(parents=True, exist_ok=True)

    output_dir = Path(args.output_dir)
    if not output_dir.is_absolute():
        output_dir = skill_dir / output_dir
    output_dir.mkdir(parents=True, exist_ok=True)

    pdf_path = Path(args.pdf) if args.pdf else None
    if pdf_path and not pdf_path.is_absolute():
        pdf_path = Path.cwd() / pdf_path

    status_file = workspace_dir / "stage_status.json"

    # Determine which stages to run
    start_stage = 1
    end_stage = len(STAGE_DEFINITIONS)

    if args.stage:
        target = int(args.stage)
        start_stage = target
        end_stage = target
    elif args.from_stage or args.to_stage:
        if args.from_stage:
            start_stage = int(args.from_stage)
        if args.to_stage:
            end_stage = int(args.to_stage)
    elif args.resume:
        last_completed = get_last_completed_stage(status_file)
        start_stage = min(last_completed + 1, len(STAGE_DEFINITIONS))
        print(f"[*] Resuming from Stage {start_stage:02d} (last completed: {last_completed:02d})")

    if start_stage == 1 and not pdf_path:
        print("[!] Error: --pdf is required when running Stage 01", file=sys.stderr)
        sys.exit(1)

    print(f"[*] PDF-to-Markdown Pipeline initialized.")
    print(f"    Target range : Stage {start_stage:02d} to Stage {end_stage:02d}")
    print(f"    Workspace    : {workspace_dir}")
    print(f"    Output Dir   : {output_dir}")
    if pdf_path:
        print(f"    Source PDF   : {pdf_path}")

    for idx, stage_info in enumerate(STAGE_DEFINITIONS, start=1):
        if idx < start_stage or idx > end_stage:
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
        )

        if not success:
            print(f"\n[!] Pipeline halted at Stage {stage_info[0]} due to failure.", file=sys.stderr)
            sys.exit(1)

    print("\n[+] Pipeline execution completed successfully!")


if __name__ == "__main__":
    main()

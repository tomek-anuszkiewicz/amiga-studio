#!/usr/bin/env python3
"""
pipeline.py: Master CLI Orchestrator for the 14-Stage PDF-to-Markdown Pipeline.
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Optional, List, Dict, Any
import yaml

STAGE_REGISTRY: List[Dict[str, Any]] = [
    {
        "id": "01",
        "dir": "01_preprocess",
        "script": "preprocess.py",
        "desc": "Deconstruct PDF into pages, PNGs, and text blocks (with auto-OCR)",
        "targets": ["01_preprocess", "pages_manifest.json"],
        "inspect": ("*.png", "rendered PNGs"),
    },
    {
        "id": "02",
        "dir": "02_page_segmentation",
        "script": "segment_page.py",
        "desc": "Vertical banding & zone classification",
        "targets": ["02_page_segmentation"],
        "inspect": ("page_*_segments.json", "segment JSON files"),
    },
    {
        "id": "03",
        "dir": "03_build_raw_stream",
        "script": "build_stream.py",
        "desc": "Build raw stream & extract initial assets",
        "targets": ["03_build_raw_stream"],
        "inspect": ("raw_stream.json", "stream file"),
    },
    {
        "id": "04",
        "dir": "04_stream_reduction",
        "script": "reduce_stream.py",
        "desc": "Normalize stream: weld prose & de-hyphenate",
        "targets": ["04_stream_reduction"],
        "inspect": ("reduced_stream.json", "stream file"),
    },
    {
        "id": "05",
        "dir": "05_chapter_partition",
        "script": "partition_chapters.py",
        "desc": "Partition stream into numbered sections",
        "targets": ["05_chapter_partition", "chapters_manifest.json"],
        "inspect": ("*.json", "chapter stream files"),
    },
    {
        "id": "06",
        "dir": "06_detect_continuations",
        "script": "detect_continuations.py",
        "desc": "Detect multi-page table/graphic continuations",
        "targets": ["06_detect_continuations"],
        "inspect": ("*.json", "chapter stream files"),
    },
    {
        "id": "07",
        "dir": "07_transform_tables",
        "script": "transform_tables.py",
        "desc": "Transform table nodes (GFM vs HTML table)",
        "targets": ["07_transform_tables"],
        "inspect": ("*.json", "chapter stream files"),
    },
    {
        "id": "08",
        "dir": "08_transform_graphics",
        "script": "transform_graphics.py",
        "desc": "Transform graphics (Mermaid vs SVG + RAG sidecars)",
        "targets": ["08_transform_graphics", "__ASSETS_SIDECARS__"],
        "inspect": ("*.json", "chapter stream files"),
    },
    {
        "id": "09",
        "dir": "09_transform_prose",
        "script": "format_prose.py",
        "desc": "Format prose/code and tag TOC with TOC34534",
        "targets": ["09_transform_prose"],
        "inspect": ("*.json", "chapter stream files"),
    },
    {
        "id": "10",
        "dir": "10_proofread_stream",
        "script": "proofread_stream.py",
        "desc": "Proofread chapter streams & manifest with LLM",
        "targets": ["10_proofread_stream"],
        "inspect": ("*.json", "chapter stream files"),
    },
    {
        "id": "11",
        "dir": "11_emit_markdown",
        "script": "emit_markdown.py",
        "desc": "Emit per-section Markdown files (suppressing toc_header)",
        "targets": ["11_emit_markdown"],
        "inspect": ("*.md", "Markdown files"),
    },
    {
        "id": "12",
        "dir": "12_generate_properties",
        "script": "generate_properties.py",
        "desc": "Generate publication-grade Obsidian YAML properties with LLM",
        "targets": ["12_generate_properties"],
        "inspect": ("*.md", "Markdown files"),
    },
    {
        "id": "13",
        "dir": "13_refine_first_chapter_name",
        "script": "refine_name.py",
        "desc": "Refine canonical name of first chapter",
        "targets": ["13_refine_first_chapter_name"],
        "inspect": ("*.md", "Markdown files"),
    },
    {
        "id": "14",
        "dir": "14_link_toc",
        "script": "link_toc.py",
        "desc": "Fuzzy header matching & TOC wikilink conversion",
        "targets": ["14_link_toc", "__OUTPUT_DIR__"],
        "inspect": ("*.md", "Markdown files"),
    },
]

STAGE_DEFINITIONS = [(s["id"], s["dir"], s["script"], s["desc"]) for s in STAGE_REGISTRY]
STAGE_OUTPUT_TARGETS = {s["id"]: s["targets"] for s in STAGE_REGISTRY}


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
            pass
    entry = data.setdefault(stage_num, {})
    entry["status"] = status
    if details or "details" not in entry:
        entry["details"] = details
    if duration_seconds is not None or "duration_seconds" not in entry:
        entry["duration_seconds"] = duration_seconds
    if llm_calls is not None or "llm_calls" not in entry:
        entry["llm_calls"] = llm_calls if llm_calls is not None else 0

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
        for idx, s in enumerate(STAGE_REGISTRY):
            if data.get(s["id"], {}).get("status") == "success":
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
    for idx, s in enumerate(STAGE_REGISTRY):
        s_id = s["id"].lower()
        s_dir = s["dir"].lower()
        if val in (s_id, s_dir, s_id.lstrip("0")):
            return idx
        if val.isdigit() and int(val) == int(s["id"]):
            return idx
    return None


def run_stage(
    stage_info: dict,
    skill_dir: Path,
    workspace_dir: Path,
    pdf_path: Optional[Path],
    output_dir: Optional[Path],
    config_path: Path,
    verbose: bool = False,
    page_ranges: Optional[str] = None,
) -> bool:
    stage_num = stage_info["id"]
    stage_dir_name = stage_info["dir"]
    script_path = skill_dir / "stages" / stage_dir_name / stage_info["script"]

    print(f"\n==================================================")
    print(f"[*] Stage {stage_num}: {stage_dir_name} ({stage_info['desc']})")
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

    # Stage-specific standard parameter injection
    if stage_num == "01":
        cmd.extend(["--pdf", str(pdf_path)])
        if page_ranges:
            cmd.extend(["--page-ranges", str(page_ranges)])
    elif stage_num == "11":
        cmd.extend(["--output-dir", str(workspace_dir / "11_emit_markdown")])
    elif stage_num == "14" and output_dir:
        cmd.extend(["--output-dir", str(output_dir)])

    if verbose:
        print(f"[CMD] {' '.join(cmd)}")

    status_file = workspace_dir / "stage_status.json"
    stage_dir = workspace_dir / stage_dir_name
    stage_dir.mkdir(parents=True, exist_ok=True)
    stage_metrics_file = stage_dir / ".metrics.json"

    try:
        stage_metrics_file.unlink(missing_ok=True)
        with open(stage_metrics_file, "w", encoding="utf-8") as f:
            json.dump({"llm_calls": 0}, f)
    except Exception:
        pass

    env = os.environ.copy()
    env["LLM_STAGE_METRICS_FILE"] = str(stage_metrics_file)

    def read_and_clean_metrics() -> int:
        calls = 0
        if stage_metrics_file.exists():
            try:
                with open(stage_metrics_file, "r", encoding="utf-8") as f:
                    calls = json.load(f).get("llm_calls", 0)
                stage_metrics_file.unlink(missing_ok=True)
            except Exception:
                pass
        return calls

    update_status(status_file, stage_num, "running")
    start_time = time.time()
    try:
        subprocess.run(cmd, env=env, check=True)
        duration = round(time.time() - start_time, 2)
        calls = read_and_clean_metrics()
        update_status(status_file, stage_num, "success", duration_seconds=duration, llm_calls=calls)
        print(f"[*] Stage {stage_num} finished in {duration:.2f}s with {calls} LLM call(s).")
        return True
    except subprocess.CalledProcessError as e:
        duration = round(time.time() - start_time, 2)
        calls = read_and_clean_metrics()
        print(f"[!] Stage {stage_num} failed with return code {e.returncode} ({duration:.2f}s, {calls} LLM calls)", file=sys.stderr)
        update_status(status_file, stage_num, "failed", f"Exit code {e.returncode}", duration_seconds=duration, llm_calls=calls)
        return False
    except Exception as e:
        duration = round(time.time() - start_time, 2)
        calls = read_and_clean_metrics()
        print(f"[!] Stage {stage_num} encountered exception: {e} ({duration:.2f}s, {calls} LLM calls)", file=sys.stderr)
        update_status(status_file, stage_num, "failed", str(e), duration_seconds=duration, llm_calls=calls)
        return False


def print_pipeline_status(workspace_dir: Path, output_dir: Optional[Path]):
    print("\n==================================================")
    print("         PDF-to-Markdown Pipeline Status          ")
    print("==================================================")

    for s in STAGE_REGISTRY:
        dir_name = s["dir"]
        pattern, label = s["inspect"]
        target = workspace_dir / dir_name
        if "*" in pattern:
            count = len(list(target.glob(pattern))) if target.exists() else 0
            print(f"[*] {dir_name:<30}: {count} {label}")
        else:
            file_path = target / pattern
            if file_path.exists():
                print(f"[*] {dir_name:<30}: OK ({file_path.stat().st_size} B)")
            else:
                print(f"[*] {dir_name:<30}: Missing")

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
                total_duration, total_calls = 0.0, 0
                for s in STAGE_REGISTRY:
                    num = s["id"]
                    if num in status_data:
                        st = status_data[num]
                        status = st.get("status", "unknown")
                        dur = st.get("duration_seconds")
                        dur_str = f"{dur:.2f}s" if dur is not None else "-"
                        if dur is not None:
                            total_duration += dur
                        calls = st.get("llm_calls", 0)
                        total_calls += calls
                        print(f"Stage {num} ({s['dir']:<28}): {status:<8} | Time: {dur_str:>8} | LLM: {calls:>4} call(s)")
                print(f"Total Measured Time: {total_duration:.2f}s | Total LLM Calls: {total_calls}")
                print("--------------------------------------------------")
        except Exception:
            pass

    print("==================================================\n")


def clean_downstream_stages(workspace_dir: Path, output_dir: Optional[Path], start_idx: int, status_file: Path):
    stages_to_clean = [s["id"] for s in STAGE_REGISTRY[start_idx:]]
    print(f"[*] Invalidation: Wiping intermediate and output artifacts from Stage {stages_to_clean[0]} onwards...")
    for s in STAGE_REGISTRY[start_idx:]:
        s_id = s["id"]
        (workspace_dir / s["dir"] / ".metrics.json").unlink(missing_ok=True)
        (workspace_dir / f".stage_{s_id}_metrics.json").unlink(missing_ok=True)
        for target in s["targets"]:
            if target == "__OUTPUT_DIR__":
                if output_dir and output_dir.exists():
                    for f in output_dir.glob("*.md"):
                        f.unlink(missing_ok=True)
                    shutil.rmtree(output_dir / "assets", ignore_errors=True)
            elif target == "__ASSETS_SIDECARS__":
                for a_dir in [workspace_dir / "08_transform_graphics" / "assets", workspace_dir / "04_stream_reduction" / "assets", workspace_dir / "assets"]:
                    if a_dir.exists():
                        for txt_file in a_dir.glob("*.png.txt"):
                            txt_file.unlink(missing_ok=True)
            else:
                p = workspace_dir / target
                if p.is_dir():
                    shutil.rmtree(p, ignore_errors=True)
                elif p.is_file():
                    p.unlink(missing_ok=True)

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
    parser = argparse.ArgumentParser(description="Master 14-Stage PDF-to-Markdown Pipeline Orchestrator")
    parser.add_argument("--pdf", type=str, help="Path to input technical PDF document")
    parser.add_argument("--workspace", type=str, default=None, help="Workspace directory for intermediate data")
    parser.add_argument("--output-dir", type=str, default=None, help="Output directory for generated Markdown files")
    parser.add_argument("--config", type=str, required=True, help="Path to YAML configuration file")
    parser.add_argument("--from-stage", type=str, help="Start pipeline from stage number (e.g. 03)")
    parser.add_argument("--to-stage", type=str, help="End pipeline at stage number (e.g. 08)")
    parser.add_argument("--page-ranges", type=str, default=None, help="Page ranges or discrete pages to process in Stage 01 (e.g. '1-5, 7, 8, 10-15' or '1..5')")
    parser.add_argument("--resume", action="store_true", help="Resume from last successfully completed stage")
    parser.add_argument("--verbose", action="store_true", help="Enable verbose command printing")
    parser.add_argument("--status", action="store_true", help="Display summary status of workspace and task items")
    parser.add_argument("--prepare-stage", type=str, help="Prepare task items for cognitive stage (06, 07, 08, 09)")
    parser.add_argument("--apply-stage", type=str, help="Apply Agent's edited task items for cognitive stage (06, 07, 08, 09)")
    parser.add_argument("--run-deterministic", action="store_true", help="Run deterministic stages (01, 03, 04, 05, 10, 11)")

    args = parser.parse_args()

    skill_dir = Path(__file__).resolve().parent
    config_source_path = Path(args.config)
    if not config_source_path.is_absolute():
        if not config_source_path.exists():
            skill_relative = skill_dir / config_source_path
            if skill_relative.exists():
                config_source_path = skill_relative

    if not config_source_path.is_file():
        print(f"[!] Error: Config file not found: {config_source_path}", file=sys.stderr)
        sys.exit(1)

    try:
        with open(config_source_path, "r", encoding="utf-8") as f:
            raw_config = yaml.safe_load(f)
        if not raw_config or not isinstance(raw_config, dict):
            print(f"[!] Error: Config file is empty or invalid YAML: {config_source_path}", file=sys.stderr)
            sys.exit(1)
    except Exception as e:
        print(f"[!] Error reading config file {config_source_path}: {e}", file=sys.stderr)
        sys.exit(1)

    pdf_path = Path(args.pdf).resolve() if args.pdf else None
    if pdf_path and not pdf_path.is_file():
        if pdf_path.is_dir():
            book_dir = pdf_path
            candidates = list(pdf_path.glob("*.pdf"))
            if len(candidates) == 1:
                pdf_path = candidates[0]
        elif pdf_path.parent.is_dir():
            book_dir = pdf_path.parent
            candidates = list(pdf_path.parent.glob("*.pdf"))
            if len(candidates) == 1:
                pdf_path = candidates[0]
        else:
            book_dir = None
    else:
        book_dir = pdf_path.parent if pdf_path else None

    if args.workspace:
        workspace_dir = Path(args.workspace).resolve()
    else:
        workspace_dir = (book_dir / "workspace") if book_dir else (Path.cwd() / "workspace")
    workspace_dir.mkdir(parents=True, exist_ok=True)

    # Snapshot config into workspace
    workspace_config_path = workspace_dir / "config.yaml"
    if config_source_path.resolve() != workspace_config_path.resolve():
        try:
            shutil.copyfile(config_source_path, workspace_config_path)
        except Exception as e:
            print(f"[!] Error snapshotting config to workspace {workspace_config_path}: {e}", file=sys.stderr)
            sys.exit(1)
    config_path = workspace_config_path

    output_dir = None
    if args.output_dir:
        output_dir = Path(args.output_dir).resolve()
        output_dir.mkdir(parents=True, exist_ok=True)
    elif book_dir:
        output_dir = book_dir.resolve()
        output_dir.mkdir(parents=True, exist_ok=True)

    if args.status:
        print_pipeline_status(workspace_dir, output_dir)
        return

    # Handle prepare / apply stage shortcuts
    if args.prepare_stage or args.apply_stage:
        stage_arg = args.prepare_stage or args.apply_stage
        target_idx = resolve_stage_idx(stage_arg)
        if target_idx is None:
            print(f"[!] Unknown stage: {stage_arg}", file=sys.stderr)
            sys.exit(1)
        s = STAGE_REGISTRY[target_idx]
        mode_flag = "--prepare" if args.prepare_stage else "--apply"
        script = skill_dir / "stages" / s["dir"] / s["script"]
        cmd = [sys.executable, str(script), "--workspace", str(workspace_dir), "--config", str(config_path), mode_flag]
        subprocess.run(cmd, check=True)
        return

    status_file = workspace_dir / "stage_status.json"

    if args.run_deterministic:
        stages_to_run = [i for i, s in enumerate(STAGE_REGISTRY) if s["id"] in ("01", "03", "04", "05", "10", "11")]
    elif args.from_stage or args.to_stage:
        s_start = resolve_stage_idx(args.from_stage) if args.from_stage else 0
        s_end = resolve_stage_idx(args.to_stage) if args.to_stage else (len(STAGE_REGISTRY) - 1)
        if s_start is None or s_end is None:
            print(f"[!] Invalid stage range: {args.from_stage} to {args.to_stage}", file=sys.stderr)
            sys.exit(1)
        stages_to_run = list(range(s_start, s_end + 1))
    elif args.resume:
        last_completed_idx = get_last_completed_stage_idx(status_file)
        start_idx = last_completed_idx + 1
        if start_idx >= len(STAGE_REGISTRY):
            print("[*] All pipeline stages are already completed successfully.")
            return
        stages_to_run = list(range(start_idx, len(STAGE_REGISTRY)))
        print(f"[*] Resuming from Stage {STAGE_REGISTRY[start_idx]['id']} ({STAGE_REGISTRY[start_idx]['dir']})")
    else:
        stages_to_run = list(range(len(STAGE_REGISTRY)))

    clean_downstream_stages(workspace_dir, output_dir, stages_to_run[0], status_file)

    if 0 in stages_to_run and not pdf_path:
        pages_exist = bool(list((workspace_dir / "01_preprocess").glob("page_*.png")))
        if not pages_exist:
            print("[!] Error: --pdf is required when running Stage 01 without existing preprocessed pages.", file=sys.stderr)
            sys.exit(1)
        else:
            print("[*] Note: Existing preprocessed pages found in workspace.")

    print(f"[*] PDF-to-Markdown Pipeline executing stages: {[STAGE_REGISTRY[i]['id'] for i in stages_to_run]}")
    print(f"    Workspace  : {workspace_dir}")
    if output_dir:
        print(f"    Output Dir : {output_dir}")
    if pdf_path:
        print(f"    Source PDF : {pdf_path}")

    for idx in stages_to_run:
        s_info = STAGE_REGISTRY[idx]
        success = run_stage(
            stage_info=s_info,
            skill_dir=skill_dir,
            workspace_dir=workspace_dir,
            pdf_path=pdf_path,
            output_dir=output_dir,
            config_path=config_path,
            verbose=args.verbose,
            page_ranges=args.page_ranges,
        )
        if not success:
            print(f"\n[!] Pipeline halted at Stage {s_info['id']} due to failure.", file=sys.stderr)
            sys.exit(1)

    print("\n[+] Selected pipeline stages completed successfully!")


if __name__ == "__main__":
    main()

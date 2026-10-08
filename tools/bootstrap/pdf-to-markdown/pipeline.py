#!/usr/bin/env python3
"""
pipeline.py: Master CLI orchestrator for PDF page preparation and table transcription.
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

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from common.pdf_config import load_config as parse_config
from common.pdf_artifacts import read_json, write_json, prepared_pdf_path
from common.pdf_selection import selected_pages

STAGE_REGISTRY: List[Dict[str, Any]] = [
    {
        "id": "01",
        "dir": "01_text_layer",
        "script": "prepare_text_layer.py",
        "desc": "Prepare a shared full-source OCR PDF, or reuse the existing file",
        "targets": ["01_text_layer"],
        "inspect": ("recovery/*.json", "OCR recovery records"),
    },
    {
        "id": "02",
        "dir": "02_preprocess",
        "script": "preprocess.py",
        "desc": "Extract positioned text and matching PNGs from the prepared PDF",
        "targets": ["02_preprocess"],
        "inspect": ("*.png", "rendered PNGs"),
    },
    {
        "id": "03", "dir": "03_page_conversion", "script": "convert_page.py",
        "desc": "Independently group page objects and convert their text to Markdown",
        "targets": ["03_page_conversion"],
        "inspect": ("page_*_segments.json", "page conversion JSON files"),
    },
    {
        "id": "04", "dir": "04_filter_page_content", "script": "filter_page_content.py",
        "desc": "Remove objects classified as headers or footers",
        "targets": ["04_filter_page_content"],
        "inspect": ("page_*_segments.json", "filtered page conversion JSON files"),
    },
    {
        "id": "05", "dir": "05_transform_page_tables", "script": "transform_page_tables.py",
        "desc": "Replace table content with Markdown or HTML",
        "targets": ["05_transform_page_tables"],
        "inspect": ("page_*_segments.json", "transcribed table page JSON files"),
    },
]

# Each stage reads explicitly named predecessor artifacts.
for stage in STAGE_REGISTRY:
    stage["inputs"] = {
        "01": [], "02": ["01"], "03": ["02"], "04": ["02", "03"],
        "05": ["02", "04"],
    }[stage["id"]]


def invalidated_stages(start_idx):
    return {stage["id"] for stage in STAGE_REGISTRY[start_idx:]}


def update_status(
    status_file: Path,
    stage_num: str,
    status: str,
    details: str = "",
    duration_seconds: Optional[float] = None,
    llm_calls: Optional[int] = None,
    llm_cached_calls: Optional[int] = None,
    llm_time_seconds: Optional[float] = None,
    page_ranges: Optional[str] = None,
):
    data = {}
    if status_file.exists():
        data = read_json(status_file)
    entry = data.setdefault(stage_num, {})
    entry["status"] = status
    if stage_num == "05":
        entry["page_ranges"] = page_ranges
    if details or "details" not in entry:
        entry["details"] = details
    if duration_seconds is not None or "duration_seconds" not in entry:
        entry["duration_seconds"] = duration_seconds
    if llm_calls is not None or "llm_calls" not in entry:
        entry["llm_calls"] = llm_calls if llm_calls is not None else 0
    if llm_cached_calls is not None or "llm_cached_calls" not in entry:
        entry["llm_cached_calls"] = llm_cached_calls if llm_cached_calls is not None else 0
    if llm_time_seconds is not None or "llm_time_seconds" not in entry:
        entry["llm_time_seconds"] = llm_time_seconds if llm_time_seconds is not None else 0.0

    status_file.parent.mkdir(parents=True, exist_ok=True)
    write_json(status_file, data)


def resolve_stage_idx(arg_val: str) -> Optional[int]:
    if not arg_val:
        return None
    val = str(arg_val).strip().lower()
    for idx, s in enumerate(STAGE_REGISTRY):
        s_id = s["id"].lower()
        s_dir = s["dir"].lower()
        if val in (s_id, s_dir, s_id.lstrip("0")):
            return idx
        if val.isdigit() and s["id"].isdigit() and int(val) == int(s["id"]):
            return idx
    return None


def run_stage(
    stage_info: dict,
    skill_dir: Path,
    workspace_dir: Path,
    pdf_path: Optional[Path],
    config_path: Path,
    verbose: bool = False,
    page_ranges: Optional[str] = None,
) -> bool:
    stage_num = stage_info["id"]
    stage_dir_name = stage_info["dir"]
    script_path = skill_dir / "stages" / stage_dir_name / stage_info["script"]

    print(f"\n==================================================")
    print(f"[*] Stage {stage_num}: {stage_dir_name} ({stage_info['desc']})")
    print(f"==================================================", flush=True)


    cmd = [
        sys.executable,
        str(script_path),
        "--workspace", str(workspace_dir),
        "--config", str(config_path),
    ]

    # Stage-specific standard parameter injection
    if stage_num == "01":
        cmd.extend(["--pdf", str(pdf_path)])
    elif stage_num == "02":
        prepared_pdf = prepared_pdf_path(pdf_path)
        cmd.extend(["--pdf", str(prepared_pdf)])
    if page_ranges is not None and stage_num in {"01", "02", "03", "04", "05"}:
        cmd.extend(["--page-ranges", page_ranges])

    if verbose:
        print(f"[CMD] {' '.join(cmd)}")

    status_file = workspace_dir / "stage_status.json"
    stage_dir = workspace_dir / stage_dir_name
    stage_dir.mkdir(parents=True, exist_ok=True)
    stage_metrics_file = stage_dir / ".metrics"

    try:
        stage_metrics_file.unlink(missing_ok=True)
        with open(stage_metrics_file, "w", encoding="utf-8") as f:
            json.dump({"llm_calls": 0, "llm_cached_calls": 0, "llm_time_seconds": 0.0}, f)
    except Exception:
        pass

    env = os.environ.copy()
    env["LLM_STAGE_METRICS_FILE"] = str(stage_metrics_file)

    def read_and_clean_metrics() -> dict:
        m = {"llm_calls": 0, "llm_cached_calls": 0, "llm_time_seconds": 0.0}
        if stage_metrics_file.exists():
            try:
                with open(stage_metrics_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    m["llm_calls"] = int(data.get("llm_calls", 0))
                    m["llm_cached_calls"] = int(data.get("llm_cached_calls", 0))
                    m["llm_time_seconds"] = float(data.get("llm_time_seconds", 0.0))
            except Exception:
                pass
        stage_metrics_file.unlink(missing_ok=True)
        return m

    update_status(status_file, stage_num, "running", page_ranges=page_ranges)
    start_time = time.time()
    try:
        subprocess.run(cmd, env=env, check=True)
        duration = round(time.time() - start_time, 2)
        m = read_and_clean_metrics()
        update_status(
            status_file,
            stage_num,
            "success",
            duration_seconds=duration,
            llm_calls=m["llm_calls"],
            llm_cached_calls=m["llm_cached_calls"],
            llm_time_seconds=m["llm_time_seconds"],
            page_ranges=page_ranges,
        )
        print(f"[*] Stage {stage_num} finished in {duration:.2f}s (Codex calls: {m['llm_calls']}, Cache: {m['llm_cached_calls']}).")
        return True
    except subprocess.CalledProcessError as e:
        duration = round(time.time() - start_time, 2)
        m = read_and_clean_metrics()
        print(f"[!] Stage {stage_num} failed with return code {e.returncode} ({duration:.2f}s, Codex calls: {m['llm_calls']}, Cache: {m['llm_cached_calls']})", file=sys.stderr)
        update_status(
            status_file,
            stage_num,
            "failed",
            f"Exit code {e.returncode}",
            duration_seconds=duration,
            llm_calls=m["llm_calls"],
            llm_cached_calls=m["llm_cached_calls"],
            llm_time_seconds=m["llm_time_seconds"],
            page_ranges=page_ranges,
        )
        return False
    except BaseException as e:
        duration = round(time.time() - start_time, 2)
        m = read_and_clean_metrics()
        print(f"[!] Stage {stage_num} encountered exception: {e} ({duration:.2f}s, Codex calls: {m['llm_calls']}, Cache: {m['llm_cached_calls']})", file=sys.stderr)
        update_status(
            status_file,
            stage_num,
            "failed",
            str(e) or type(e).__name__,
            duration_seconds=duration,
            llm_calls=m["llm_calls"],
            llm_cached_calls=m["llm_cached_calls"],
            llm_time_seconds=m["llm_time_seconds"],
            page_ranges=page_ranges,
        )
        if not isinstance(e, Exception):
            raise
        return False


def print_pipeline_status(workspace_dir: Path):
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

    status_file = workspace_dir / "stage_status.json"
    if status_file.exists():
        try:
            with open(status_file, "r", encoding="utf-8") as f:
                status_data = json.load(f)
            if status_data:
                print("\n---------------- Stage Statistics ----------------")
                total_duration, total_calls, total_cached = 0.0, 0, 0
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
                        cached = st.get("llm_cached_calls", 0)
                        total_calls += calls
                        total_cached += cached
                        print(f"Stage {num} ({s['dir']:<28}): {status:<8} | Time: {dur_str:>8} | Codex calls: {calls:>4} | Cache: {cached:>4}")
                print(f"Total Measured Time: {total_duration:.2f}s | Total LLM API Calls: {total_calls} | Total Cache Hits: {total_cached}")
                print("--------------------------------------------------")
        except Exception:
            pass

    print("==================================================\n")


def clean_downstream_stages(workspace_dir: Path, start_idx: int, status_file: Path):
    def remove_directory(path, anchor):
        resolved, root = path.resolve(), anchor.resolve()
        if resolved == root or not resolved.is_relative_to(root):
            raise ValueError("Cleanup directory escapes its conversion workspace/output")
        shutil.rmtree(path)

    stages_to_clean = invalidated_stages(start_idx)
    invalidate_status(status_file, stages_to_clean)
    print(f"[*] Invalidation: Wiping Stage {STAGE_REGISTRY[start_idx]['id']} and all later stages...")
    for s in STAGE_REGISTRY:
        if s["id"] not in stages_to_clean:
            continue
        s_id = s["id"]
        (workspace_dir / s["dir"] / ".metrics.json").unlink(missing_ok=True)
        (workspace_dir / f".stage_{s_id}_metrics.json").unlink(missing_ok=True)
        for target in s["targets"]:
            p = workspace_dir / target
            if p.is_dir():
                if s_id == "01":
                    # Partial OCR results are reusable only after Stage 01
                    # checks source/image/procedure/configuration identities.
                    for child in p.iterdir():
                        if child.name != "recovery":
                            if child.is_dir():
                                remove_directory(child, workspace_dir)
                            else:
                                child.unlink()
                else:
                    remove_directory(p, workspace_dir)
            elif p.is_file():
                p.unlink(missing_ok=True)


def invalidate_status(status_file, stages):
    if status_file.exists():
        data = read_json(status_file)
        write_json(status_file, {key: value for key, value in data.items() if key not in stages})


def main():
    parser = argparse.ArgumentParser(description="PDF preparation pipeline (page objects and table transcription)")
    parser.add_argument("--pdf", type=Path, help="Source PDF or directory containing one source PDF")
    parser.add_argument("--workspace", type=Path)
    parser.add_argument("--config", type=Path, required=True, help="Initialize config.yaml only when the attempt has none")
    parser.add_argument("--cache-dir", type=Path, help="Codex cache directory for worker subprocesses")
    parser.add_argument("--from-stage", help="Start stage (default: 01); clears this stage and all later results")
    parser.add_argument("--to-stage", help="Last stage to execute; does not limit cleanup")
    parser.add_argument("--page-ranges", help="Physical 1-based pages for this run only; omitted selects all available pages")
    parser.add_argument("--verbose", action="store_true")
    parser.add_argument("--status", action="store_true")
    args = parser.parse_args()
    skill_dir = Path(__file__).resolve().parent
    pdf = args.pdf.resolve() if args.pdf else None
    if pdf and pdf.is_dir():
        pdf, = (path for path in pdf.glob("*.pdf") if not path.stem.lower().endswith("-ocr")
                and not path.name.lower().endswith(".candidate.pdf"))
    workspace = args.workspace.resolve() if args.workspace else ((pdf.parent / "workspace") if pdf else Path.cwd() / "workspace")
    if args.status:
        print_pipeline_status(workspace)
        return
    if pdf is None:
        parser.error("--pdf is required for conversion and restart")
    start = resolve_stage_idx(args.from_stage) if args.from_stage else 0
    end = resolve_stage_idx(args.to_stage) if args.to_stage else len(STAGE_REGISTRY) - 1
    if start is None or end is None or start > end:
        raise ValueError("Invalid stage interval")

    config_path = workspace / "config.yaml"
    config_source = config_path if config_path.exists() else args.config.resolve()
    config = parse_config(config_source)
    if start <= resolve_stage_idx("05") <= end:
        config.setdefault("llm", {}).setdefault("stages", {}).setdefault(
            "05_transform_page_tables", {"model": "gpt-6.1-sol", "reasoning_effort": "medium"})
    # Retire former settings from existing attempt configurations.
    for key in ("input", "render", "table_conversion", "ocr"):
        config.pop(key, None)
    for key in ("timeout_seconds", "concurrency"):
        config.get("llm", {}).pop(key, None)
    pages = selected_pages(args.page_ranges)

    status_file = workspace / "stage_status.json"
    statuses = read_json(status_file) if status_file.exists() else {}
    if any(stage_id in statuses for stage_id in ("00", "02.80", "02.81")):
        raise ValueError("This attempt uses the previous stage numbering; start a new named workspace")
    selected = STAGE_REGISTRY[start:end + 1]
    selected_ids = {stage["id"] for stage in selected}
    invalid = invalidated_stages(start)
    retained_ids = {key for key, entry in statuses.items() if key not in invalid and entry.get("status") == "success"}
    for stage in selected:
        for input_id in stage["inputs"]:
            if input_id not in selected_ids and input_id not in retained_ids:
                raise ValueError(f"Missing successful predecessor Stage {input_id} for Stage {stage['id']}")

    workspace.mkdir(parents=True, exist_ok=True)
    candidate = config_path.with_suffix(".yaml.tmp")
    try:
        candidate.write_text(yaml.safe_dump(config, sort_keys=False, allow_unicode=True), encoding="utf-8")
        candidate.replace(config_path)
    finally:
        candidate.unlink(missing_ok=True)
    if args.cache_dir:
        os.environ["CONVERSION_CACHE_DIR"] = str(args.cache_dir.resolve())
    # Persist invalidation before any cleanup can fail or be interrupted.
    invalidate_status(status_file, invalid)
    clean_downstream_stages(workspace, start, status_file)
    page_ranges = ",".join(map(str, pages)) if pages is not None else None
    print(f"[*] PDF source: {pdf.name}; selected pages: {pages}; stages: {[stage['id'] for stage in selected]}")
    for stage in selected:
        if not run_stage(stage, skill_dir, workspace, pdf, config_path, args.verbose, page_ranges):
            raise RuntimeError(f"Pipeline halted at Stage {stage['id']}")
    print("[+] Selected PDF pipeline stages completed.")


if __name__ == "__main__":
    main()

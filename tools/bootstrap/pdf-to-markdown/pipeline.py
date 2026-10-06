#!/usr/bin/env python3
"""
pipeline.py: Master CLI Orchestrator for PDF-to-Markdown Stages 00-14.
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
import pymupdf

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from conversion.config import load_config as parse_config, validate_config, PDF_STAGES, selection
from conversion.transport import CodexTransport
from conversion.lineage import read_state, write_state, file_hash, stage_identity, validate_prefix, restore_shared, complete_stage, first_incomplete_stage, artifact_predecessor
from conversion.pdf_artifacts import validate_text_layer
from conversion.pdf_selection import parse_page_ranges, selected_pages
from conversion.publication import book_directory, check_destination, publish_output

MANUAL_TASKS = {"06": "continuations", "07": "tables", "08": "graphics", "09": "prose"}

STAGE_REGISTRY: List[Dict[str, Any]] = [
    {
        "id": "00",
        "dir": "00_text_layer",
        "script": "prepare_text_layer.py",
        "desc": "Prepare and validate a separate PDF with native/OCR text provenance",
        "targets": ["00_text_layer"],
        "inspect": ("text_layer_manifest.json", "text-layer manifest"),
        "artifact_contract": "pdf_text_layer",
    },
    {
        "id": "01",
        "dir": "01_preprocess",
        "script": "preprocess.py",
        "desc": "Extract positioned text and matching PNGs from the prepared PDF",
        "targets": ["01_preprocess", "pages_manifest.json"],
        "inspect": ("*.png", "rendered PNGs"),
        "artifact_contract": "pdf_preprocess",
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
        "id": "02d", "dir": "02d_page_conversion", "script": "convert_page.py",
        "desc": "Independently group page objects and convert their text to Markdown",
        "targets": ["02d_page_conversion"],
        "inspect": ("page_*_segments.json", "page conversion JSON files"),
        "artifact_contract": "pdf_page_conversion",
    },
    {
        "id": "02k",
        "dir": "02k_segmentation_review",
        "script": "render_review.py",
        "desc": "Render segment frames and classification labels for visual review",
        "targets": ["02k_segmentation_review"],
        "inspect": ("page_*_review.png", "segmentation review PNGs"),
    },
    {
        "id": "02m", "dir": "02m_page_conversion_review", "script": "render_review.py",
        "desc": "Review independent page objects without textual geometry",
        "targets": ["02m_page_conversion_review"],
        "inspect": ("page_*_review.png", "page conversion review PNGs"),
        "artifact_contract": "pdf_page_conversion_review",
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
        "targets": ["08_transform_graphics"],
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
        "desc": "Emit per-section Markdown files (suppressing toc_heading)",
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

# Execution order and artifact dependencies are deliberately separate.
for index, stage in enumerate(STAGE_REGISTRY):
    stage["inputs"] = {
        "02d": ["01"], "02k": ["02"], "02m": ["01", "02d"], "03": ["02k"],
    }.get(stage["id"], [STAGE_REGISTRY[index-1]["id"]] if index else [])


def invalidated_stages(start_idx):
    invalid = {STAGE_REGISTRY[start_idx]["id"]}
    for stage in STAGE_REGISTRY:
        if invalid.intersection(stage["inputs"]):
            invalid.add(stage["id"])
    return invalid


STAGE_DEFINITIONS = [(s["id"], s["dir"], s["script"], s["desc"]) for s in STAGE_REGISTRY]
STAGE_OUTPUT_TARGETS = {s["id"]: s["targets"] for s in STAGE_REGISTRY}




def update_status(
    status_file: Path,
    stage_num: str,
    status: str,
    details: str = "",
    duration_seconds: Optional[float] = None,
    llm_calls: Optional[int] = None,
    llm_cached_calls: Optional[int] = None,
    llm_time_seconds: Optional[float] = None,
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
    if llm_cached_calls is not None or "llm_cached_calls" not in entry:
        entry["llm_cached_calls"] = llm_cached_calls if llm_cached_calls is not None else 0
    if llm_time_seconds is not None or "llm_time_seconds" not in entry:
        entry["llm_time_seconds"] = llm_time_seconds if llm_time_seconds is not None else 0.0

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
        if val.isdigit() and s["id"].isdigit() and int(val) == int(s["id"]):
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
    if stage_num == "00":
        if pdf_path is None:
            raise ValueError("--pdf is required to regenerate Stage 00")
        cmd.extend(["--pdf", str(pdf_path)])
        if page_ranges:
            cmd.extend(["--page-ranges", str(page_ranges)])
    elif stage_num == "01":
        _, prepared_pdf = validate_text_layer(workspace_dir, require_completion=True)
        cmd.extend(["--pdf", str(prepared_pdf)])
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
        return m

    update_status(status_file, stage_num, "running")
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
        )
        return False
    except Exception as e:
        duration = round(time.time() - start_time, 2)
        m = read_and_clean_metrics()
        print(f"[!] Stage {stage_num} encountered exception: {e} ({duration:.2f}s, Codex calls: {m['llm_calls']}, Cache: {m['llm_cached_calls']})", file=sys.stderr)
        update_status(
            status_file,
            stage_num,
            "failed",
            str(e),
            duration_seconds=duration,
            llm_calls=m["llm_calls"],
            llm_cached_calls=m["llm_cached_calls"],
            llm_time_seconds=m["llm_time_seconds"],
        )
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
        print(f"[*] Final Workspace Output        : {md_count} files in {output_dir.name}/")

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



def clean_downstream_stages(workspace_dir: Path, output_dir: Optional[Path], start_idx: int, status_file: Path, *, keep_tasks_for=None):
    def remove_directory(path, anchor):
        resolved, root = path.resolve(), anchor.resolve()
        if resolved == root or not resolved.is_relative_to(root):
            raise ValueError("Cleanup directory escapes its conversion workspace/output")
        shutil.rmtree(path)

    stages_to_clean = invalidated_stages(start_idx)
    print(f"[*] Invalidation: Wiping Stage {STAGE_REGISTRY[start_idx]['id']} and its descendants...")
    for s in STAGE_REGISTRY:
        if s["id"] not in stages_to_clean:
            continue
        s_id = s["id"]
        if s_id in MANUAL_TASKS and s_id != keep_tasks_for:
            task_dir = workspace_dir / "tasks" / MANUAL_TASKS[s_id]
            if task_dir.exists():
                remove_directory(task_dir, workspace_dir)
            (workspace_dir / "tasks/.lineage" / f"{s_id}.json").unlink(missing_ok=True)
        (workspace_dir / s["dir"] / ".metrics.json").unlink(missing_ok=True)
        (workspace_dir / f".stage_{s_id}_metrics.json").unlink(missing_ok=True)
        for target in s["targets"]:
            if target == "__OUTPUT_DIR__":
                if output_dir and output_dir.exists():
                    for f in output_dir.glob("*.md"):
                        f.unlink(missing_ok=True)
                    if (output_dir / "assets").exists():
                        remove_directory(output_dir / "assets", output_dir)
            else:
                p = workspace_dir / target
                if p.is_dir():
                    if s_id == "00":
                        # Partial OCR results are reusable only after Stage 00
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

    if status_file.exists():
        try:
            with open(status_file, "r", encoding="utf-8") as f:
                data = json.load(f)
            updated = {k: v for k, v in data.items() if k not in stages_to_clean}
            with open(status_file, "w", encoding="utf-8") as f:
                json.dump(updated, f, indent=2)
        except Exception:
            pass


def complete_pipeline_stage(state, stage, identity, workspace, output):
    if stage["id"] in ("05", "06"):
        position = next(index for index, item in enumerate(STAGE_REGISTRY) if item["id"] == stage["id"])
        previous_dir = STAGE_REGISTRY[position - 1]["dir"]
        assets = workspace / previous_dir / "assets"
        if assets.is_dir():
            shutil.copytree(assets, workspace / stage["dir"] / "assets", dirs_exist_ok=True)
    return complete_stage(state, stage, identity, workspace, output)


def main():
    parser = argparse.ArgumentParser(description="PDF-to-Markdown Pipeline Orchestrator (Stages 00-14)")
    parser.add_argument("--pdf", type=Path, help="Source PDF or directory containing exactly one PDF")
    parser.add_argument("--workspace", type=Path)
    parser.add_argument("--publish", action="store_true", help="Copy finished Markdown/assets into the empty sibling book directory without -tmp")
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--cache-dir", type=Path, help="Codex cache directory for worker subprocesses")
    parser.add_argument("--from-stage")
    parser.add_argument("--to-stage")
    parser.add_argument("--page-ranges", help="Physical 1-based source PDF pages")
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--verbose", action="store_true")
    parser.add_argument("--status", action="store_true")
    parser.add_argument("--prepare-stage")
    parser.add_argument("--apply-stage")
    parser.add_argument("--run-deterministic", action="store_true", help="Run the next ready deterministic stage (00, 01, 02k, 02m, 03, 05, 11, 14)")
    args = parser.parse_args()
    skill_dir = Path(__file__).resolve().parent
    config_source = args.config.resolve()
    config = parse_config(config_source, known_stages=PDF_STAGES, required_stages=())
    pdf = args.pdf.resolve() if args.pdf else None
    if pdf and pdf.is_dir():
        candidates = list(pdf.glob("*.pdf"))
        if len(candidates) != 1:
            raise ValueError("Source directory must contain exactly one PDF")
        pdf = candidates[0]
    if pdf and not pdf.is_file():
        raise FileNotFoundError(pdf)
    workspace = args.workspace.resolve() if args.workspace else ((pdf.parent / "workspace") if pdf else Path.cwd() / "workspace")
    output = workspace / "14_link_toc"
    destination = book_directory(workspace) if args.publish else None
    if destination is not None:
        check_destination(destination)
        if args.status or args.prepare_stage or args.apply_stage:
            raise ValueError("Publishing requires completed conversion, not status or manual handoff")
    if args.status:
        print_pipeline_status(workspace, output)
        return
    if sum(bool(value) for value in (args.prepare_stage, args.apply_stage, args.resume, args.run_deterministic)) > 1:
        raise ValueError("Choose one execution mode")
    if args.resume and (args.from_stage or args.to_stage):
        raise ValueError("Resume cannot be combined with a stage interval")
    state = read_state(workspace)
    completed = first_incomplete_stage(state, STAGE_REGISTRY, workspace, output)
    manual = args.prepare_stage or args.apply_stage
    if manual:
        start = resolve_stage_idx(manual)
        end = start
    elif args.run_deterministic:
        start = completed
        end = start
        if start >= len(STAGE_REGISTRY) or STAGE_REGISTRY[start]["id"] not in ("00", "01", "02k", "02m", "03", "05", "11", "14"):
            raise ValueError("Next stage requires inference; use an explicit stage interval")
    elif args.resume:
        start, end = completed, len(STAGE_REGISTRY)-1
        if not state.get("source") or state.get("stages") and "00" not in state["stages"]:
            raise ValueError("Legacy/unverified workspace cannot resume; use --pdf <source.pdf> --from-stage 00")
    else:
        start = resolve_stage_idx(args.from_stage) if args.from_stage else 0
        end = resolve_stage_idx(args.to_stage) if args.to_stage else len(STAGE_REGISTRY)-1
    if start is None or end is None or start > end and start != len(STAGE_REGISTRY):
        raise ValueError("Invalid stage interval")
    if args.publish and end != len(STAGE_REGISTRY)-1:
        raise ValueError("Publishing requires completion through Stage 14")
    source = {"name": pdf.name, "sha256": file_hash(pdf)} if pdf else state.get("source")
    if not source:
        raise ValueError("--pdf is required for a new conversion")
    source = dict(source)
    if args.page_ranges is not None:
        source["pages"] = parse_page_ranges(args.page_ranges)
        if not source["pages"] or min(source["pages"]) < 1:
            raise ValueError("Invalid physical PDF page selection")
    elif start == 0 and not args.resume:
        source["pages"] = None
    else:
        source["pages"] = state.get("source", {}).get("pages")
    if pdf:
        with pymupdf.open(pdf) as document:
            selected_pages(",".join(map(str, source["pages"])) if source["pages"] is not None else None, len(document))
    if pdf and pdf.is_relative_to(workspace):
        raise ValueError("Keep the original source PDF outside the conversion workspace")
    invalid = invalidated_stages(start) if start < len(STAGE_REGISTRY) else set()
    retained_registry = [item for item in STAGE_REGISTRY if item["id"] not in invalid and item["id"] in state.get("stages", {})]
    validate_prefix(state, retained_registry, len(retained_registry), config, source, skill_dir, workspace, output)
    if start < len(STAGE_REGISTRY):
        predecessor = artifact_predecessor(state, STAGE_REGISTRY, STAGE_REGISTRY[start])
    if start == len(STAGE_REGISTRY):
        restore_shared(state, retained_registry, len(retained_registry), workspace)
        print("[+] All PDF stages have validated completion records.")
        if destination is not None:
            publish_output(output, destination)
            print(f"[+] Published reference book: {destination}")
        return
    if start == 0 and pdf is None:
        raise ValueError("--pdf is required to regenerate Stage 00")
    selected = STAGE_REGISTRY[start:end+1]
    selected_ids = {stage["id"] for stage in selected}
    for stage in selected:
        for input_id in stage["inputs"]:
            if input_id not in selected_ids and input_id not in {item["id"] for item in retained_registry}:
                raise ValueError(f"Missing validated artifact input Stage {input_id} for Stage {stage['id']}")
    required = {stage["dir"] for stage in selected
                if stage["id"] not in {item["id"] for item in retained_registry} and stage["dir"] in PDF_STAGES}
    validate_config(config, PDF_STAGES, required)
    if required and not manual:
        transport = CodexTransport()
        try:
            for stage in required:
                transport.validate(selection(config, stage), images=stage[:2] in ("00", "02", "04", "07", "08", "09"))
        finally:
            transport.close()
    stage = STAGE_REGISTRY[start]
    identity = stage_identity(stage, config, source, predecessor, skill_dir)
    marker = workspace / "tasks/.lineage" / f"{stage['id']}.json"
    if manual and stage["id"] not in MANUAL_TASKS:
        raise ValueError("Manual handoff is available only for Stages 06–09")
    if args.apply_stage:
        if not marker.is_file() or json.loads(marker.read_text(encoding="utf-8")) != identity:
            raise ValueError("Prepared tasks do not match source, predecessor, model/effort or procedure; prepare again")
    # All configuration and predecessor validation precedes writes and cleanup.
    workspace.mkdir(parents=True, exist_ok=True)
    config_path = workspace / "config.yaml"
    if config_source != config_path:
        shutil.copyfile(config_source, config_path)
    if args.cache_dir:
        os.environ["CONVERSION_CACHE_DIR"] = str(args.cache_dir.resolve())
    restore_shared(state, retained_registry, len(retained_registry), workspace)
    clean_downstream_stages(workspace, output, start, workspace / "stage_status.json",
                            keep_tasks_for=stage["id"] if args.apply_stage else None)
    state["source"] = source
    retained_ids = {item["id"] for item in retained_registry}
    state["stages"] = {key: value for key, value in state.get("stages", {}).items() if key in retained_ids}
    write_state(workspace, state)
    if manual:
        script = skill_dir / "stages" / stage["dir"] / stage["script"]
        mode = "--prepare" if args.prepare_stage else "--apply"
        subprocess.run([sys.executable, str(script), "--workspace", str(workspace), "--config", str(config_path), mode], check=True)
        if args.prepare_stage:
            marker.parent.mkdir(parents=True, exist_ok=True)
            marker.write_text(json.dumps(identity, indent=2), encoding="utf-8")
        else:
            complete_pipeline_stage(state, stage, identity, workspace, output)
            update_status(workspace / "stage_status.json", stage["id"], "success", "Validated manual handoff")
        return
    print(f"[*] PDF source: {source['name']}; selected pages: {source['pages']}; stages: {[stage['id'] for stage in selected]}")
    for stage in selected:
        if stage["id"] in retained_ids:
            print(f"[*] Retaining validated Stage {stage['id']}")
            continue
        inputs = set(stage["inputs"])
        ancestors = set(inputs)
        for item in reversed(STAGE_REGISTRY):
            if item["id"] in ancestors:
                ancestors.update(item["inputs"])
        input_registry = [item for item in STAGE_REGISTRY if item["id"] in ancestors]
        validate_prefix(state, input_registry, len(input_registry), config, source, skill_dir, workspace, output)
        predecessor = artifact_predecessor(state, STAGE_REGISTRY, stage)
        identity = stage_identity(stage, config, source, predecessor, skill_dir)
        page_ranges = ",".join(map(str, source["pages"])) if source["pages"] else None
        if not run_stage(stage, skill_dir, workspace, pdf, output, config_path, args.verbose, page_ranges):
            raise RuntimeError(f"Pipeline halted at Stage {stage['id']}")
        try:
            predecessor = complete_pipeline_stage(state, stage, identity, workspace, output)
        except Exception:
            update_status(workspace / "stage_status.json", stage["id"], "failed", "Artifact completion validation failed")
            raise
    print("[+] Selected PDF pipeline stages completed with validated lineage.")
    if destination is not None:
        publish_output(output, destination)
        print(f"[+] Published reference book: {destination}")


if __name__ == "__main__":
    main()

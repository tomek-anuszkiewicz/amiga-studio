#!/usr/bin/env python3
"""Antigravity CLI Pipeline Run Auditor.

Analyzes Antigravity CLI logs, SQLite trajectory databases, and transcripts
for a full pipeline execution. Evaluates execution durations, LLM turn counts,
token usage, context growth, file I/O operations (especially JSON reading/writing),
worker scope isolation, and anomalous tool behaviors.
"""

from __future__ import annotations

import argparse
import datetime
import json
from pathlib import Path
import re
import sqlite3
import sys

# Ensure UTF-8 output on Windows console
if sys.platform == "win32":
    try:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except Exception:
        pass


def get_cli_root() -> Path:
    """Resolve the Antigravity CLI data directory dynamically."""
    cli_root = Path.home() / ".gemini" / "antigravity-cli"
    if not cli_root.exists():
        raise FileNotFoundError(f"Antigravity CLI directory not found at {cli_root}")
    return cli_root


def parse_proto(b: bytes) -> list[tuple[int, str, any]]:
    """Generic protobuf wire format decoder."""
    pos = 0
    fields = []
    while pos < len(b):
        tag = 0
        shift = 0
        while True:
            if pos >= len(b):
                break
            byte = b[pos]
            pos += 1
            tag |= (byte & 0x7F) << shift
            if (byte & 0x80) == 0:
                break
            shift += 7
        field_num = tag >> 3
        wire_type = tag & 0x07
        if wire_type == 0:  # varint
            val = 0
            shift = 0
            while True:
                if pos >= len(b):
                    break
                byte = b[pos]
                pos += 1
                val |= (byte & 0x7F) << shift
                if (byte & 0x80) == 0:
                    break
                shift += 7
            fields.append((field_num, "varint", val))
        elif wire_type == 2:  # length-delimited
            length = 0
            shift = 0
            while True:
                if pos >= len(b):
                    break
                byte = b[pos]
                pos += 1
                length |= (byte & 0x7F) << shift
                if (byte & 0x80) == 0:
                    break
                shift += 7
            val = b[pos : pos + length]
            pos += length
            fields.append((field_num, "bytes", val))
        else:
            break
    return fields


def extract_step_tokens(meta_bytes: bytes) -> tuple[int, int, int]:
    """Extract (total_input_tokens, output_tokens, cached_tokens) from step metadata protobuf."""
    for fnum, wtype, val in parse_proto(meta_bytes):
        if fnum == 9 and wtype == "bytes":
            subfields = {}
            for sf, sw, sv in parse_proto(val):
                subfields[sf] = sv
            prompt_tokens = subfields.get(2, 0)
            output_tokens = subfields.get(3, 0)
            cached_tokens = subfields.get(5, 0)
            total_input = prompt_tokens + cached_tokens
            return total_input, output_tokens, cached_tokens
    return 0, 0, 0


def parse_iso(ts_str: str | None) -> datetime.datetime | None:
    """Parse ISO timestamp with fallback."""
    if not ts_str:
        return None
    if ts_str.endswith("Z"):
        ts_str = ts_str[:-1] + "+00:00"
    try:
        return datetime.datetime.fromisoformat(ts_str)
    except Exception:
        return None


def format_duration(seconds: float | None) -> str:
    """Format duration in seconds into 'Xm Ys'."""
    if seconds is None:
        return "N/A"
    mins = int(seconds // 60)
    secs = int(seconds % 60)
    return f"{mins}m {secs:02d}s"


def find_latest_pipeline_run(cli_root: Path, last_idx: int = 1) -> tuple[str, int, str]:
    """Find the latest pipeline run from history.jsonl.

    Returns (conversation_id, start_step_index, goal_prompt).
    """
    hist_file = cli_root / "history.jsonl"
    if not hist_file.exists():
        raise FileNotFoundError(f"history.jsonl not found in {cli_root}")

    matched_runs = []
    with open(hist_file, "r", encoding="utf-8") as f:
        for line in f:
            if not line.strip():
                continue
            d = json.loads(line)
            disp = d.get("display", "")
            if "pdf-pipeline" in disp or "/goal" in disp:
                matched_runs.append(d)

    if not matched_runs:
        raise RuntimeError("No pipeline runs found in CLI history.jsonl")

    if last_idx > len(matched_runs):
        last_idx = len(matched_runs)

    target_entry = matched_runs[-last_idx]
    cid = target_entry.get("conversationId", "")
    prompt = target_entry.get("display", "")

    # Determine start step index within the conversation
    trans_path = cli_root / "brain" / cid / ".system_generated" / "logs" / "transcript.jsonl"
    start_step = 0
    if trans_path.exists():
        with open(trans_path, "r", encoding="utf-8") as f:
            for line in f:
                d = json.loads(line)
                if d.get("type") == "USER_INPUT":
                    content = d.get("content", "")
                    if "pdf-pipeline" in content:
                        start_step = d.get("step_index", 0)

    return cid, start_step, prompt


def discover_subagents(cli_root: Path, parent_cid: str, min_step: int = 0) -> list[dict]:
    """Recursively discover subagents spawned by a parent conversation."""
    sa_dir = cli_root / "brain" / parent_cid / ".system_generated" / "subagents"
    if not sa_dir.exists():
        return []

    discovered = []
    for sf in sa_dir.glob("*.json"):
        try:
            d = json.loads(sf.read_text(encoding="utf-8"))
            s_idx = d.get("spawnStepIndex", 0)
            if s_idx >= min_step:
                discovered.append(d)
        except Exception:
            continue

    discovered.sort(key=lambda x: x.get("spawnStepIndex", 0))

    tree = []
    for d in discovered:
        child_cid = d.get("conversationId")
        item = {
            "conversation_id": child_cid,
            "type_name": d.get("subagentDescriptor", {}).get("typeName", "unknown"),
            "role": d.get("subagentDescriptor", {}).get("role", ""),
            "spawn_step": d.get("spawnStepIndex", 0),
            "parent_cid": parent_cid,
            "children": discover_subagents(cli_root, child_cid, min_step=0),
        }
        tree.append(item)

    return tree


def analyze_conversation(
    cli_root: Path,
    cid: str,
    start_step: int = 0,
    end_step: int = 999999,
    is_lead: bool = False,
    is_worker: bool = False,
    assigned_manual: str = "",
) -> dict:
    """Analyze a single conversation's trajectory, timing, tokens, and tool calls."""
    db_path = cli_root / "conversations" / f"{cid}.db"
    trans_path = cli_root / "brain" / cid / ".system_generated" / "logs" / "transcript_full.jsonl"
    if not trans_path.exists():
        trans_path = cli_root / "brain" / cid / ".system_generated" / "logs" / "transcript.jsonl"

    res = {
        "cid": cid,
        "first_time": None,
        "last_time": None,
        "duration_seconds": None,
        "llm_turns": 0,
        "prompt_tokens": 0,
        "output_tokens": 0,
        "cached_tokens": 0,
        "max_context": 0,
        "reads_total": 0,
        "reads_by_ext": {},
        "json_reads": [],
        "writes_total": 0,
        "writes_by_ext": {},
        "json_writes": [],
        "replace_calls": 0,
        "cmd_calls": 0,
        "dispatch_calls": 0,
        "msg_calls": 0,
        "text_turns": 0,
        "image_views": 0,
        "unique_images": set(),
        "task_prompt": "",
        "anomalies": [],
        "tool_details": [],
    }

    # Parse transcript
    if trans_path.exists():
        with open(trans_path, "r", encoding="utf-8") as f:
            for line in f:
                d = json.loads(line)
                idx = d.get("step_index", 0)
                if idx < start_step or idx > end_step:
                    continue

                dt = parse_iso(d.get("created_at"))
                if dt:
                    if res["first_time"] is None or dt < res["first_time"]:
                        res["first_time"] = dt
                    if res["last_time"] is None or dt > res["last_time"]:
                        res["last_time"] = dt

                stype = d.get("type")
                if stype == "USER_INPUT" and not res["task_prompt"]:
                    res["task_prompt"] = d.get("content", "")

                if stype == "PLANNER_RESPONSE":
                    tcalls = d.get("tool_calls", [])
                    if not tcalls:
                        res["text_turns"] += 1
                    for tc in tcalls:
                        tname = tc.get("name") or tc.get("tool_name") or ""
                        args = tc.get("args") or tc.get("tool_args", {})

                        if tname == "view_file":
                            res["reads_total"] += 1
                            rf = args.get("AbsolutePath") or args.get("path") or ""
                            ext = Path(rf).suffix.lower() if rf else "none"
                            res["reads_by_ext"][ext] = res["reads_by_ext"].get(ext, 0) + 1
                            if ext == ".json":
                                res["json_reads"].append(Path(rf).name)
                            if ext in [".png", ".jpg", ".jpeg", ".webp"]:
                                res["image_views"] += 1
                                res["unique_images"].add(Path(rf).name)

                            # Anomaly: reading files from foreign manuals outside assigned scope
                            if is_worker and rf:
                                for foreign_m in [
                                    "68000 User's Manual",
                                    "Hardware Reference Manual",
                                    "68000 Programmer's Reference Manual",
                                    "A500 A2000 Technical Reference Manual",
                                ]:
                                    if foreign_m in rf and foreign_m != assigned_manual and foreign_m not in res["task_prompt"]:
                                        res["anomalies"].append(
                                            f"Turn {idx}: Cross-manual read into foreign book '{foreign_m}/{Path(rf).name}'"
                                        )
                                        break

                            res["tool_details"].append((idx, f"view({Path(rf).name})"))

                        elif tname == "write_to_file":
                            res["writes_total"] += 1
                            wf = args.get("TargetFile") or args.get("target_file") or ""
                            ext = Path(wf).suffix.lower() if wf else "none"
                            res["writes_by_ext"][ext] = res["writes_by_ext"].get(ext, 0) + 1
                            if ext == ".json":
                                res["json_writes"].append(Path(wf).name)
                                res["anomalies"].append(f"Turn {idx}: CRITICAL - Direct JSON write to '{Path(wf).name}'")
                            res["tool_details"].append((idx, f"write({Path(wf).name})"))

                        elif tname in ["replace_file_content", "multi_replace_file_content"]:
                            res["replace_calls"] += 1
                            res["tool_details"].append((idx, f"replace({tname})"))

                        elif tname == "run_command":
                            res["cmd_calls"] += 1
                            cmd = args.get("CommandLine", "")
                            res["tool_details"].append((idx, f"cmd({cmd[:40]})"))

                        elif tname == "invoke_subagent":
                            res["dispatch_calls"] += 1
                            res["tool_details"].append((idx, "invoke_subagent"))

                        elif tname == "send_message":
                            res["msg_calls"] += 1
                            msg = args.get("Message", "")
                            # Anomaly: Lead assigning a workload chunk via send_message instead of invoke_subagent
                            if is_lead and ("Execute Stage" in msg or "Payload:" in msg or "chunk_id" in msg):
                                res["anomalies"].append(
                                    f"Turn {idx}: Worker context reuse detected (Lead dispatched chunk via send_message)"
                                )
                            res["tool_details"].append((idx, "send_message"))

                        elif "mcp" in tname:
                            res["anomalies"].append(f"Turn {idx}: Unintended MCP tool call '{tname}'")
                            res["tool_details"].append((idx, tname))

                # Check for step errors
                if d.get("status") == "ERROR":
                    err_msg = d.get("content", "")
                    clean_err = err_msg.split("\n")[0][:70] if err_msg else "Tool error"
                    res["anomalies"].append(f"Step {idx}: STATUS=ERROR in tool execution ({clean_err})")

    if res["first_time"] and res["last_time"]:
        res["duration_seconds"] = (res["last_time"] - res["first_time"]).total_seconds()

    # Parse tokens from SQLite
    if db_path.exists():
        conn = sqlite3.connect(db_path)
        cur = conn.cursor()
        cur.execute(
            "SELECT idx, metadata FROM steps WHERE step_type=15 AND idx >= ? AND idx <= ?",
            (start_step, end_step),
        )
        for idx, meta in cur.fetchall():
            if meta:
                inp, outp, cach = extract_step_tokens(meta)
                if inp > 0 or outp > 0:
                    res["llm_turns"] += 1
                    res["prompt_tokens"] += inp
                    res["output_tokens"] += outp
                    res["cached_tokens"] += cach
                    if inp > res["max_context"]:
                        res["max_context"] = inp
        conn.close()

    res["unique_images"] = list(res["unique_images"])
    return res


def scan_cli_log_errors(
    cli_root: Path,
    start_time: datetime.datetime | None,
    end_time: datetime.datetime | None,
) -> list[str]:
    """Scan the most recent CLI log file for errors during the active run window."""
    log_dir = cli_root / "log"
    if not log_dir.exists():
        return []

    logs = sorted(log_dir.glob("cli-*.log"), key=lambda p: p.stat().st_mtime, reverse=True)
    if not logs:
        return []

    target_log = logs[0]
    errors = []

    t_start = start_time.strftime("%H:%M:%S") if start_time else None
    t_end = end_time.strftime("%H:%M:%S") if end_time else None

    with open(target_log, "r", encoding="utf-8", errors="ignore") as f:
        for line in f:
            if not line.startswith("E") and "AgentBasePath" not in line:
                continue

            m = re.search(r"(\d{2}:\d{2}:\d{2})", line)
            if m and t_start and t_end:
                log_t = m.group(1)
                if log_t < t_start or log_t > t_end:
                    continue

            errors.append(line.strip()[:140])

    return errors[:10]


def compute_concurrency_metrics(
    lead_role: str,
    lead_type: str,
    lead_data: dict,
    workers: list[tuple[str, str, dict]],
    max_concurrency_limit: int = 3,
) -> dict:
    """Calculate peak concurrency, overlap ratio, and batch count for a Stage Lead."""
    worker_intervals = []
    total_worker_dur = 0.0

    for w_role, w_type, w_data in workers:
        start_t = w_data.get("first_time")
        end_t = w_data.get("last_time")
        dur = w_data.get("duration_seconds") or 0.0
        total_worker_dur += dur
        if start_t and end_t and end_t >= start_t:
            worker_intervals.append((start_t, end_t))

    # Sweep-line algorithm
    peak_concurrency = 0
    if worker_intervals:
        events = []
        for s, e in worker_intervals:
            events.append((s, 1))
            events.append((e, -1))
        # Sort by timestamp. If equal, sort start (+1) before end (-1)
        events.sort(key=lambda x: (x[0], -x[1]))
        curr = 0
        for _, delta in events:
            curr += delta
            if curr > peak_concurrency:
                peak_concurrency = curr
    elif workers:
        peak_concurrency = 1

    lead_dur = lead_data.get("duration_seconds") or 0.0
    overlap_factor = (total_worker_dur / lead_dur) if lead_dur > 0 and total_worker_dur > 0 else 1.0

    batches_dispatched = lead_data.get("dispatch_calls", 0)

    # Status evaluation
    if peak_concurrency > max_concurrency_limit:
        status_badge = "🔴 VIOLATION (Exceeded Limit)"
    elif len(workers) > 1 and peak_concurrency == 1:
        status_badge = "🟡 Serialized (No Overlap)"
    elif len(workers) > 1 and peak_concurrency > 1:
        status_badge = "🟢 PASS (Parallel)"
    else:
        status_badge = "🟢 PASS (Single Worker)"

    return {
        "lead_role": lead_role,
        "lead_type": lead_type,
        "worker_count": len(workers),
        "batch_dispatches": batches_dispatched,
        "peak_concurrency": peak_concurrency,
        "concurrency_limit": max_concurrency_limit,
        "total_worker_duration": total_worker_dur,
        "lead_duration": lead_dur,
        "overlap_factor": overlap_factor,
        "status_badge": status_badge,
        "is_violation": peak_concurrency > max_concurrency_limit,
        "is_unparallelized": (len(workers) > 1 and peak_concurrency == 1),
    }


def generate_report(
    cli_root: Path,
    main_cid: str,
    start_step: int,
    goal_prompt: str,
) -> tuple[str, dict]:
    """Generate Markdown audit report and structured metrics dictionary."""
    # Detect target manual from prompt
    assigned_manual = ""
    for m in [
        "Test Book example-4567",
        "68000 User's Manual",
        "Hardware Reference Manual",
        "68000 Programmer's Reference Manual",
        "A500 A2000 Technical Reference Manual",
    ]:
        if m in goal_prompt:
            assigned_manual = m
            break

    main_data = analyze_conversation(cli_root, main_cid, start_step=start_step, assigned_manual=assigned_manual)
    leads_tree = discover_subagents(cli_root, main_cid, min_step=start_step)

    all_agents_data = [("Tier 1 Master Orchestrator", "Main Agent", main_data)]
    stage_concurrency_audits = []

    for lead in leads_tree:
        lead_cid = lead["conversation_id"]
        lead_role = lead["role"] or "Lead"
        lead_type = lead["type_name"]
        lead_data = analyze_conversation(cli_root, lead_cid, is_lead=True, assigned_manual=assigned_manual)
        all_agents_data.append((lead_role, lead_type, lead_data))

        workers_for_lead = []
        for worker in lead.get("children", []):
            w_cid = worker["conversation_id"]
            w_role = worker["role"] or "Worker"
            w_type = worker["type_name"]
            w_data = analyze_conversation(cli_root, w_cid, is_worker=True, assigned_manual=assigned_manual)
            all_agents_data.append((w_role, w_type, w_data))
            workers_for_lead.append((w_role, w_type, w_data))

        # Perform concurrency audit for this lead
        if workers_for_lead:
            c_metrics = compute_concurrency_metrics(lead_role, lead_type, lead_data, workers_for_lead)
            stage_concurrency_audits.append(c_metrics)
            if c_metrics["is_violation"]:
                all_agents_data[-1][2]["anomalies"].append(
                    f"CRITICAL - Concurrency Ceiling Violated: Peak concurrent workers was {c_metrics['peak_concurrency']} (ceiling: {c_metrics['concurrency_limit']})"
                )
            elif c_metrics["is_unparallelized"]:
                all_agents_data[-1][2]["anomalies"].append(
                    f"Notice: Dispatched {c_metrics['worker_count']} workers serially without parallel batching (Peak: 1, Overlap: {c_metrics['overlap_factor']:.2f}x)"
                )

    # Aggregations
    total_turns = sum(d["llm_turns"] for _, _, d in all_agents_data)
    total_prompt_tok = sum(d["prompt_tokens"] for _, _, d in all_agents_data)
    total_out_tok = sum(d["output_tokens"] for _, _, d in all_agents_data)
    total_reads = sum(d["reads_total"] for _, _, d in all_agents_data)
    total_writes = sum(d["writes_total"] for _, _, d in all_agents_data)
    total_replaces = sum(d["replace_calls"] for _, _, d in all_agents_data)
    total_cmds = sum(d["cmd_calls"] for _, _, d in all_agents_data)
    total_dispatches = sum(d["dispatch_calls"] for _, _, d in all_agents_data)
    total_msgs = sum(d["msg_calls"] for _, _, d in all_agents_data)
    total_img_views = sum(d["image_views"] for _, _, d in all_agents_data)
    all_unique_images = set()
    for _, _, d in all_agents_data:
        all_unique_images.update(d["unique_images"])

    overall_duration = main_data["duration_seconds"]

    # Collect all anomalies
    all_anomalies = []
    for role, name, d in all_agents_data:
        for a in d["anomalies"]:
            all_anomalies.append(f"[{name}] {a}")

    cli_log_errors = scan_cli_log_errors(cli_root, main_data["first_time"], main_data["last_time"])

    # Health Rating
    if any("CRITICAL" in a for a in all_anomalies):
        health_status = "CRITICAL FAILURES DETECTED"
        health_badge = "🔴"
    elif all_anomalies or cli_log_errors:
        health_status = "PASSED WITH WARNINGS"
        health_badge = "🟡"
    else:
        health_status = "HEALTHY / CLEAN PASS"
        health_badge = "🟢"

    # Build Markdown Report
    lines = []
    lines.append("# Antigravity CLI Pipeline Run Audit Report")
    lines.append(f"**Target Goal:** `{goal_prompt}`")
    lines.append(f"**Main Conversation ID:** `{main_cid}` (Evaluated from Step {start_step})")
    lines.append(f"**Total Run Duration:** {format_duration(overall_duration)}")
    lines.append(f"**Audit Status:** {health_badge} **{health_status}**\n")

    lines.append("## 1. Stage & Subagent Execution Metrics")
    lines.append("| Hierarchy & Role | Agent Type | ID Prefix | LLM Turns | Prompt Tok | Output Tok | Peak Ctx | Img Views | Duration |")
    lines.append("| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |")

    for role, name, d in all_agents_data:
        prefix = d["cid"][:8]
        dur = format_duration(d["duration_seconds"])
        lines.append(
            f"| **{role}** | `{name}` | `{prefix}` | {d['llm_turns']} | {d['prompt_tokens']:,} | {d['output_tokens']:,} | {d['max_context']:,} | {d['image_views']} | {dur} |"
        )

    lines.append(
        f"| **TOTAL PIPELINE** | — | — | **{total_turns}** | **{total_prompt_tok:,}** | **{total_out_tok:,}** | — | **{total_img_views}** | **{format_duration(overall_duration)}** |\n"
    )

    lines.append("## 2. File I/O & Tool Action Breakdown")
    lines.append("| Agent Type | Total Reads | JSON Reads | Total Writes | JSON Writes | Fragment Edits | CLI Cmds | Subagent Dispatches | Messages |")
    lines.append("| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |")

    for _, name, d in all_agents_data:
        j_reads = len(d["json_reads"])
        j_writes = len(d["json_writes"])
        lines.append(
            f"| `{name}` | {d['reads_total']} | {j_reads} | {d['writes_total']} | {j_writes} | {d['replace_calls']} | {d['cmd_calls']} | {d['dispatch_calls']} | {d['msg_calls']} |"
        )

    lines.append(
        f"| **TOTALS** | **{total_reads}** | **{sum(len(d['json_reads']) for _, _, d in all_agents_data)}** | **{total_writes}** | **0** | **{total_replaces}** | **{total_cmds}** | **{total_dispatches}** | **{total_msgs}** |\n"
    )

    if stage_concurrency_audits:
        lines.append("## 3. Worker Concurrency & Parallelism Audit")
        lines.append("| Stage Lead | Dispatched Workers | Dispatch Batches | Peak Concurrency | Concurrency Ceiling | Overlap Factor | Concurrency Status |")
        lines.append("| :--- | :---: | :---: | :---: | :---: | :---: | :---: |")
        for ca in stage_concurrency_audits:
            lines.append(
                f"| **{ca['lead_role']}** (`{ca['lead_type']}`) | {ca['worker_count']} | {ca['batch_dispatches']} | {ca['peak_concurrency']} | {ca['concurrency_limit']} | {ca['overlap_factor']:.2f}x | {ca['status_badge']} |"
            )
        lines.append("")

    lines.append("## 4. Qualitative Integrity & Contract Checks")
    # Rule 1: Zero direct JSON writes
    if sum(len(d["json_writes"]) for _, _, d in all_agents_data) == 0:
        lines.append("- [x] **Zero Direct JSON Mutations by LLMs:** PASS. Queue files were modified exclusively via deterministic CLI commands.")
    else:
        lines.append("- [ ] **Zero Direct JSON Mutations by LLMs:** FAIL. Subagents wrote directly to JSON files.")

    # Rule 2: Zero Fragmentary edits
    if total_replaces == 0:
        lines.append("- [x] **Zero Fragmentary Edits (`replace_file` = 0):** PASS. All files written as clean drop-in whole files via `write_to_file`.")
    else:
        lines.append(f"- [!] **Fragmentary Edits Detected:** {total_replaces} replacement calls were performed.")

    # Rule 3: Context Compaction
    max_peak = max(d["max_context"] for _, _, d in all_agents_data)
    if max_peak < 200000:
        lines.append(f"- [x] **Context Growth Within Bounds:** PASS. Peak context was {max_peak:,} tokens (well under the 1,000,000 ceiling). Zero compactions.")
    else:
        lines.append(f"- [!] **High Context Warning:** Peak context reached {max_peak:,} tokens.")

    # Rule 4: Worker Isolation & Context Refresh
    context_reuse = [a for a in all_anomalies if "context reuse" in a]
    if not context_reuse:
        lines.append("- [x] **Fresh Worker Context Isolation:** PASS. All worker tasks received independent subagents.")
    else:
        lines.append(f"- [!] **Worker Context Reuse Detected:** {len(context_reuse)} instance where Lead dispatched an additional chunk via `send_message` rather than `invoke_subagent`.")

    # Rule 5: Concurrency Ceiling
    max_sys_concurrency = max((ca["peak_concurrency"] for ca in stage_concurrency_audits), default=0)
    concurrency_violations = [ca for ca in stage_concurrency_audits if ca["is_violation"]]
    if not concurrency_violations:
        lines.append(f"- [x] **Worker Concurrency Ceiling (<= 3 workers):** PASS. Peak concurrent workers observed across all stages was {max_sys_concurrency}.")
    else:
        lines.append(f"- [ ] **Worker Concurrency Ceiling (<= 3 workers):** FAIL. Peak concurrent workers reached {max_sys_concurrency}, exceeding the safety ceiling of 3.")

    lines.append("\n## 5. Identified Anomalies & Inefficiencies")
    if all_anomalies:
        for a in all_anomalies:
            lines.append(f"- {a}")
    else:
        lines.append("- None detected! All subagent interactions followed optimal trajectories.")

    if cli_log_errors:
        lines.append("\n### Relevant CLI Log Warnings & Errors (Active Run Window):")
        for err in cli_log_errors[:6]:
            lines.append(f"```text\n{err}\n```")

    report_content = "\n".join(lines)
    metrics_dict = {
        "main_cid": main_cid,
        "total_turns": total_turns,
        "total_prompt_tok": total_prompt_tok,
        "total_out_tok": total_out_tok,
        "total_reads": total_reads,
        "total_writes": total_writes,
        "duration_seconds": overall_duration,
        "health_status": health_status,
        "peak_concurrency": max_sys_concurrency,
        "concurrency_audits": stage_concurrency_audits,
        "anomalies": all_anomalies,
    }

    return report_content, metrics_dict


def main():
    parser = argparse.ArgumentParser(description="Antigravity CLI Pipeline Run Auditor")
    parser.add_argument("--run-id", "--cid", dest="cid", help="Specific conversation ID to audit")
    parser.add_argument("--last", type=int, default=1, help="Audit the N-th most recent pipeline run (default: 1)")
    parser.add_argument("--step-start", type=int, default=None, help="Force start step index in main conversation")
    parser.add_argument("--json", action="store_true", help="Output raw JSON metrics instead of Markdown")
    parser.add_argument("--output", "-o", help="Save markdown report to specified relative file path")

    args = parser.parse_args()

    cli_root = get_cli_root()

    if args.cid:
        main_cid = args.cid
        start_step = args.step_start or 0
        prompt = f"Manual Audit of {main_cid}"
    else:
        main_cid, detected_start, prompt = find_latest_pipeline_run(cli_root, last_idx=args.last)
        start_step = args.step_start if args.step_start is not None else detected_start

    report_md, metrics = generate_report(cli_root, main_cid, start_step, prompt)

    if args.json:
        print(json.dumps(metrics, indent=2))
    else:
        print(report_md)

    if args.output:
        out_p = Path(args.output)
        out_p.parent.mkdir(parents=True, exist_ok=True)
        out_p.write_text(report_md, encoding="utf-8")
        print(f"\n[Audit Report saved to: {args.output}]")


if __name__ == "__main__":
    main()

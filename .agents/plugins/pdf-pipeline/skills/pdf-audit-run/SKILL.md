---
name: pdf-audit-run
description: >-
  Audits, inspects, and evaluates the performance, file I/O, token usage, subagent hierarchy, context growth, and execution quality of Antigravity CLI pipeline runs. Analyzes the latest run by default, providing per-stage, per-lead, and per-worker tables with anomaly detection.
---

# Antigravity CLI Pipeline Run Auditor (`pdf-audit-run`)

This skill provides automated post-execution forensics, performance auditing, and architectural quality evaluation for runs of the PDF-to-Markdown conversion pipeline executed in the Antigravity CLI (`agy`).

It reconstructs the multi-agent hierarchy (Tier 1 Master Orchestrator $\rightarrow$ Tier 2 Leads $\rightarrow$ Tier 3 Workers), parses internal SQLite trajectory databases, extracts exact Gemini API token usage, tracks file reads/writes (especially JSON mutations), and flags execution anomalies.

---

## Input & Output

### Input (Read-Only)
- **CLI Command History:** `~/.gemini/antigravity-cli/history.jsonl` (used to detect the latest or specified goal run).
- **Session Trajectory Databases:** `~/.gemini/antigravity-cli/conversations/<conversation_id>.db` (SQLite databases containing step types, metadata protobufs, prompt tokens, candidate tokens, and cached tokens).
- **Subagent Declarations:** `~/.gemini/antigravity-cli/brain/<conversation_id>/.system_generated/subagents/*.json`.
- **Transcripts:** `~/.gemini/antigravity-cli/brain/<conversation_id>/.system_generated/logs/transcript_full.jsonl`.
- **Language Server Logs:** `~/.gemini/antigravity-cli/log/cli-*.log`.

### Output
- **Execution Summary Table:** Detailed per-stage, per-lead, and per-worker breakdown showing LLM turns, prompt tokens, output tokens, peak context length, image views, and execution duration.
- **File I/O & Tool Breakdown Table:** Complete accounting of file reads (by extension, specifically tracking `.json`), whole-file writes (`write_to_file`), fragmentary edits (`replace_file_content`), CLI commands, subagent dispatches, and messages.
- **Worker Concurrency & Parallelism Audit Table:** Per-stage accounting of total workers dispatched, dispatch batches, sweep-line calculated peak concurrency, concurrency ceiling comparison, overlap speedup factor, and execution status.
- **Contract & Integrity Evaluation:** Automated pass/fail checks on zero direct JSON writes, zero fragmentary edits, context growth limits, worker context isolation, and adherence to the `MAX_CONCURRENCY <= 3` ceiling.
- **Anomaly Report:** Explicit enumeration of tool errors, excessive turns, worker context reuse via `send_message`, cross-manual reference leaks, concurrency ceiling violations, unintended serialization, and CLI log errors.
- **Optional Markdown Report File:** Saved to `--output <path>` (e.g. `<manual_dir>/build/<stem>_audit_report.md`).

---

## Key Invariants & Quality Rubric

The auditor systematically enforces the architectural standards of the PDF conversion pipeline:

1. **Zero Direct JSON Writes by LLMs:**
   - **Requirement:** Subagents (Leads and Workers) MUST NEVER modify `queue.json`, `assets_queue.json`, or execution plans directly via `write_to_file`. All queue mutations must occur strictly via deterministic Python CLI scripts (`stage6_prepare_queue.py`, `stage11_prepare_convert.py`).
2. **Zero Fragmentary Edits:**
   - **Requirement:** Agents must generate whole, drop-in Markdown files via `write_to_file`. Fragile and token-expensive fragment replacements (`replace_file_content` / `multi_replace_file_content`) are flagged as anti-patterns.
3. **Context Growth & Compaction Safeguards:**
   - **Requirement:** Worker contexts must remain bounded within their assigned chunks. Context lengths are checked to ensure they do not exceed reasonable operating ceilings or trigger emergency model context compaction.
4. **Worker Scope Isolation & Fresh Contexts:**
   - **Requirement:** Each chunk of pages or visual assets must be processed by an independent worker subagent with a fresh context. If a Lead reuses an existing worker via `send_message` rather than invoking a fresh `invoke_subagent`, a context reuse anomaly is reported.
5. **Worker Concurrency Ceiling & Parallel Efficiency:**
   - **Requirement:** Tier 2 Leads must enforce bounded parallelism ($\le 3$ workers simultaneously per batch wave). Peak concurrency is calculated via a timestamp sweep-line algorithm. Runs that exceed `MAX_CONCURRENCY` are flagged as critical contract violations, while multi-chunk runs that fall back to completely serialized execution without overlap are flagged for review.
6. **Cross-Manual Boundary Protection:**
   - **Requirement:** Workers must transcribe and inspect files strictly within their assigned manual directory. Reading files from other books in the repository is flagged as a calibration leak.

---

## Operational Usage

### 1. Audit the Latest Pipeline Run (Default)
To audit the most recent CLI run without specifying IDs:
```bash
python .agents/plugins/pdf-pipeline/skills/pdf-audit-run/scripts/audit_run.py
```

### 2. Audit a Specific Conversation ID
To audit an older run or a specific session:
```bash
python .agents/plugins/pdf-pipeline/skills/pdf-audit-run/scripts/audit_run.py --run-id <CONVERSATION_ID>
```

### 3. Audit the N-th Most Recent Run
To inspect the run before the latest:
```bash
python .agents/plugins/pdf-pipeline/skills/pdf-audit-run/scripts/audit_run.py --last 2
```

### 4. Save Audit Report to Markdown File
```bash
python .agents/plugins/pdf-pipeline/skills/pdf-audit-run/scripts/audit_run.py --output "Test Book example-4567/build/audit_report.md"
```

### 5. Machine-Readable JSON Output
```bash
python .agents/plugins/pdf-pipeline/skills/pdf-audit-run/scripts/audit_run.py --json
```

---

## Interpreting Audit Results

- 🟢 **HEALTHY / CLEAN PASS:** All invariants satisfied; workers operated in isolated chunks; zero direct JSON edits; zero fragmentary replacements; peak contexts bounded; zero errors.
- 🟡 **PASSED WITH WARNINGS:** Pipeline completed successfully, but non-fatal inefficiencies were identified (e.g. Lead reused a worker for a subsequent chunk, an agent attempted to view a non-existent file path, or serial tool chaining inflated turn counts).
- 🔴 **CRITICAL FAILURES DETECTED:** Severe contract violations (e.g. an LLM overwrote a queue JSON directly, or tool execution crashed unhandled).

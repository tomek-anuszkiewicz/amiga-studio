---
name: stage17-proofread-lead
description: >-
  Tier 2 Stage 17 Coordinator. Manages per-page semantic proofreading and retrocomputing syntax normalization, dispatching parallel batches of <= 10 pages (max 3 workers) to stage17-worker.
tools:
  - run_command
  - view_file
  - invoke_subagent
---

# Tier 2: Stage 17 Proofreading Lead

You coordinate Stage 17 (Multimodal Semantic Proofreading and Retrocomputing Syntax Normalization).

## Operational Workflow

1. **Load Pre-Computed Batches:**
   - Read the Stage 17 step from `build/<stem>_execution_plan.json`.
   - Identify the list of pre-grouped `batches` of pending pages (each batch contains strictly $\le 3$ chunks, each chunk $\le 10$ pages).
   - Note: Each `stage17-worker` reads its read-only preceding page sliding window context exclusively from `build/01_page_layout/page_{XXXX-1}-embed.md` (Stage 16 output, pre-existing on disk for all pages). Therefore, chunks have **zero inter-chunk runtime dependencies**.

2. **Batch-by-Batch Iteration Loop:**
   For **each** batch in the plan's `batches` list, execute the following cycle:

   a. **Dispatch Parallel Worker Subagents (One per Chunk):**
      - Invoke `stage17-worker` subagents concurrently in a single `invoke_subagent` call by specifying all chunks of this batch in the `Subagents` array (maximum 3 workers):
        ```json
        {
          "Subagents": [
            {
              "Role": "Stage 17 Worker Chunk <ID1>",
              "TypeName": "stage17-worker",
              "Prompt": "{\n  \"chunk_id\": <ID1>,\n  \"manual_dir\": \"<manual_dir>\",\n  \"pages\": [\"page_0001\", \"page_0002\", ...]\n}"
            },
            ...
          ]
        }
        ```
      - **CRITICAL:** Do NOT exceed the batch size defined in the plan (strictly $\le 3$ workers concurrently). Each chunk must have its own independent worker entry.

   b. **Disk Verification:**
      - Await return contracts from all workers in this batch.
      - Verify on disk via deterministic `run_command` that `build/01_page_layout/page_XXXX-proofread.md` exists and is non-empty for all target pages in this batch:
        `python .agents/plugins/pdf-pipeline/skills/pdf-stage17-proofread-page/scripts/stage17_proofread_page.py "<manual_dir>" --verify-pages <P1,P2,...>`
      - **Deterministic Enforcement:** `stage17_proofread_page.py` automatically checks all specified page files and exits with code 1 if any file is missing or empty. Do NOT run ad-hoc shell/PowerShell commands.

   c. **Advance:**
      - Proceed to the next batch in the list.

3. **Termination & Report to Tier 1:**
   - When all batches for the active scope are verified on disk, return ONLY a compact JSON summary to Tier 1:
     ```json
     {
       "status": "completed",
       "stage": 17,
       "pages_proofread": N
     }
     ```

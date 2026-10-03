---
name: stage12-table-html-lead
description: >-
  Tier 2 Stage 12 Coordinator. Dispatches parallel batches of <= 10 assets (max 3 workers) to stage12-worker to convert data tables into HTML tables and classify visual images, monitors queue updates, and reports completion to Tier 1.
tools:
  - run_command
  - view_file
  - invoke_subagent
---

# Tier 2: Stage 12 Table Lead

You coordinate Stage 12 (Table Conversion & Image Classification).

## Operational Workflow

1. **Load Pre-Computed Batches:**
   - Read the Stage 12 step from `build/<stem>_execution_plan.json`.
   - Identify the list of pre-grouped `batches` (each batch contains strictly $\le 3$ chunks, each chunk $\le 10$ assets).

2. **Batch-by-Batch Iteration Loop:**
   For **each** batch in the plan's `batches` list, execute the following cycle:

   a. **Dispatch Parallel Worker Subagents (One per Chunk):**
      - Invoke `stage12-worker` subagents concurrently in a single `invoke_subagent` call by specifying all chunks of this batch in the `Subagents` array (maximum 3 workers):
        ```json
        {
          "Subagents": [
            {
              "Role": "Stage 12 Worker Chunk <ID1>",
              "TypeName": "stage12-worker",
              "Prompt": "{\n  \"chunk_id\": <ID1>,\n  \"manual_dir\": \"<manual_dir>\",\n  \"assets\": [\"page_0001_crop_1\", \"page_0002_crop_1\", ...]\n}"
            },
            ...
          ]
        }
        ```
      - **CRITICAL:** Do NOT exceed the batch size defined in the plan (strictly $\le 3$ workers concurrently). Each chunk must have its own independent worker entry.

   b. **Queue Update & Disk Verification:**
      - Await return contracts from all workers in the batch.
      - Collect all converted table IDs and classified image IDs from the workers in this batch.
      - If converted tables exist, execute deterministic queue update via `run_command` (this automatically verifies on disk that each `assets/<asset_id>_html.md` exists and is non-empty):
        `python .agents/plugins/pdf-pipeline/skills/pdf-stage11-prepare-convert/scripts/stage11_prepare_convert.py "<manual_dir>" --update-stage 12 --completed <asset_ids...>`
      - If classified images exist, execute deterministic queue update via `run_command`:
        `python .agents/plugins/pdf-pipeline/skills/pdf-stage11-prepare-convert/scripts/stage11_prepare_convert.py "<manual_dir>" --update-stage 12 --classified-images <asset_ids...>`
      - **Deterministic Enforcement:** `stage11_prepare_convert.py` automatically validates that all converted HTML table files exist and are non-empty. Do NOT run ad-hoc shell/PowerShell commands.

   c. **Advance:**
      - Proceed to the next batch in the list.

3. **Termination & Report to Tier 1:**
   - When all batches for the active scope are verified on disk, return ONLY a compact JSON summary to Tier 1:
     ```json
     {
       "status": "completed",
       "stage": 12,
       "tables_converted": N,
       "images_classified": M
     }
     ```

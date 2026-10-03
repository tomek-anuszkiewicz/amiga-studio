---
name: stage19-chapter-merge-lead
description: >-
  Tier 2 Stage 19 Coordinator. Manages final semantic assembly and chapter fusion, dispatching parallel batches of chapter drafts (max 3 workers) to stage19-worker.
tools:
  - run_command
  - view_file
  - invoke_subagent
---

# Tier 2: Stage 19 Chapter Merge Lead

You coordinate Stage 19 (Multimodal Semantic Fusion of Chapter Drafts into Final Chapters).

## Operational Workflow

1. **Load Pre-Computed Batches:**
   - Read the Stage 19 step from `build/<stem>_execution_plan.json`.
   - Identify the list of pre-grouped `batches` (each batch contains strictly $\le 3$ chapter chunks, each chunk is 1 chapter draft).

2. **Batch-by-Batch Iteration Loop:**
   For **each** batch in the plan's `batches` list, execute the following cycle:

   a. **Dispatch Parallel Worker Subagents (One per Chapter):**
      - Invoke `stage19-worker` subagents concurrently in a single `invoke_subagent` call by specifying all chapters in this batch in the `Subagents` array (maximum 3 workers):
        ```json
        {
          "Subagents": [
            {
              "Role": "Stage 19 Chapter Fusion Worker Chunk <ID1>",
              "TypeName": "stage19-worker",
              "Prompt": "{\n  \"chunk_id\": <ID1>,\n  \"manual_dir\": \"<manual_dir>\",\n  \"chapter_file\": \"<manual_dir>/build/02_detect_cont_chapters/<filename1>\"\n}"
            },
            ...
          ]
        }
        ```
      - **CRITICAL:** Do NOT exceed the batch size defined in the plan (strictly $\le 3$ workers concurrently). Each chapter chunk must have its own independent worker entry.

   b. **Disk & Invariant Verification:**
      - Await return contracts from all workers in this batch.
      - Execute deterministic verification via `run_command` ensuring that `build/02_final_chapters/<filename>` exists, is non-empty, and contains **zero** remaining `<continuation-marker>` tags:
        `python .agents/plugins/pdf-pipeline/skills/pdf-stage19-merge-chapters/scripts/stage19_merge_chapters.py "<manual_dir>" --verify-chapters <filename1> <filename2>...`
      - **Deterministic Enforcement:** `stage19_merge_chapters.py` automatically checks file existence, size, and scans for unmerged continuation markers, failing with exit code 1 if any issues are found. Do NOT run ad-hoc shell/PowerShell commands.

   c. **Advance:**
      - Proceed to the next batch in the list.

3. **Termination & Report to Tier 1:**
   - When all batches for the active scope are verified on disk, return ONLY a compact JSON summary to Tier 1:
     ```json
     {
       "status": "completed",
       "stage": 19,
       "chapters_merged": N
     }
     ```

---
name: pdf-reset-stages
description: >-
  Resets and cleans up pipeline stages and artifacts for specified pages (Stages 7 through 19). Unwinds queue statuses back to pending, removes derived markdown, cropped assets, recrops, evaluation frames, table fragments, embedded documents, proofread pages, continuation tags, and chapter files to cleanly prepare pages for re-execution.
---

# Pipeline Stage Reset & Cleanup (Stages 7 to 19)

This skill provides deterministic, non-destructive stage rollback and artifact cleanup for specific pages across the technical PDF conversion pipeline. When an inference error, bounding box defect, conversion adjustment, or re-run is required, this tool cleanly unwinds queue states back to `pending`, purges obsolete assets and frames, and prepares the exact page range so that re-running the designated pipeline stages succeeds without friction or orphaned artifacts.

## Input & Output
- **Input:**
  - Target stage boundaries: `--from-stage` and `--to-stage` (restricted to Stages 7 through 19).
  - Target specification: `--pages <range>` (e.g. `70-80`, `73`), `--chapter <num>` (e.g. `1`, `1-3`), or `--all`.
  - Target manual directory (or all target manuals when omitted).
  - Active queue files: `build/<stem>_queue.json` and `build/<stem>_assets_queue.json`.
- **Output:**
  - Synchronized queue states with targeted page entries unwound to `"pending"` and reset metadata fields.
  - Selectively purged downstream intermediate artifacts for targeted pages (transcriptions, crops, recrops, eval frames, converted table/image fragments, embedded pages, proofread pages, draft chapters in `build/02_detect_cont_chapters/`, and final chapters in `build/02_final_chapters/`).
  - Automatically regenerated Ahead-Of-Time execution plan (`build/<stem>_execution_plan.json`) configured with the exact pending work units and chunks for the reset scope (unless `--no-plan` is specified).
  - Stage 5 layout ground truth (`page_XXXX.json`, `page_XXXX.png`) and source PDFs strictly preserved untouched.

---

## Supported Operational Scope (Stages 7 to 19)

> [!IMPORTANT]
> The reset tool supports **Stages 7 through 19**. Requests specifying stages outside this range (Stages 1–6) are rejected, as raw environment, downloads, OCR, chapters, and layout extractions should not be casually wiped.

| Stage | Name | Action Taken on Reset |
| :--- | :--- | :--- |
| **Stage 7** | Infer Markdown Prose | Deletes `page_XXXX.md`. Unwinds `build/<stem>_queue.json` status to `"pending"`. Cascades cleanup to all downstream Stage 8–19 assets. |
| **Stage 8** | Crop Visual Assets | Preserves `page_XXXX.md`. Deletes `page_XXXX-images.md`, all `assets/page_XXXX_*.png` crops, and removes page asset items from `build/<stem>_assets_queue.json`. Cascades to Stages 9–19. |
| **Stage 9** | Prepare Eval Frames & Recrop | Preserves `page_XXXX.md`, `page_XXXX-images.md`, and version 0 base crops. Deletes recrops (`_v*.png`), final clips (`_clip_final.png`), finalized docs (`page_XXXX-images-clip_final.md`), and frames (`eval_frames/`, `eval_final/`). Reverts queue items to version 0 baseline. |
| **Stage 10** | Evaluate & Reclip Assets | Preserves frames and base crops. Reverts evaluation verdicts (`eval_clip` -> `null`, `status` -> `"pending"`), removes `corrected_box`, and rolls back any finalized clip files. |
| **Stage 11** | Prep Conversion Queue | Resets all conversion tracking fields (`conversion_status` -> `"pending"`, fragment paths -> `null`, counters -> 0). |
| **Stage 12** | Convert Tables & Classify | Deletes `assets/<asset_id>_html.md`, `assets/<asset_id>_reduced.md`, `assets/<asset_id>.md`, `assets/<asset_id>.txt`, and `asset_frames/`. Clears `detected_type` -> `null` and resets `conversion_status` to `"pending"`. |
| **Stage 13** | Reduce HTML Tables | Deletes `assets/<asset_id>_reduced.md`. Clears `reduced_to_markdown` and restores `generated_markdown` pointer to original `_html.md`. |
| **Stage 14** | Convert Images & RAG | Deletes `assets/<asset_id>.md` and `assets/<asset_id>.txt` for `image` assets. Resets `conversion_status` to `"pending"`. |
| **Stage 15** | Render Asset Frames | Deletes review frames in `build/01_page_layout/asset_frames/`. |
| **Stage 16** | Embed Markdown | Deletes `build/01_page_layout/page_XXXX-embed.md`. Sets `is_embedded` -> `false` and clears `embedded_file` in queue. |
| **Stage 17** | Proofread Page | Deletes `build/01_page_layout/page_XXXX-proofread.md`. Cascades cleanup to Stage 18 draft chapters and Stage 19 final chapters. |
| **Stage 18** | Prepare Chapters | Deletes draft chapter documents in `build/02_detect_cont_chapters/` containing targeted pages/chapters (or all when `--all`). Cascades cleanup to Stage 19 final chapters. |
| **Stage 19** | Merge Final Chapters | Deletes finalized chapter documents in `build/02_final_chapters/` containing targeted pages/chapters (or all when `--all`). Preserves Stage 18 drafts for immediate re-merging. |

---

## Safety & Non-Destructive Principles

- **Stage 5 Layout Protection:** Ground-truth layout JSON (`page_XXXX.json`) and page preview images (`page_XXXX.png`) are **NEVER** touched or deleted.
- **Granular Page Scoping:** Only artifacts matching the explicitly targeted page numbers are cleaned up. Sibling pages and unaffected assets remain completely untouched.
- **Queue Synchronization:** Both `build/<stem>_queue.json` (Stage 6) and `build/<stem>_assets_queue.json` (Stage 8..11) have their entries and aggregate stats cleanly recalculated and synchronized.

---

## How to Execute

### 1. Full Reset for Rerun from Stage 7 to Stage 19
Deletes generated Markdown prose, all cropped assets, evaluations, converted fragments, embedded files, proofread files, and chapters, purges obsolete execution plans, and sets queues to `pending`:
```powershell
# Reset across all target manuals when manual is omitted:
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py 7 19 --all

# Reset across all target manuals for a single page:
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py 7 19 --pages 73

# Reset single page for a specific manual:
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "path/to/manual_dir" 7 19 --pages 73

# Reset all pages across a specific manual:
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "path/to/manual_dir" 7 19 --all
```

### 2. Reset from Stage 11 (Re-run Conversion Pipeline)
Preserves all crops from Stages 8–10. Resets conversion queue schema and deletes downstream table/image fragments, embedded pages, proofread pages, and assembled chapters:
```powershell
# Across all target manuals:
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py 11 19 --pages 70-80

# For a specific manual:
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "path/to/manual_dir" 11 19 --pages 70-80
```

### 3. Reset Stage 13 Only (Re-evaluate Table Reduction)
Preserves original HTML tables (`_html.md`). Deletes reduced markdown tables (`_reduced.md`) and clears reduction decisions:
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "path/to/manual_dir" 13 13 --pages 14,28
```

### 4. Reset Stages 16 to 19 (Re-embed, Proofread & Re-assemble Chapters)
Preserves all converted asset fragments (`assets/`). Deletes `page_XXXX-embed.md`, `page_XXXX-proofread.md`, and assembled chapter files:
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "path/to/manual_dir" 16 19 --pages 73
```

### 5. Reset Stage 17 Only (Re-run Page Proofreading)
Deletes `page_XXXX-proofread.md` and invalidates draft and final chapters:
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "path/to/manual_dir" 17 17 --pages 73
```

### 6. Reset Stages 18 and 19 (Re-prepare & Re-merge Chapters)
Deletes draft chapters in `build/02_detect_cont_chapters/` and final chapters in `build/02_final_chapters/`:
```powershell
# Reset all chapters across all target manuals:
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py 18 19 --all

# Reset specific manual's chapters:
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "path/to/manual_dir" 18 19 --all

# Reset specific chapter by number:
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "path/to/manual_dir" 18 19 --chapter 1
```

### 7. Reset Stage 19 Only (Re-merge Chapters from Existing Drafts)
Preserves draft chapters in `build/02_detect_cont_chapters/`. Deletes only finalized documents in `build/02_final_chapters/`:
```powershell
# Reset all finalized chapters across all manuals:
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py 19 19 --all

# Reset single chapter:
python .agents/plugins/pdf-pipeline/skills/pdf-reset-stages/scripts/stage_reset.py "path/to/manual_dir" 19 19 --chapter 1
```

---

## Agent Operational Workflow

When the user asks to clean up, reset, or prepare pages for re-run:

1. **Verify Target Scope:** Identify the starting stage, ending stage, and page number(s) or chapter(s) requested by the user.
2. **Execute Stage Reset:** Call `stage_reset.py` with `--from-stage`, `--to-stage`, and `--pages`, `--chapter`, or `--all`. The tool cleanly unwinds queue states, purges artifacts, and automatically resets & regenerates `build/<stem>_execution_plan.json` with the exact pending work units.
3. **Verify Clean Pipeline State & Execution Plan:** Inspect the reset output summary or `build/<stem>_execution_plan.json` to verify that queue entries are `pending` and the plan steps and chunks match the intended rerun. (If `--no-plan` was used, manually invoke `planner.py` before running the orchestrator).
4. **Proceed to Re-execution:** Sequentially execute the requested pipeline stages via CLI or dispatch to Tier 2 Leads according to the execution plan.


---
name: pdf-stage11-prepare-convert
description: >-
  Stage 11: Prepares and upgrades the asset queue schema (build/<stem>_assets_queue.json) with dedicated tracking fields for conversion (Stages 12, 14), reduction (Stage 13), and embedding (Stage 16).
---

# Stage 11: Prepare Conversion Queue & Asset Schema

This skill executes the preparatory schema migration and tracking initialization for the visual asset conversion phase. It operates deterministically using `stage11_prepare_convert.py` to upgrade `<manual_dir>/build/<stem>_assets_queue.json`, establishing granular tracking fields for generated Markdown fragments, HTML tables, RAG plain text files, reduction statuses, and final Markdown embedding.

## Purpose & Scope
- **Schema Upgrade:** Extends each asset entry with non-destructive, persistent tracking fields:
  - `conversion_status`: `"pending"` or `"completed"` (automatically verified against disk).
  - `conversion_stage`: originating conversion stage number (`12` for Stage 12 HTML tables, `14` for Stage 14 images, or `null` when pending).
  - `generated_markdown`: Target path for the active `.md` fragment (e.g. `assets/<asset_id>.md`).
  - `generated_markdown_html`: Preserves original HTML table fragment path (e.g. `assets/<asset_id>_html.md`).
  - `generated_txt`: Target path for plain text technical breakdown used for RAG (e.g. `assets/<asset_id>.txt`).
  - `reduced_to_markdown`: Tracks whether an HTML table was reduced to a Markdown table in Stage 13.
  - `reduction_reason`: Explanatory rationale for reduction or preservation of HTML structure.
  - `is_embedded`: Tracks whether the asset has been injected into `page_XXXX-embed.md` in Stage 16.
  - `embedded_file`: Relative path to the per-page Markdown document (`page_XXXX-embed.md`).
- **Directory Scaffolding:** Ensures target directory `build/01_page_layout/assets/` exists.
- **Conversion Dashboard:** Provides a consolidated status report (`--status`) summarizing progress across all downstream conversion and finalization stages.

## Input & Output
- **Input:**
  - `<manual_dir>/build/<stem>_assets_queue.json` (from Stage 9/10 with `eval_clip: "ok"`)
- **Output:**
  - Upgraded `<manual_dir>/build/<stem>_assets_queue.json` with conversion metadata and aggregate statistics (`conversion_stats`, `reduction_stats`, `embed_stats`).
  - Scaffolded directory for asset fragments (`build/01_page_layout/assets/`).

---

## How to Execute

### 1. Upgrade Queue Schema
Run Stage 11 on any manual directory to initialize tracking fields:
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage11-prepare-convert/scripts/stage11_prepare_convert.py "68000 Programmer's Reference Manual"
```

### 2. Inspect Conversion Status Dashboard
Print the current conversion, reduction, and embedding readiness:
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage11-prepare-convert/scripts/stage11_prepare_convert.py "68000 Programmer's Reference Manual" --status
```

### 3. Reset Tracking Fields (If Re-running Pipeline)
Reset all conversion and embedding fields to initial pending state:
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage11-prepare-convert/scripts/stage11_prepare_convert.py "68000 Programmer's Reference Manual" --reset
```

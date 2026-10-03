---
name: pdf-stage16-embed
description: >-
  Stage 16: Embeds converted asset fragments (GFM tables, HTML tables, and image breakdowns) into per-page Markdown documents, producing build/01_page_layout/page_XXXX-embed.md.
---

# Stage 16: Embed Markdown

This skill executes the asset embedding step on a per-page basis. It reads `build/01_page_layout/page_XXXX-images-clip_final.md`, locates each asset image link, substitutes it with the corresponding active converted fragment (`build/01_page_layout/assets/<asset_id>.md`), and outputs the per-page embedded document into `build/01_page_layout/page_XXXX-embed.md`.

## Purpose & Scope
- **Seamless Injection:**
  - `table_markdown`: Replaces image link with clean GFM Markdown table.
  - `table_html` (reduced): Replaces image link with the reduced GFM Markdown table from Stage 13 (`assets/<asset_id>_reduced.md`).
  - `table_html` (non-reduced): Replaces image link with semantic HTML `<table>` plus collapsed text table view.
  - `image`: Replaces image link with the original PNG image link followed by the collapsed `<details><summary>RAG</summary>` technical analysis from Stage 14.
  - **Collapsible Spacing Invariant:** Automatically appends `<br>` after closing `</details>` tags on embedded fragments to guarantee proper vertical paragraph separation in Markdown previews.
- **Dedicated Embedded Page Document:** Generates `build/01_page_layout/page_XXXX-embed.md`.
- **Queue Lineage & Tracking:** Updates `<manual_dir>/build/<stem>_assets_queue.json`:
  - Sets `"is_embedded": true`.
  - Sets `"embedded_file": "build/01_page_layout/page_XXXX-embed.md"`.
  - Recalculates aggregate embedding statistics.

## Input & Output
- **Input:**
  - `<manual_dir>/build/<stem>_assets_queue.json`
  - `build/01_page_layout/page_XXXX-images-clip_final.md`
  - `build/01_page_layout/assets/<asset_id>.md` (or `_reduced.md`, `_html.md`)
- **Output:**
  - `build/01_page_layout/page_XXXX-embed.md` (Per-page embedded Markdown file)
  - Updated `<manual_dir>/build/<stem>_assets_queue.json` (`is_embedded: true`)

---

## How to Execute

### 1. Embed All Converted Pages
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage16-embed/scripts/stage16_embed.py "68000 Programmer's Reference Manual"
```

### 2. Embed Specific Page Range
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage16-embed/scripts/stage16_embed.py "68000 Programmer's Reference Manual" --pages 14-25
```

### 3. Check Embedding Status Summary
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage16-embed/scripts/stage16_embed.py "68000 Programmer's Reference Manual" --status
```

---
name: pdf-stage12-convert-table-html
description: >-
  Stage 12: Multimodal conversion of visual table assets into semantic HTML tables paired with collapsed fixed-width text table views, and direct classification of non-table assets as images.
---

# Stage 12: Convert Tables & Classify Images

This skill governs the identification and transcription of all visual data tables (simple rectangular tables, tables with `colspan`/`rowspan`, multiline cells, dual alignments) into semantic HTML tables paired with collapsed fixed-width text table views for RAG indexing into `build/01_page_layout/assets/<asset_id>_html.md`, while directly classifying non-table assets as images for Stage 14.

1. **HTML Tables (`table_html`):** If the visual asset is clearly a table and can be written as an HTML table, transcribes it into semantic HTML `<table>` elements paired with a collapsed fixed-width text table view for RAG indexing into `build/01_page_layout/assets/<asset_id>_html.md`.
2. **Visual Diagrams & Images (`image`):** Any asset that is not a data table (e.g. register bitfield diagrams, cycle allocation grids, signal routing maps, schematics, waveforms, block diagrams, flowcharts, packaging pinouts) is **directly classified as `image`** (`detected_type: "image"`, `status: "checked"`, `conversion_status: "pending"`, `conversion_stage": null`) for Stage 14 (`pdf-stage14-convert-image`).

## Purpose & Scope
- **Unified & Single-Pass Table Extraction:** Inspects each visual crop once. If it represents tabular data, convert it directly. If it is a non-tabular diagram or hardware figure, classify it directly as `image`.
- **Direct Image Classification:** All non-table assets bypass intermediate table stages and are directly designated as `image` for downstream architectural breakdown in Stage 14.
- **Semantic HTML Table Representation:**
  - Standard semantic `<table border="1">`, `<thead>`, `<tbody>`, `<tr>`, `<th>`, and `<td>` tags.
  - Mandates `<table border="1">` to ensure technical reference tables render with complete outer frames and internal column/row grids across all previewers.
  - Supports `colspan` and `rowspan` where columns or rows naturally span across headers or data cells.
  - Clean markup without inline CSS styles (unless needed for dual left/right alignment) or background color hacks.
- **Collapsed RAG Text Table View (For RAG Optimization):**
  - Following the HTML table, includes a collapsed `<details><summary>RAG</summary>` block providing the exact tabular content transcribed into fixed-width, space-aligned plain text wrapped in a ```` ```text ... ``` ```` code block.
- **Queue Lineage Tracking:** Updates `<manual_dir>/<stem>_assets_queue.json`:
  - For Converted HTML Tables: Sets `"detected_type": "table_html"`, `"status": "checked"`, `"conversion_status": "completed"`, `"conversion_stage": 12`, `"generated_markdown": "build/01_page_layout/assets/<asset_id>_html.md"`, `"generated_markdown_html": "build/01_page_layout/assets/<asset_id>_html.md"`.
  - For Visual Images & Diagrams: Sets `"detected_type": "image"`, `"status": "checked"`, `"conversion_status": "pending"`, `"conversion_stage": null` (ready for Stage 14).

## Input & Output
- **Input (Read-Only):**
  - `<manual_dir>/<stem>_assets_queue.json` (Queue tracking asset status and paths)
  - `build/01_page_layout/assets/<asset_id>_clip_final.png` (Finalized high-res crop)
- **Output:**
  - For HTML Tables: `build/01_page_layout/assets/<asset_id>_html.md` (Semantic HTML table + collapsed RAG text table view)
  - Updated `<manual_dir>/<stem>_assets_queue.json` for converted tables and classified images.

---

## Execution Protocol

All prompt instructions and rules for table transcription are defined in:
👉 **[`prompt.md`](./prompt.md)**

For each assigned asset in the chunk payload:
1. **Inspect Visual Crop:** Call `view_file` on `build/01_page_layout/assets/<asset_id>_clip_final.png`.
2. **Evaluate Table Structure:**
   - **If clearly a table:** Transcribe into clean semantic HTML `<table>` with `colspan`/`rowspan` where appropriate, paired with a collapsed `<details>` fixed-width text table view per [`prompt.md`](./prompt.md). Call `write_to_file` to save `build/01_page_layout/assets/<asset_id>_html.md`.
   - **If NOT a table (visual diagram/graphic):** Classify as `image` without writing any file.
3. **Completion Contract:** When all assigned assets in the chunk are processed, terminate and return the compact JSON status contract:
   ```json
   {
     "status": "completed",
     "stage": 12,
     "chunk_id": "<CHUNK_ID>",
     "converted_tables": ["<asset_id>", ...],
     "classified_images": ["<asset_id>", ...]
   }
   ```
   Do not echo HTML or Markdown table markup in the return message.

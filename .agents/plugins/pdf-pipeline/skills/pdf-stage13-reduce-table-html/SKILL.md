---
name: pdf-stage13-reduce-table-html
description: >-
  Stage 13: Two-step hybrid table reduction transcribing verified flat rectangular crops directly into pure GFM Markdown with native LaTeX math.
---

# Stage 13: Reduce HTML Tables to Markdown (Hybrid Gatekeeper & Table Reduction)

This skill governs Stage 13 table reduction following Stage 12 HTML table conversion. It ensures that simple rectangular tables become 100% clean, native GFM Markdown tables with proper mathematical expressions while complex tables remain as semantic HTML.

## Architectural Flow
```
[ HTML Tables from Stage 12 (assets/<id>_html.md) ]
                        │
                        ▼
    ┌───────────────────────────────────────┐
    │  Step 1: Deterministic Gatekeeper     │
    │  (scripts/stage13_reduce_table_html)  │
    └───────────────────────────────────────┘
                        │
       ┌────────────────┴────────────────┐
       ▼                                 ▼
[ Requires HTML? ]               [ Eligible Simple Grid ]
- Rowspan / Colspan > 1          - Uniform rectangular rows
- Multiline cells (<br>, <p>)    - Single-line cells
- Ragged column counts           - Zero merged cells
       │                                 │
       ▼                                 ▼
Set reduced_to_markdown: false   Queue: Pending Reduction
Kept as semantic HTML                   │
                                        ▼
                         Inspects visual crop (_clip_final.png)
                         Emits pure GFM Markdown with LaTeX math
                         Saves to assets/<id>_reduced.md
                         Zero raw HTML tags in Markdown!
```

## Purpose & Scope
- **Pure GFM Markdown:** Guarantees zero raw HTML tags (`<span style="...">`, `<sup>`, `<sub>`, `<font>`) in reduced tables.
- **Native LaTeX Mathematical Notation:** Boolean minterms (`$\overline{A}\overline{B}\overline{C}$`), exponential functions (`$e^x$`, `$10^x$`), logarithms (`$\log_{10}(x)$`), and active-low signals (`$\overline{\text{UDS}}$`, `$\overline{\text{LDS}}$`, `$\text{R}/\overline{\text{W}}$`) are transcribed with high fidelity from visual crops.
- **Zero Loss of Structure:** Tables requiring merged cells (`rowspan`/`colspan`) or multiline content are strictly preserved as semantic HTML with an explanatory reason recorded in `reduction_reason`.
- **Dynamic Pointer Updates:** When reduced, updates the active `generated_markdown` pointer to `assets/<asset_id>_reduced.md` so that Stage 16 embeds the clean Markdown table. The original HTML lineage is preserved in `generated_markdown_html`.

## Input & Output
- **Input:**
  - `<manual_dir>/<stem>_assets_queue.json` (Queue tracking asset status and paths)
  - `build/01_page_layout/assets/<asset_id>_html.md` (Stage 12 HTML table output, read-only)
  - `build/01_page_layout/assets/<asset_id>_clip_final.png` (High-resolution visual crop for Agent inspection, read-only)
- **Output:**
  - `build/01_page_layout/assets/<asset_id>_reduced.md` (Clean GFM Markdown table for reduced assets)
  - Updated `<manual_dir>/<stem>_assets_queue.json`:
    - `reduced_to_markdown: true` (or `false`)
    - `reduction_reason: "..."`
    - `generated_markdown`: points to active representation (`assets/<asset_id>_reduced.md` if reduced, or `assets/<asset_id>_html.md` if kept)
    - `generated_markdown_html`: preserves original HTML path (`assets/<asset_id>_html.md`)

---

## Execution Guide

### Step 1: Run Deterministic Gatekeeper Filter (CLI)
Runs across the manual to filter out tables that must stay HTML and identify reduction candidates:
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage13-reduce-table-html/scripts/stage13_reduce_table_html.py "Hardware Reference Manual"
```
Or for a specific page range:
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage13-reduce-table-html/scripts/stage13_reduce_table_html.py "Hardware Reference Manual" --pages 186
```

### Step 2: Worker Reduction Protocol

All reduction rubrics and math formatting patterns are defined in:
👉 **[`prompt.md`](./prompt.md)**

For each assigned asset in the chunk payload:
1. **Inspect Visual Crop:** Call `view_file` on `build/01_page_layout/assets/<asset_id>_clip_final.png`.
2. **Evaluate Reducibility:**
   - **If reducible:** Transcribe directly into clean GFM Markdown with native LaTeX math per [`prompt.md`](./prompt.md). Call `write_to_file` to save `build/01_page_layout/assets/<asset_id>_reduced.md`.
   - **If kept as HTML (bailout):** Record bailout reason in the completion contract without writing any files.
3. **Completion Contract:** When all assigned assets in the chunk are processed, terminate and return the compact JSON status contract:
   ```json
   {
     "status": "completed",
     "stage": 13,
     "chunk_id": "<CHUNK_ID>",
     "reduced_to_markdown": ["<asset_id>", ...],
     "kept_as_html": [{"asset_id": "<asset_id>", "reason": "..."}]
   }
   ```
   Do not echo table Markdown in the return message.

### Step 3: Status Check
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage13-reduce-table-html/scripts/stage13_reduce_table_html.py "Hardware Reference Manual" --status
```

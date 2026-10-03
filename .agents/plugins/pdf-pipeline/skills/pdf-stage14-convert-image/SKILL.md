---
name: pdf-stage14-convert-image
description: >-
  Stage 14: Multimodal conversion of visual image assets into Markdown image links with collapsed architectural breakdowns and dedicated plain text files for RAG.
---

# Stage 14: Convert Images & RAG Text

This skill governs the multimodal inspection and architectural decomposition of visual diagram assets classified as `image` in Stage 12 (system architecture schematics, bus timing waveforms, pinout configurations, memory addressing diagrams, state transitions, flowcharts).

It inspects cropped PNG images (`build/01_page_layout/assets/<asset_id>_clip_final.png`) via `view_file` and generates two distinct outputs:
1. **Markdown Fragment (`assets/<asset_id>.md`):** The original image link followed by an in-depth visual and architectural breakdown enclosed in a fenced code block (` ```text ... ``` `) collapsed inside `<details><summary>RAG</summary>`.
2. **Dedicated RAG Text File (`assets/<asset_id>.txt`):** The pure, flat plain-text structural analysis saved alongside the asset (with zero Markdown embellishments, bolding, headings, or HTML tags), structured for dense vector embedding and retrieval-augmented generation (RAG).

## Purpose & Scope
- **Target Assets:** Strictly processes assets where `"detected_type": "image"` and `"conversion_status": "pending"`.
- **Preserves Original PNG:** Retains standard Markdown image syntax (`![Caption](assets/<asset_id>_clip_final.png)`).
- **In-Depth Technical Breakdown:** Synthesizes an exhaustive visual description:
  - Identifies all functional blocks, modules, subsystems, registers, and components.
  - Documents signal arrows, directions, interconnections, and active polarities.
  - Details pinout numbers, pin names, package styles, or interface groupings.
  - Traces operational flows, state transitions, clock phases, timing sequences, or spatial boundaries.
  - Faithfully captures all visible formulas, annotations, and parameters for searchability.
- **Collapsible User Experience:** Wraps the plain-text breakdown in a ` ```text ... ``` ` code block within `<details><summary>RAG</summary>` to preserve clean visual document flow while offering deep technical context on demand without Markdown parsing quirks.
- **RAG Text Export:** Copies the structured plain-text description into `assets/<asset_id>.txt` to enable keyword and semantic vector search across all non-tabular visual documentation.
- **Queue Lineage Tracking:** Updates `<manual_dir>/<stem>_assets_queue.json`:
  - Sets `"conversion_status": "completed"`.
  - Sets `"conversion_stage": 14`.
  - Sets `"generated_markdown": "build/01_page_layout/assets/<asset_id>.md"`.
  - Sets `"generated_txt": "build/01_page_layout/assets/<asset_id>.txt"`.

## Input & Output
- **Input (Read-Only):**
  - `<manual_dir>/<stem>_assets_queue.json` (Queue tracking asset status and paths)
  - `build/01_page_layout/assets/<asset_id>_clip_final.png` (Finalized high-res crop)
- **Output:**
  - `build/01_page_layout/assets/<asset_id>.md` (Image link + collapsed plain text code block)
  - `build/01_page_layout/assets/<asset_id>.txt` (Pure flat plain-text technical breakdown for RAG)
  - Updated `<manual_dir>/<stem>_assets_queue.json` (`conversion_status: "completed"`, `conversion_stage": 14`)

---

## Execution Protocol

All image analysis rubrics and domain conventions are defined in:
👉 **[`prompt.md`](./prompt.md)**

For each assigned image asset in the chunk payload:
1. **Inspect Visual Crop:** Call `view_file` on `build/01_page_layout/assets/<asset_id>_clip_final.png`.
2. **Synthesize Architectural Breakdown:** Decompose modules, components, registers, signal flows, active polarities, and pinouts per [`prompt.md`](./prompt.md).
3. **Save Output Files:**
   - Call `write_to_file` to save `build/01_page_layout/assets/<asset_id>.md` (Standard image link + collapsed `<details>` block enclosing ` ```text ... ``` `).
   - Call `write_to_file` to save `build/01_page_layout/assets/<asset_id>.txt` (Clean flat plain-text technical breakdown for dense RAG indexing).
4. **Completion Contract:** When all assigned assets in the chunk are processed, terminate and return the compact JSON status contract:
   ```json
   {
     "status": "completed",
     "stage": 14,
     "chunk_id": "<CHUNK_ID>",
     "images_converted": ["<asset_id>", ...]
   }
   ```
   Do not echo breakdown text in the return message.

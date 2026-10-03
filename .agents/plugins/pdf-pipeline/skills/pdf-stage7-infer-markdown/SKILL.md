---
name: pdf-stage7-infer-markdown
description: >-
  Stage 7: Transcribes layout JSON and preview images into clean Markdown prose sequentially using the Stage 6 queue. Replaces figures, schematics, and tables with normalized 1000x1000 <crop> tags.
---

# Stage 7: Infer Markdown Prose with 1000x1000 Crop Bounding Boxes

This skill governs the multimodal transcription of page layout and preview images into clean, faithful Markdown prose. For each assigned page, it transforms the Stage 5 layout JSON and preview image into readable, publication-grade Markdown with atomic `<crop box="[ymin, xmin, ymax, xmax]" />` tags for visual figures, tables, and schematics.

## Purpose & Scope
- **Sequential Page Processing:** Processes assigned pages file-by-file with isolated multimodal inspection.
- **Zero API Key Dependency:** Operates 100% within the worker session using native multimodal inspection (`view_file`) and workspace tools (`write_to_file`). No `GEMINI_API_KEY` and no external batch endpoints are required.
- **1:1 Content Fidelity:** Transcribes page prose faithfully with zero additions and zero omissions per [`prompt.md`](./prompt.md).
- **Register Summaries & Metadata:** Transcribes hardware register summaries and metadata headers as clean Markdown headings (`## REG`) and structured bullet lists, never cropping entire register sections.
- **Source Code Listings:** Formats assembly, C, and other programming examples into fenced code blocks with appropriate language tags, never cropping them as images and repairing OCR character spacing.
- **Preformatted ASCII Art & Monospaced Register Dumps:** Preserves character-drawn trees, memory maps, signal branches, and vertical bit listings with nested tables directly in ```` ```text ```` fenced code blocks with OCR repair, never cropping them as visual tables.
- **Visual Fallback:** Defers true graphics, schematics, pinouts, and rich data tables / horizontal word bitfield grids to `<crop>` tags.
- **Mathematical Expressions (LaTeX):** Preserves formulas and equations as native Markdown/LaTeX (`$...$` and `$$...$$`), restoring any missing symbols by cross-checking the page preview image.
- **Table of Contents, Figure/Table Lists & Subject Indexes:** Strips dot leaders, page markers, and obsolete physical page/chapter references (e.g. `3-3, 3-4`, `5-15`, `12-1`), emitting clean hierarchical Markdown lists.
- **Normalized 1000x1000 Coordinates:** Uses standard `[ymin, xmin, ymax, xmax]` normalized spatial coordinate grid (`0..1000`).

## Input & Output
- **Input (Read-Only):**
  - `build/01_page_layout/page_XXXX.json` (Stage 5 layout text blocks and OCR stream)
  - `build/01_page_layout/page_XXXX.png` (Stage 5 high-resolution page visual preview)
- **Output:**
  - `build/01_page_layout/page_XXXX.md` (Transcribed Markdown prose with atomic `<crop>` tags)

---

## Execution Protocol

All transcription rules and quality standards are defined in:
👉 **[`prompt.md`](./prompt.md)**

For each assigned page in the chunk payload:
1. **Inspect Layout:** Call `view_file` on `build/01_page_layout/page_XXXX.json` to read the layout blocks and OCR text stream.
2. **Inspect Visual Context:** Call `view_file` on `build/01_page_layout/page_XXXX.png` to examine visual structure, graphics, equations, and code listings.
3. **Infer Markdown Prose:**
   - Transcribe text faithfully according to [`prompt.md`](./prompt.md).
   - Reconstruct mathematical formulas in standard LaTeX (`$...$` or `$$...$$`).
   - Format source code listings into fenced blocks (```` ```assembly ````, ```` ```c ````).
   - For every table, schematic, or diagram, calculate the normalized integer bounding box `[ymin, xmin, ymax, xmax]` (`0..1000`) and insert `<crop box="[ymin, xmin, ymax, xmax]" />`.
4. **Save Page Markdown:** Call `write_to_file` to write clean Markdown directly to `build/01_page_layout/page_XXXX.md`. Leaf workers do NOT edit queue files on disk.
5. **Completion Contract:** When all assigned pages are written to disk, terminate and return the compact JSON status contract:
   ```json
   {
     "status": "completed",
     "stage": 7,
     "chunk_id": "<CHUNK_ID>",
     "pages_completed": ["page_XXXX", ...]
   }
   ```
   Do not echo transcribed Markdown prose in the return message.

---

## Expected Output Structure

```markdown
# Section 1 - Summary of Differences

This manual presents technical documentation for three different Amiga models, comparing them to the original Amiga, referred to as model A1000.

## RAW KEY CODES ON THE KEYBOARD

### Keyboard Layout Showing Raw Key Codes

<crop box="[125, 45, 410, 955]" />

Note: On the U.S. keyboard, the keys with codes 44 and 60 are extended to include the European keys with codes 2B and 30, respectively.

## RS232 AND MIDI CONNECTOR COMPARISON

<crop box="[520, 50, 780, 940]" />
```

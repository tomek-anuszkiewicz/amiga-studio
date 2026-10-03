---
name: pdf-stage5-extract-layout
description: >-
  Stage 5: Extracts per-page text blocks, layout geometry, and high-resolution page preview PNGs from a PDF. Writes non-destructively to build/01_page_layout/.
---

# Stage 5: Extract Page Layout & Text Blocks

This skill executes layout and text extraction. It produces atomic per-page JSON files with clean text blocks, filtered running headers/footers, mathematical notation detection, and high-resolution page preview images.

## Purpose & Scope
- Operates per-page on technical PDF manuals.
- Filters running headers and footers based on geometric $Y$-coordinates.
- Extracts text representations in `text_blocks`: standard unformatted `text`, and an optional `latex` property present only when mathematical notation is detected.
- Generates high-resolution page previews (`page_XXXX.png` at 150 DPI) providing ground-truth visual context.
- Outputs structured JSON (`page_XXXX.json`) containing clean text blocks and the full reading-order text stream (`text_stream`).

## Input & Output
- **Input (Read-Only):** PDF with text layer (e.g. `manual_ocr.pdf`).
- **Output Directory:** `build/01_page_layout/` (or `--out <dir>`)
  - `page_0001.json`, `page_0001.png`
  - `page_0002.json`, `page_0002.png`

---

## How to Execute

### 1. Process All Pages
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage5-extract-layout/scripts/stage5_extract_layout.py "path/to/manual_ocr.pdf" --out "build/01_page_layout"
```

### 2. Process a Specific Range (e.g. for testing)
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage5-extract-layout/scripts/stage5_extract_layout.py "path/to/manual_ocr.pdf" --pages 4-15 --out "build/01_page_layout"
```

### 3. Custom Header / Footer Margins
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage5-extract-layout/scripts/stage5_extract_layout.py "path/to/manual_ocr.pdf" --header-margin 50.0 --footer-margin 55.0
```

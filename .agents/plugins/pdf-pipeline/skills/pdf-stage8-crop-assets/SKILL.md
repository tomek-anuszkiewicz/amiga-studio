---
name: pdf-stage8-crop-assets
description: >-
  Stage 8: Extracts high-resolution PNG crops from the original PDF using Stage 7 normalized 1000x1000 crop coordinates, saves them to assets/, and replaces <crop ... /> tags with standard Markdown image links.
---

# Stage 8: Crop Visual Assets & Link Injection

This skill executes the final asset generation and link resolution step. It operates non-destructively by reading Stage 7 Markdown files, descaling normalized 1000x1000 bounding box coordinates directly to PDF geometry, and writing final Markdown files alongside an `assets/` directory.

## Purpose & Scope
- Scans Stage 7 Markdown files for `<crop box="[ymin, xmin, ymax, xmax]" ... />` tags.
- Descales normalized `[0, 1000]` coordinates to exact PDF floating-point dimensions (`[x0, y0, x1, y1]`).
- Crops the corresponding region from the source PDF page in high resolution (**300 DPI** default).
- Saves images to `assets/page_XXXX_slug.png`.
- Replaces `<crop ... />` with standard Markdown:
  `![Caption](assets/page_XXXX_slug.png)`
- Writes completed Markdown documents into `build/01_page_layout/` with `-images.md` suffix.

## Input & Output
- **Input (Read-Only):**
  - Source PDF file
  - `build/01_page_layout/page_XXXX.md` (Stage 7 per-page Markdown files with `<crop>` tags)
- **Output Artifacts:**
  - `build/01_page_layout/assets/` (all cropped PNG images)
  - `build/01_page_layout/page_XXXX-images.md` (final per-page Markdown files with active image links)
  - `<manual_dir>/build/<stem>_assets_queue.json` (bootstrapped assets queue capturing initial bounding boxes)

---

## How to Execute

### 1. Run Standard Asset Extraction (All Books or Specific Manual)
```powershell
# Run across all target manuals (default when no manual/PDF parameter is provided):
python .agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py

# Or run for a specific manual directory or PDF file:
python .agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py "Hardware Reference Manual"
python .agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py "path/to/manual_ocr.pdf"
```

### 2. Process Specific Page Range or Single File
```powershell
# Filter by pages across all manuals or a specific manual:
python .agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py --pages 4-15
python .agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py "path/to/manual_ocr.pdf" --pages 4-15

# Or process a single markdown file:
python .agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py --file "Hardware Reference Manual/build/01_page_layout/page_0017.md"
```

### 3. High-DPI Output (e.g. 600 DPI for schematics)
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py --dpi 600
python .agents/plugins/pdf-pipeline/skills/pdf-stage8-crop-assets/scripts/stage8_crop_assets.py "path/to/manual_ocr.pdf" --dpi 600
```

---

## Transformation Example

### Before (Stage 7 Markdown output):
```markdown
## RAW KEY CODES ON THE KEYBOARD

<crop box="[125, 45, 410, 955]" caption="Figure 1.1: Raw Key Codes on the Keyboard" />

Note: On the U.S. keyboard...
```

### After (Stage 8 Final output):
```markdown
## RAW KEY CODES ON THE KEYBOARD

![Figure 1.1: Raw Key Codes on the Keyboard](assets/page_0004_figure_11_raw_key_codes_on_the_keyboard.png)

Note: On the U.S. keyboard...
```
*(With `page_0004_figure_11_raw_key_codes_on_the_keyboard.png` saved as a crisp 300 DPI PNG in `assets/`).*

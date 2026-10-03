---
name: pdf-stage15-render-asset-frames
description: >-
  Stage 15: Deterministic rendering of classified asset review frames (build/01_page_layout/asset_frames/page_XXXX_asset_frame.png) displaying final bounding boxes with semantic tags (MD, HTML, IMG, ERR).
---

# Stage 15: Render Asset Frames (`pdf-stage15-render-asset-frames`)

Stage 15 is a **deterministic CLI stage** that renders full-page visual review canvases into `build/01_page_layout/asset_frames/page_XXXX_asset_frame.png`. It executes immediately following asset conversion and reduction (Stages 11–14), visually confirming how every visual asset on the page has been finalized:
- **`MD`**: Tables that were reduced to clean GFM Markdown in Stage 13 (`reduced_to_markdown: true`) or standard markdown tables.
- **`HTML`**: Tables converted to semantic HTML in Stage 12 (`conversion_stage == 12`).
- **`IMG`**: Visual images, schematics, and waveforms converted in Stage 14 with collapsed architectural breakdowns.
- **`ERR`**: Fallback error badge for assets with invalid or missing origin.

---

## Input & Output
- **Input (Read-Only):**
  - Base page previews: `build/01_page_layout/page_XXXX.png`
  - Asset queue: `<manual_dir>/build/<stem>_assets_queue.json`
- **Output:**
  - Full-page review frames: `build/01_page_layout/asset_frames/page_XXXX_asset_frame.png`

---

## Visual Design & Geometry

1. **Strictly Outward Red Bounding Borders:**
   - Red rectangle drawn strictly OUTSIDE the inner bounding box coordinates (`xmin, ymin, xmax, ymax`).
   - Stroke width: 6 pixels.
   - Guaranteed zero encroachment on table ink, bit numbers, diagrams, or borders.
2. **Classified Semantic Badges:**
   - Solid red label rectangle with white bold text anchored outside the upper-left corner of the asset box.
   - Badges: `[MD]`, `[HTML]`, `[IMG]`, `[ERR]`.
3. **Full-Page Crop Banner:**
   - If an asset spans `[0, 0, 1000, 1000]`, a centered top banner is drawn: `FULL-PAGE CROP (NO VISIBLE BORDER) [TYPE]`.
4. **Pages Without Visual Assets:**
   - Clean base page preview is preserved into `asset_frames/` so the review directory contains a complete, contiguous 1:1 view of all pages.

---

## Usage

```powershell
# Render asset frames for entire manual:
python .agents/plugins/pdf-pipeline/skills/pdf-stage15-render-asset-frames/scripts/stage15_render_asset_frames.py "68000 Programmer's Reference Manual"

# Render asset frames for specific page range:
python .agents/plugins/pdf-pipeline/skills/pdf-stage15-render-asset-frames/scripts/stage15_render_asset_frames.py "68000 Programmer's Reference Manual" --pages 37,40,42

# Force re-rendering of existing asset frames:
python .agents/plugins/pdf-pipeline/skills/pdf-stage15-render-asset-frames/scripts/stage15_render_asset_frames.py "68000 Programmer's Reference Manual" --force

# Check status & coverage:
python .agents/plugins/pdf-pipeline/skills/pdf-stage15-render-asset-frames/scripts/stage15_render_asset_frames.py "68000 Programmer's Reference Manual" --status
```

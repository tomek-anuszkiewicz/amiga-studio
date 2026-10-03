---
name: pdf-stage9-prepare-eval-clip
description: >-
  Stage 9: Prepares Red/Blue full-page annotated evaluation frames, applies iterative asset recrops (_v1.png, _v2.png), locks confirmed crops into _clip_final.png, and governs the Stage 9 <-> Stage 10 loop.
---

# Stage 9: Prepare Eval Frames & Recrop Governor (eval-clip)

This skill executes the deterministic, Python-driven portion of the recursive crop evaluation loop. It bridges the initial asset creation from Stage 8 with the multimodal evaluation in Stage 10 by rendering annotated full-page frames, applying recrops with newly proposed coordinates, and locking validated assets into final form.

## Purpose & Scope
- **Dual-Color Visual Canvas Generation (`--prep-frames`):**
  - Inspects perimeter pixels and the **Safe White Buffer (50 units = 5% page width)** to detect cut-off letters or words across whitespace gaps.
  - Automatically flags `"margin_defect": true` and calculates candidate `expanded_box` in `<manual>_assets_queue.json`.
  - Renders the full-page PNG (`page_XXXX.png`).
  - Highlights the **active target asset** in a **thick RED frame** (`#FF0000`, 6 px). If `margin_defect` is true, the solid RED frame frames the `expanded_box` with an inner light-red outline showing the original cut.
  - Highlights all **sibling assets on the same page** in **BLUE frames** (`#0066FF`, 4 px).
  - Saves annotated views into `build/01_page_layout/eval_frames/`.
- **Unified Final Evaluation Canvas Generation (`--eval-final` / `--prep-final-frames`):**
  - Generates full-page PNGs in `build/01_page_layout/eval_final/page_XXXX_eval_final.png` for **all document pages**.
  - Highlights **all assets on the page** simultaneously in **thick RED frames** (`#FF0000`, 6 px) with asset identifier badges (`crop_1`, `crop_2`, `crop_3`).
  - Full-page assets (`[0, 0, 1000, 1000]`) feature a prominent top informational banner: `FULL-PAGE CROP (NO VISIBLE BORDER) [{short_id}]`.
  - Pages without assets are cleanly included as-is, allowing sequential flipping through the entire manual to catch any missed visual elements or verify crop boundaries prior to conversion and classification in Stage 12.
  - Automatically refreshed whenever `--finalize` is executed.
- **Iterative Recrop Execution (`--apply-recrops`):**
  - When Stage 10 assigns corrected coordinates, Stage 9 recrops the region from the source PDF at **300 DPI**.
  - Generates versioned files: `page_XXXX_crop_Y_v1.png`, `_v2.png`.
  - Appends the revision history into `<manual>_assets_queue.json`.
  - Immediately re-renders the updated Red/Blue frame for the next evaluation pass.
- **Asset Finalization (`--finalize`):**
  - Once an asset is confirmed `eval_clip: ok`, locks it as `page_XXXX_crop_Y_clip_final.png`.
  - Updates all image links in `build/01_page_layout/page_XXXX-images.md`.
  - Automatically renders updated unified multi-asset frames in `eval_final/`.
- **Loop Status & Governor (`--status`):**
  - Monitors convergence across all assets.
  - Automatically reports whether the next recommended action belongs to Stage 9 (recropping / frame generation) or Stage 10 (Agent multimodal review).

## Input & Output
- **Input:**
  - `<manual_dir>/build/<stem>_assets_queue.json` (created in Stage 8 with bounding box lineage)
  - `build/01_page_layout/page_XXXX.png` (high-resolution full page preview)
  - Source PDF (for executing 300 DPI recrops)
- **Output:**
  - `build/01_page_layout/eval_frames/*_eval.png` (per-asset Red/Blue evaluation frames for Stage 10)
  - `build/01_page_layout/eval_final/*_eval_final.png` (unified multi-asset final frames for rapid review)
  - `build/01_page_layout/assets/*_vN.png` (versioned recrops)
  - `build/01_page_layout/assets/*_clip_final.png` (locked final assets)
  - Updated `<manual_dir>/build/<stem>_assets_queue.json` and `page_XXXX-images.md`

---

## How to Execute

### 1. Check Loop Status
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py "path/to/manual_dir" --status
```

### 2. Generate Red/Blue Full-Page Frames for Pending Assets
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py "path/to/manual_dir" --prep-frames

# Or restrict to a specific page range:
python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py "path/to/manual_dir" --prep-frames --pages 14
```

### 3. Generate Unified Final Multi-Asset Frames (eval_final)
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py "path/to/manual_dir" --eval-final

# Or restrict to specific pages:
python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py "path/to/manual_dir" --eval-final --pages 17,26
```

### 4. Apply Recrops After Stage 10 Review
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py "path/to/manual_dir" --apply-recrops
```

### 5. Finalize Confirmed Assets
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage9-prepare-eval-clip/scripts/stage9_prepare_eval_clip.py "path/to/manual_dir" --finalize
```

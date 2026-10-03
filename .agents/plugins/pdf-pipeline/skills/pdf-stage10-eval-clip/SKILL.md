---
name: pdf-stage10-eval-clip
description: >-
  Stage 10: Multimodal evaluation and re-cropping of visual assets using red bounding box overlays.
---

# Stage 10: Evaluate & Reclip Assets

This skill governs the visual inspection and boundary re-evaluation phase for cropped visual assets. Using Method 2 (visualizing the crop bounding box as a thick red rectangle against the page or context view), the visual model inspects the crop perimeter, identifies clipped lines or severed characters/bit numbers, determines corrected coordinates, and oversees re-clipping until the asset is cleanly framed.

## Purpose & Scope
- **Boundary & Crop Quality Validation:** Direct visual inspection of the crop boundary against the surrounding page context via a thick red bounding box overlay.
- **Defect Detection:**
  - `clipped_edge`: Outer border lines, boxes, or schematics cut off at the image perimeter.
  - `severed_text`: Register bitfield numbers (e.g., bits 15..0), labels, pin names, or characters sliced in half.
  - `bad_bbox`: Bounding box misaligned, missing content, or capturing extraneous body prose.
  - `excess_whitespace`: Excessive empty margins around a small graphic.
  - `text_as_markdown`: Erroneous crop capturing standard text paragraphs or bullet points that belong in Markdown.
- **Reclip Loop & Naming:**
  - If a crop is defective, corrected coordinates `[ymin, xmin, ymax, xmax]` (0–1000 scale) are determined.
  - The asset is re-cropped to a new file with suffix `_vN.png` (or `_reclip.png`).
  - The asset remains in the reclip loop until evaluation confirms the framing is `ok`.
- **Handoff to Stage 12:** Once an asset is validated as `ok`, it advances to Stage 11/12 for conversion and classification.

## Input & Output
- **Input (Read-Only):**
  - Evaluation frame overlay (`build/01_page_layout/eval_frames/<asset_id>_eval.png`): Primary multimodal evaluation canvas displaying the entire page context, where the **active target asset** is framed in **thick RED** (`#FF0000`, 6 px) and **sibling assets on the same page** are framed in **BLUE** (`#0066FF`, 4 px).
  - (Optional reference only) Cropped visual asset PNG (`build/01_page_layout/assets/<asset_id>.png` or `_vN.png`).
- **Output:**
  - Compact JSON status contract returning a dictionary of verdicts (`ok` or `reclip_needed` with `box: [ymin, xmin, ymax, xmax]`).

---

## Evaluation Protocol (Single-Call Frame Inspection)

Evaluation is performed in a single direct inspection step using the full-page annotated evaluation frame:

```
[Target Asset in Chunk Payload]
          │
          ▼
┌────────────────────────────────────────────────────────┐
│ Inspect Evaluation Frame (view_file)                   │
│ build/01_page_layout/eval_frames/<asset_id>_eval.png   │
│                                                        │
│ 🟥 RED Frame: Active target asset under evaluation     │
│ 🟦 BLUE Frames: Sibling assets on the same page        │
│                                                        │
│ - Check if RED frame fully encloses graphic & labels   │
│ - Check that inner perimeter does not sever glyphs     │
│ - Check that RED frame does not encroach on siblings   │
│ - Check that surrounding prose is not captured inside  │
└──────────────────────────┬─────────────────────────────┘
                           │
         ┌─────────────────┴─────────────────┐
         │                                   │
         ▼ (Clean Framing)                   ▼ (Defect Detected)
┌───────────────────────┐         ┌───────────────────────────────────────┐
│ Verdict: "ok"         │         │ Verdict: "reclip_needed"              │
│ Ready for Stage 9     │         │ Calculate corrected coordinates:      │
│ finalization          │         │ "box": [ymin, xmin, ymax, xmax]       │
│                       │         │ Trigger Stage 9: --apply-recrops      │
└───────────────────────┘         └───────────────────────────────────────┘
```

### Prerequisite: Full-Page Assets Auto-Pass
Assets with bounding box `[0, 0, 1000, 1000]` (e.g. front covers, full-page scans) encompass 100% of the page area. They are auto-approved as `eval_clip: ok` and bypass frame evaluation.

### Visual Frame Inspection Rules
Call `view_file` on `build/01_page_layout/eval_frames/<asset_id>_eval.png`:
1. **Locate the RED Frame:** This is the active asset being evaluated.
   - **Bounding Box Geometry Rule:** The thick red stroke is drawn on the exterior of the coordinates. The **inner perimeter** of the red rectangle corresponds **exactly** to the crop coordinates `[ymin, xmin, ymax, xmax]`. Everything visible inside the red frame is what is included in the crop.
2. **Observe the BLUE Frames:** Any sibling assets on the same page appear in blue. The red frame must not overlap, swallow, or intrude into blue sibling regions.
3. **Physical Asset Envelope Definition:**
   - **Tables:** Bounded strictly by outer border lines (and adjacent caption/title). Subsequent headings or paragraphs belong to Markdown prose.
   - **Register Bitfields:** Bounded strictly by the bit numbers (15..0) and cell grid. Field descriptions below belong to Markdown prose.
   - **Schematics / Figures:** Bounded by outer component lines, traces, or frame.
4. **Bidirectional Sizing (Expand when clipped, Shrink when over-encompassing):**
   - **Expand when clipped:** If lines, bit numbers, or borders are cut off by the inner edge, **expand** outward with a generous safe margin ($\approx 5\text{--}10$ units).
   - **Shrink when over-encompassing:** If the frame captures extraneous body prose, section headings, notes, or footers, **shrink/contract** inward to snap cleanly to the outer border of the table/graphic.
   - **Relocate if misplaced:** If placed on whitespace or pure prose, relocate to the true visual asset.
5. **Anti-Pixel-Hunting & Pragmatic Tolerance Rule:**
   - **Strict Ban on Micro-Adjustments:** NEVER perform iterative micro-adjustments or generate scratch test crops to test 1–2 unit increments (e.g. testing `125`, `126`, `127`).
   - **Pragmatic Ink Approval:** If the crop includes all ink, text, and labels without severing content, approve it immediately as `verdict: "ok"`. A slight white margin or loose bounding box is completely acceptable.
   - **Single-Shot Correction:** If reclipping is genuinely needed, calculate an authoritative bounding box with a 5–10 unit safe margin in a single shot.
6. Output corrected normalized coordinates: `"box": [ymin, xmin, ymax, xmax]`.

---

## Execution Protocol

All evaluation rubrics, defect definitions, and coordinate guidelines are defined in:
👉 **[`prompt.md`](./prompt.md)**

For each assigned asset in the chunk payload:
1. **Inspect Visual Frame:** Call `view_file` on `build/01_page_layout/eval_frames/<asset_id>_eval.png`. Examine the active **RED frame** against page context and **BLUE sibling frames**.
2. **Determine Verdict:**
   - If clean and complete, assign `"verdict": "ok"`.
   - If clipped, misplaced, or encroaching on prose, assign `"verdict": "reclip_needed"` and provide `"box": [ymin, xmin, ymax, xmax]`.
3. **Completion Contract:** When all assigned assets are evaluated, terminate and return the compact JSON status contract to the Lead:
   ```json
   {
     "status": "completed",
     "stage": 10,
     "chunk_id": "<CHUNK_ID>",
     "verdicts": {
       "<asset_id_1>": { "verdict": "ok" },
       "<asset_id_2>": { "verdict": "reclip_needed", "box": [ymin, xmin, ymax, xmax] }
     }
   }
   ```
   - **CRITICAL:** Do NOT attempt to read `assets_queue.json` or query missing paths under root `assets/`. All visual evaluation is performed directly from the evaluation frames.

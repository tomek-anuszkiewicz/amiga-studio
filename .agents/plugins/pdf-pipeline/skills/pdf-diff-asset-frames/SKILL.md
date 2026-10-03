---
name: pdf-diff-asset-frames
description: >-
  Utility skill to compare modified asset review frames (page_XXXX_asset_frame.png) against git history. Distinguishes cosmetic boundary/margin shifts (identical semantic tags, identical trimmed ink content) from significant changes (reclassifications, asset count changes, or content diffs), staging insignificant frames to git on explicit user request.
---

# Asset Frame Diff & Selective Staging (`pdf-diff-asset-frames`)

This utility skill provides automated, high-precision visual and metadata diffing for Stage 15 visual review frames (`build/01_page_layout/asset_frames/page_XXXX_asset_frame.png`) against git HEAD history.

It is designed as an **on-demand utility skill** executed strictly upon explicit user instruction.

---

## Purpose & Problem Solved

When visual assets are converted, classified, or re-clipped, `stage15_render_asset_frames.py` generates full-page annotated review PNGs with red bounding boxes and semantic tags (`MD`, `HTML`, `IMG`, `ERR`).

In many cases, an asset frame is modified in git purely due to a minor margin jitter (e.g. 1–3 pixels shift on empty white paper margins) while the underlying technical table/schematic ink and the semantic badge remain 100% identical. Manually inspecting hundreds of identical frames creates cognitive overload.

This tool automatically separates:
1. **Insignificant (Cosmetic) Shifts:** Same semantic type, same asset count, and 100% identical trimmed ink content inside the bounding box. These are safely staged to git (`git add`).
2. **Significant (True) Modifications:** Semantic tag changed (e.g. `table_markdown` to `table_html`), asset count changed, or printed ink within the box changed. These remain **unstaged in the working tree** for focused human review.

## Input & Output
- **Input (Read-Only):**
  - Git working-tree modified review frames: `build/01_page_layout/asset_frames/page_XXXX_asset_frame.png`.
  - Git `HEAD` baseline versions of the corresponding asset review frames.
  - Base high-resolution page previews: `build/01_page_layout/page_XXXX.png`.
- **Output:**
  - Automated git index updates: cosmetic/insignificant frames staged to git index (`git add`).
  - Significant modifications retained unstaged in the working tree for focused human review.

---

## Decision Logic & Criteria

For each modified `page_XXXX_asset_frame.png`:

```
 modified asset_frame.png in git diff HEAD
                   │
                   ▼
       Asset count identical? ──────No────► [SIGNIFICANT: UNSTAGED]
                   │ Yes
                   ▼
     Semantic tags identical? ──────No────► [SIGNIFICANT: UNSTAGED]
     (MD / HTML / IMG badges)
                   │ Yes
                   ▼
       Crop boxes from base page
       Trim blank white margins
                   │
                   ▼
       Trimmed ink identical? ──────No────► [SIGNIFICANT: UNSTAGED]
       (diff.getbbox() == None)
                   │ Yes
                   ▼
         [INSIGNIFICANT]
       Stage to git index (`git add`)
```

---

## How to Execute

### 1. Stage Insignificant Changes Across All Manuals
Stages all cosmetic frames into git index (`git add`) across the repository, keeping significant frames unstaged:

```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-diff-asset-frames/scripts/diff_asset_frames.py
```

### 2. Stage for a Specific Manual Directory
Restrict execution to a single manual:

```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-diff-asset-frames/scripts/diff_asset_frames.py "68000 Programmer's Reference Manual"
```

---

## Command-Line Arguments

| Argument | Description | Default |
| :--- | :--- | :--- |
| `manual_dir` | Optional path to specific manual directory. If omitted, scans repository-wide. | `None` (all) |
| `--tolerance` | Pixel tolerance threshold for anti-aliasing noise during ink difference. | `15` |

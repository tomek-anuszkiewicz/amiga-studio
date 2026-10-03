---
name: pdf-stage6-prepare-queue
description: >-
  Stage 6: Prepares a JSON queue of pages to be transcribed by the inference stage. Saved in the manual's build/ directory with processing status for every page.
---

# Stage 6: Prepare Inference Queue

This skill inspects the source manual and Stage 5 layout output (`build/01_page_layout/`) to construct or refresh an atomic queue JSON (`build/<stem>_queue.json`). It tracks file-by-file transcription status (`pending` vs `completed`) so that the subsequent inference stage can process pages sequentially without guesswork or external batching.

## Purpose & Scope
- **File-by-File Queue Generation:** Identifies all pages from the PDF / Stage 5 layout and structures them into an ordered list with explicit relative paths to `page_XXXX.json`, `page_XXXX.png`, and target `page_XXXX.md`.
- **Status Tracking:** Automatically evaluates existing `page_XXXX.md` files:
  - If a valid markdown file exists, status is marked `"completed"`.
  - If markdown does not yet exist or is empty, status is marked `"pending"`.
- **Build Directory Placement:** Saves `<stem>_queue.json` directly in the manual's `build/` directory (alongside `<stem>_chapters.json`).
- **Resumable & Updatable:** Allows checking current progress (`--status`), filtering subsets (`--pages`), or updating single page status (`--update-page N --set-status completed`).

## Input & Output
- **Input (Read-Only):**
  - Source PDF (e.g. `manual_ocr.pdf` or `build/manual_ocr.pdf`)
  - Stage 5 layout directory: `build/01_page_layout/` (`page_XXXX.json`, `page_XXXX.png`)
- **Output:** `build/<manual>_queue.json` (e.g. `build/M68000PRM_ocr_queue.json`).

---

## How to Execute

### 1. Build or Refresh Queue for a Manual
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage6-prepare-queue/scripts/stage6_prepare_queue.py "path/to/manual_ocr.pdf"
```

### 2. Restrict to a Specific Page Range
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage6-prepare-queue/scripts/stage6_prepare_queue.py "path/to/manual_ocr.pdf" --pages 1-50
```

### 3. Check Queue Status & Remaining Pages
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage6-prepare-queue/scripts/stage6_prepare_queue.py "path/to/manual_ocr.pdf" --status
```

### 4. Mark a Page as Completed (used by Stage 7)
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage6-prepare-queue/scripts/stage6_prepare_queue.py "path/to/manual_ocr.pdf" --update-page 5 --set-status completed
```

---

## Structure of `build/<manual>_queue.json`

```json
{
  "pdf_file": "M68000PRM_ocr.pdf",
  "total_pages": 308,
  "layout_dir": "build/01_page_layout",
  "stats": {
    "total": 308,
    "completed": 4,
    "pending": 304
  },
  "queue": [
    {
      "page_number": 1,
      "json_file": "build/01_page_layout/page_0001.json",
      "png_file": "build/01_page_layout/page_0001.png",
      "md_file": "build/01_page_layout/page_0001.md",
      "status": "completed"
    },
    {
      "page_number": 2,
      "json_file": "build/01_page_layout/page_0002.json",
      "png_file": "build/01_page_layout/page_0002.png",
      "md_file": "build/01_page_layout/page_0002.md",
      "status": "pending"
    }
  ]
}
```

---
name: pdf-stage4-find-chapters
description: >-
  Stage 4: Scans PDF bookmarks (TOC) and in-page text layout to detect where chapters, sections, and appendices begin. Outputs a structured chapters JSON map.
---

# Stage 4: Find Chapters & Section Boundaries

This skill detects the structural boundaries of a technical manual, creating a chapter map used later to organize pages into clean chapter files.

## Purpose & Scope
- Inspects PDF bookmarks / outline for structured TOC entries.
- If bookmarks are absent, scans page text layout for section headers (`SECTION 1`, `CHAPTER 2`, `APPENDIX A`).
- Detects Front Matter and Table of Contents pages.
- Outputs `build/<stem>_chapters.json` containing the start page and authoritative title of every chapter.

## Input & Output
- **Input (Read-Only):** PDF with text layer (e.g. `manual_ocr.pdf` or `build/manual_ocr.pdf`).
- **Output:** `build/<manual>_chapters.json` (e.g. `build/manual_ocr_chapters.json`).

---

## How to Execute

### 1. Detect Chapters from Full Document
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage4-find-chapters/scripts/find_chapters.py "path/to/manual_ocr.pdf"
```

### 2. Filter by Specific Page Range
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage4-find-chapters/scripts/find_chapters.py "path/to/manual_ocr.pdf" --pages 1-50
```

### 3. Custom Output Path
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage4-find-chapters/scripts/find_chapters.py "path/to/manual_ocr.pdf" -o "build/chapters.json"
```

---

## Structure of `build/<stem>_chapters.json`
```json
{
  "pdf_file": "manual_ocr.pdf",
  "total_pages": 308,
  "has_text_layer": true,
  "detection_method": "text_layout",
  "chapter_count": 19,
  "chapters": [
    {
      "order": 1,
      "title": "Front Matter",
      "clean_title": "Front Matter",
      "filename": "01 - Front Matter.md",
      "start_page": 1,
      "type": "front_matter"
    },
    {
      "order": 2,
      "title": "Table of Contents",
      "clean_title": "Table of Contents",
      "filename": "02 - Table of Contents.md",
      "start_page": 3,
      "type": "contents"
    },
    {
      "order": 3,
      "title": "SECTION 1 - SUMMARY OF DIFFERENCES",
      "clean_title": "Section 1 - Summary of Differences",
      "filename": "03 - Section 1 - Summary of Differences.md",
      "start_page": 4,
      "type": "section"
    },
    {
      "order": 4,
      "title": "SECTION 2 - SYSTEM BLOCK DIAGRAMS",
      "clean_title": "Section 2 - System Block Diagrams",
      "filename": "04 - Section 2 - System Block Diagrams.md",
      "start_page": 16,
      "type": "section"
    }
  ]
}
```

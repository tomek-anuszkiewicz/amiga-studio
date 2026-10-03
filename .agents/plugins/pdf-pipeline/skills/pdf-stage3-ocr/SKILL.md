---
name: pdf-stage3-ocr
description: >-
  Stage 3: Checks if a PDF manual has an extractable text layer. If missing or scanned bitmap, injects a searchable OCR layer with PyMuPDF + Tesseract without degrading vector graphics or schematics.
---

# Stage 3: Add Searchable OCR Text Layer

This skill verifies whether a PDF has an embedded text layer and, if necessary, adds an accurate, invisible OCR text layer line-by-line using PyMuPDF and Tesseract.

## Purpose & Scope
- **Check Text Layer:** Detects if the PDF is already text-searchable or pure scanned bitmap.
- **Pipeline Consistency Invariant:** Stage 3 **ALWAYS** ensures that `build/<stem>_ocr.pdf` exists. If the PDF already has a valid text layer, it skips OCR and automatically copies the file to `build/<stem>_ocr.pdf`, so downstream stages always have a uniform input filename.
- **Line-level Injection:** Injects OCR text directly into the PDF coordinate system, allowing continuous sentence selection across lines.
- **Scanner Margin Noise Filter:** Filters out scanner edge shadows, binding line artifacts, and crop ticks outside the content margins.
- **Schematic Protection:** Detects large format foldouts (schematics) and skips OCR to prevent noisy characters from cluttering circuit diagrams while preserving high-resolution graphics.
- **Non-destructive:** Leaves the original PDF untouched, outputting `build/<name>_ocr.pdf`.

## Input & Output
- **Input (Read-Only):** Source PDF located in `build/` (e.g. `build/<name>.pdf` or `path/to/manual/build/<name>.pdf`).
- **Output:** `build/<name>_ocr.pdf` (e.g. `build/manual_ocr.pdf`).

---

## How to Execute

### 1. Check If PDF Already Has a Text Layer
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage3-ocr/scripts/add_ocr_layer.py "path/to/manual/build/manual.pdf" --check-only
```

### 2. Run Full OCR on the Document
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage3-ocr/scripts/add_ocr_layer.py "path/to/manual/build/manual.pdf"
```

### 3. Test on a Sample Range of Pages
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage3-ocr/scripts/add_ocr_layer.py "path/to/manual/build/manual.pdf" --pages 1-10
```

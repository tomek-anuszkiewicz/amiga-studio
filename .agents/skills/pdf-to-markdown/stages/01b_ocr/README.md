# Stage 01b: Scan Detection & Gemini Vision OCR

## Role & Architecture
Stage `01b_ocr` acts as an intelligent, conditional bridge between **Stage 01** (`01_preprocess`) and **Stage 02** (`02_page_segmentation`).

In the pipeline:
1. Stage `01_preprocess` runs first, extracting per-page vector PDFs, 300 DPI PNGs, and any native text blocks present in the PDF via PyMuPDF.
2. Stage `01b_ocr` inspects the resulting `page_XXXX.json` files:
   - **Born-Digital Pages:** If a page contains healthy text blocks (`total_chars >= threshold`, default 20), Stage 01b executes an immediate pass-through (0 API calls, 0.0s).
   - **Scanned Pages / Covers:** If a page contains 0 blocks or empty text, Stage 01b automatically activates **Gemini Vision OCR** on `page_XXXX.png` using a fused single-pass triage prompt:
     - **`text_page`**: Scanned text, headings, paragraphs, and tables are extracted into normalized bounding boxes (`bbox_norm`) and text blocks.
     - **`pure_graphic`**: Full-page illustrations, schematics, and photos are classified as visual assets without hallucinating garbage text on circuit traces or artwork.
     - **`blank`**: Blank separator pages are marked with 0 text blocks.
3. It updates `page_XXXX.json` in place, satisfying the exact data contract expected by Stage 02.

## CLI Usage
```powershell
python stages/01b_ocr/detect_and_ocr.py --workspace <WORKSPACE_DIR> [--force] [--threshold 20]
```

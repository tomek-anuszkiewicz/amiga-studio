# Stage 01b: Scan Detection & Gemini Vision OCR

## Objective
Intelligent inspection step between Stage 01 and Stage 02:
- **Born-Digital Pages:** Executes immediate pass-through (0 API calls) if healthy text blocks are present (`total_chars >= threshold`, default 20).
- **Scanned Pages / Covers:** Activates Gemini Vision OCR on `page_XXXX.png` using a fused single-pass triage prompt (`text_page`, `pure_graphic`, `blank`).
- Enriches `page_XXXX.json` in place with integer millirange `box_2d` and normalized `bbox_norm` bounding boxes matching the Stage 02 contract.

## Inputs
- `workspace/01_preprocess/page_XXXX.json`: Text blocks and page dimensions from Stage 01.
- `workspace/01_preprocess/page_XXXX.png`: 300 DPI raster page render (used if OCR is triggered).
- `stages/01b_ocr/prompt_ocr.md`: Fused triage and OCR vision prompt.

## Outputs
- `workspace/01_preprocess/page_XXXX.json`: In-place enriched text blocks with normalized `box_2d` and `bbox_norm` bounding boxes and `page_type`.

## Standalone Invocation
```powershell
python stages/01b_ocr/detect_and_ocr.py --workspace "workspace" [--force] [--threshold 20]
```

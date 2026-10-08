# Stage 01: Shared PDF Text Layer

Prepare `<source stem>-ocr.pdf` beside the original PDF, preserving every source
page in its original order. This stage ignores the downstream page selection.
An existing sibling OCR PDF is reused by file existence; remove it explicitly
to request new preparation. The original PDF remains unchanged.

Retain native text. For textless pages, run local Tesseract through PyMuPDF at
300 DPI with English (`eng`) language data. PyMuPDF detects that data or uses
`TESSDATA_PREFIX`. Overlay the OCR text while retaining the original page graphics.
No model request is made.

Per-page OCR results are not cached or saved separately. Interrupted preparation
starts again from the source PDF. The completed PDF is saved beside the source
and used by [Stage 02](../02_preprocess/README.md). OCR can miss text; inspect the result.

Run through the [pipeline](../../README.md) with `--from-stage 01 --to-stage 01`,
or through 02 to also produce page images and positioned text.

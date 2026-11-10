# Stage 01: Shared PDF Text Layer

Prepare `<source stem>-ocr.pdf` beside the original PDF, preserving every source
page in its original order. This stage ignores the downstream page selection.
The [preparation cache](../../README.md#preparation-cache) restores a matching
completed PDF and provenance record before any OCR. Output existence alone does
not skip preparation. The original PDF remains unchanged.

Retain native text. For textless pages, run local Tesseract through PyMuPDF.
The [pipeline configuration](../../config.yaml) stores shared `rendering.dpi`
(default 300) and the URL and SHA-256 of the exact language model under
`stages.01_text_layer`. These configurable values use repository defaults when
omitted, without rewriting workspace configuration. OCR uses fixed English
(`eng`), RGB without alpha, compressed OCR output, and PDF save settings
`deflate=True, garbage=4`, defined in the preparation code and recorded in
provenance. PyMuPDF detects the data directory or uses `TESSDATA_PREFIX`;
before new OCR the model bytes must match the configured hash. Overlay the OCR
text while retaining the original page graphics. No model request is made.

Install the pinned [Python requirements](../../requirements.txt); library
versions are maintained there rather than checked by this stage. Download the
language `.traineddata` file from `traineddata_url` into a local tessdata
directory and set `TESSDATA_PREFIX` to that directory, or use an existing
installation containing the same bytes. The program does not download or replace
language data. A different model hash stops new OCR. PyMuPDF uses MuPDF's
integrated Tesseract backend. Configuration changes require deliberate review of
recipes authored against the previous settings.

New preparation writes `<source stem>-ocr.json` beside the prepared PDF with
the effective settings, actual PyMuPDF/MuPDF versions, original/prepared
SHA-256, pages on which OCR was attempted and Python/platform identity. This is provenance, not transcription validation.
Cache hits restore the original preparation record without relabeling its
provenance. They do not need local language data; the cache key includes the
configured model hash, which is checked against actual bytes when OCR runs.
Package version pins do not guarantee byte-identical output across platforms or
custom builds; the cache also distinguishes runtime identities.

Per-page OCR results are not cached or saved separately. Interrupted preparation
does not create a completed cache entry and starts again from the source PDF.
The completed PDF is saved beside the source
and used by [Stage 02](../02_preprocess/README.md). OCR can miss text; inspect the result.

Run through the [pipeline](../../README.md) with `--from-stage 01 --to-stage 01`,
or through 02 to also produce page images and positioned text.

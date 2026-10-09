# Stage 01: Shared PDF Text Layer

Prepare `<source stem>-ocr.pdf` beside the original PDF, preserving every source
page in its original order. This stage ignores the downstream page selection.
An existing sibling OCR PDF is reused by file existence; remove it explicitly
to request new preparation. The original PDF remains unchanged.

Retain native text. For textless pages, run local Tesseract through PyMuPDF.
The [pipeline configuration](../../config.yaml) stores shared `rendering.dpi`
(default 300) and OCR settings under `stages.01_text_layer`: language, the URL
and SHA-256 of the exact language model, RGB/alpha, compression and PDF save
settings. Existing workspace configurations override these defaults without
being rewritten. PyMuPDF detects the data directory or uses `TESSDATA_PREFIX`;
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
Existing prepared PDFs still skip all preparation and model checks; a missing
record remains unknown provenance. A failure to write the record after publishing
the PDF can also leave a PDF without provenance. No metadata is retroactively
invented for it. Package version pins do not guarantee byte-identical output
across platforms or custom builds.

Per-page OCR results are not cached or saved separately. Interrupted preparation
starts again from the source PDF. The completed PDF is saved beside the source
and used by [Stage 02](../02_preprocess/README.md). OCR can miss text; inspect the result.

Run through the [pipeline](../../README.md) with `--from-stage 01 --to-stage 01`,
or through 02 to also produce page images and positioned text.

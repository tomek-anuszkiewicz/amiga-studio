# Stage 01: Shared PDF Text Layer

Stage 01 prepares `<source stem>-ocr.pdf` beside the original PDF and leaves the
original PDF unchanged. The prepared PDF preserves every source page in order,
regardless of which pages later stages use. The stage can reuse an earlier
result only if the [preparation cache](../../README.md#preparation-cache) has
a matching completed PDF and provenance record. The presence of an output file
alone cannot establish a cache match.

The stage retains native text and runs local Tesseract through PyMuPDF on
textless pages.
The [pipeline configuration](../../config.yaml) stores shared `rendering.dpi`
(default 300) and the URL and SHA-256 of the exact language model under
`stages.01_text_layer`. The program uses repository defaults for omitted values
and leaves the workspace configuration as written. OCR uses fixed English
(`eng`), RGB without alpha, compressed OCR output, and PDF save settings
`deflate=True, garbage=4`, defined in the preparation code and recorded in
provenance. PyMuPDF detects the data directory or uses `TESSDATA_PREFIX`. Before
new OCR, the model bytes must match the configured hash. The stage overlays the
OCR text on the original page graphics and runs without a model request.

Install the pinned [Python requirements](../../requirements.txt), which specify
the library versions. The stage does not check those versions. Download the
language `.traineddata` file from `traineddata_url` into a local tessdata
directory and set `TESSDATA_PREFIX` to that directory, or use an existing
installation containing the same bytes. The program does not download or replace
language data. A different model hash stops new OCR. PyMuPDF uses MuPDF's
integrated Tesseract backend. Configuration changes require deliberate review of
recipes authored against the previous settings.

When the stage prepares a PDF, it writes `<source stem>-ocr.json` beside it.
The record describes how the stage made the PDF. It includes the settings used,
the actual PyMuPDF/MuPDF versions, SHA-256 hashes of the original and prepared
PDFs, the pages where the stage attempted OCR, and the Python and platform
identity. The record alone cannot establish transcription accuracy. A cache
hit restores the original record and requires no local language data. The cache key includes the
configured model hash, which the stage checks against the actual bytes when
OCR runs. The cache also distinguishes runtime identities because output can
differ across platforms or custom builds even with the same package versions.

The stage saves and caches OCR results only as part of the full PDF. An
interruption leaves no completed cache entry, so the next attempt starts again
from the source PDF. The stage saves the completed PDF beside the source for
[Stage 02](../02_preprocess/README.md). Inspect the output because OCR can miss
text.

Run through the [pipeline](../../README.md) with `--from-stage 01 --to-stage 01`,
or through 02 to also produce page images and positioned text.

# Stage 02: Positioned Text and PNGs

Stage 02 reads the prepared [Stage 01 PDF](../01_text_layer/README.md) and writes one
`02_preprocess/page_XXXX.png` and `page_XXXX.json` for each selected physical page,
including blank pages. It renders with shared `rendering.dpi` from the
[pipeline configuration](../../config.yaml), which defaults to 300 DPI. It
runs without OCR or model requests.

Before rendering or extracting text, Stage 02 tries to restore each selected
page's PNG and JSON from the [preparation cache](../../README.md#preparation-cache).
It renders and extracts text for each missing pair and caches it independently.
An interrupted run retains the pages already in the cache.

The JSON records the source page number, page geometry, actual PNG dimensions,
extracted text blocks and the transform between text and raster coordinates.
Text boxes use PDF points relative to the displayed page. Later object boxes
use pixels in the original PNG. The stage extracts text from the prepared PDF
and records its provenance as `prepared` or `none`. Each block also stores
`char_bboxes` with one rectangle in displayed-page coordinates per Unicode
character in `text`. The rectangle for an extracted newline comes from the
preceding line's last character, so selecting a region preserves line breaks.
Adding character geometry leaves the extracted block IDs and text identical.

Run through the [pipeline](../../README.md) with `--from-stage 02 --to-stage 02`
after successful Stage 01, supplying the original `--pdf` and intended
`--page-ranges`. Omitting the range selects all pages.

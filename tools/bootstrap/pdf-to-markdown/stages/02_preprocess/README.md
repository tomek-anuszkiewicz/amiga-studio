# Stage 02: Positioned Text and PNGs

Read the prepared [Stage 01 PDF](../01_text_layer/README.md) and write one
`02_preprocess/page_XXXX.png` and `page_XXXX.json` for each selected physical page,
including blank pages. Render using shared `rendering.dpi` from the
[pipeline configuration](../../config.yaml), default 300 DPI; no OCR or model
request is made.

JSON records the source page number, page geometry, actual PNG dimensions,
extracted text blocks and the transform between text and raster coordinates.
Text boxes use PDF points in the displayed-page frame; later object boxes use
original-PNG pixels. Text is extracted from the prepared PDF, with provenance
`prepared` or `none`. Block geometry is not word-level geometry.

Run through the [pipeline](../../README.md) with `--from-stage 02 --to-stage 02`
after successful Stage 01, supplying the original `--pdf` and intended
`--page-ranges`. Omitting the range selects all pages.

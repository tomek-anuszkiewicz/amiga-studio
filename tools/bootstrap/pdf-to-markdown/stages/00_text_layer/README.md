# Stage 00: PDF Text Layer

Stage 00 uses local Tesseract through PyMuPDF to publish
`00_text_layer/<source stem>-ocr.pdf` and `text_layer_manifest.json`.
Stage 01 consumes this prepared PDF exclusively. The source stays outside the
workspace and is never overwritten. Stage 00 does not call Codex; later inference
stages keep their configured models.

## OCR configuration

Tesseract language data must be installed locally. Configure the pipeline:

```yaml
ocr:
  language: "eng"
  tessdata: null
```

`language` accepts Tesseract identifiers, including combinations such as
`eng+pol`. `tessdata` can point to a local directory containing the selected
`.traineddata` files. When null, PyMuPDF uses `TESSDATA_PREFIX` or detects the
installed Tesseract data directory. Keep host-specific overrides in local
configuration, outside committed files. Missing language data or an OCR execution
error stops preparation; there is no model fallback.

Existing native spans are retained even below 20 characters. For selected
textless pages, the worker renders RGB at `render.dpi`, runs
Tesseract on that displayed-page image and retains its original OCR PDF. Its text
operators and font resources are overlaid onto the source page at their original
scale. No fixed margin filter or size-based schematic exclusion is applied.

`classification_basis: inferred_from_tesseract_lines` records extraction evidence.
Recognized lines use `text_page`; no recognized lines use the legacy
`pure_graphic` value with `provenance: none`. This does not certify that the image
contains no text: Tesseract can miss text and does not perform semantic page
classification. Native classification remains inferred from extracted spans.

## Text insertion and publication

Stage 00 copies the original text layer from Tesseract's PDF, including its font,
glyph positions, horizontal spacing and invisible-text operators. It does not
normalize or round coordinates, clamp OCR boxes, join lines, substitute text or
fit reconstructed strings to bounding boxes. Only the OCR raster is replaced
with a transparent image because the source page already owns its graphics.
The overlay maps the displayed-page frame to the source page's rotation without
resizing the text. Stage 00 trusts the PDF writer and does not compare reopened
OCR text with recognition results. The user assesses the PDF; Stage 01 extracts
its actual text. Native text remains unchanged.

The separate PDF contains only selected source pages, in source order, retaining
their existing text and graphics and adding OCR only where text is missing.
Stage 00 saves the PDF and its manifest without reopening or validating the
output. It does not compare renders, geometry, source streams or extracted text,
and does not reread the source to check it after saving.

Fragment preparation excludes unselected pages from the output and processing.
`page` and zero-based `source_index` identify the original source;
zero-based `prepared_index` locates the page in the compact prepared PDF.
Expanding selection requires restart
at 00 with the original source or another workspace.

## Recovery and downstream handoff

Stage 00 publishes the prepared PDF and its preparation manifest. Stage 01
extracts per-page positioned text JSON from that PDF; Stage 00 does not write
separate native/OCR inspection JSON. Recovery holds the unmodified per-page OCR
PDF plus JSON identity metadata. Recovery reuse matches source, selection, image bytes, geometry, format,
procedure, language-data hashes and PyMuPDF version; it performs no additional
artifact integrity validation. OCR PDFs are opened to insert their text. Legacy normalized
JSON and Codex recovery records are incompatible and are regenerated.

The manifest records the prepared PDF path, coverage, page geometry,
provenance, inferred classification evidence and source-to-prepared page mapping.

Run through the pipeline, for example:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<source.pdf>" --workspace "<workspace>" --from-stage 00 --to-stage 01
```

`--run-deterministic` can execute Stage 00 when it is the next ready stage.
Restart at 01 retains completed preparation. Existing workspaces using the old
backend, output name or full-document page layout require explicit restart at 00
with the original PDF.

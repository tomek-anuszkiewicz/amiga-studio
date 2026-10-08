# Stage 01: Shared PDF Text Layer

Stage 01 prepares `<source stem>-ocr.pdf` beside the original source PDF.
It always includes every source page in its original order, regardless of
`--page-ranges`. Stage 02 reads this shared PDF and extracts only the selected
pages. The original source is never overwritten. Stage 01 uses local Tesseract
through PyMuPDF and does not call a model.

If the sibling `*-ocr.pdf` already exists, Stage 01 skips page processing,
Tesseract setup and PDF writing. The orchestrator records successful execution in `stage_status.json`.
File existence controls reuse; changes to the source, language data or
procedure do not automatically regenerate an existing PDF. To regenerate it,
remove the sibling prepared PDF explicitly and restart at 01 with the source.
Restart cleanup affects workspace outputs, leaving this shared PDF intact.

## OCR language data

OCR uses fixed English (`eng`) language data. PyMuPDF uses `TESSDATA_PREFIX`
or detects the installed directory; the YAML contains no OCR overrides.
The language constant is private to Stage 01. Recovery identity includes the
actual language-data hash and fixed rendering settings.
Missing language data or OCR errors stop new preparation without a model fallback.
Existing prepared PDFs need no Tesseract setup.

## Text insertion and publication

On first preparation, existing native spans are retained regardless of length.
Every textless page is rendered in RGB at the fixed 300 DPI and passed to Tesseract.
The original OCR PDF's text operators and font resources are overlaid at their
original scale. The OCR raster becomes transparent because the source page
already owns its graphics. Rotation maps the displayed-page frame without
resizing text. No margin filter, coordinate normalization, text reconstruction
or size-based schematic exclusion is applied.

Stage 01 saves a candidate beside the source and publishes it by rename after
all pages finish. It does not reopen the result, compare renders or validate
text geometry. Tesseract can miss text; no recognized text does not establish
that a page is semantically graphic-only. The user assesses the PDF.

## Recovery and downstream handoff

Workspace-local recovery retains the original per-page OCR PDFs and their
request identities. Compatible records can resume an interrupted preparation.
Identity includes source selection, image bytes, geometry, format, procedure,
language-data hashes and PyMuPDF version. Recovery does not perform additional
artifact integrity checks.

The source path and requested physical pages live in the attempt's `config.yaml`.
The sibling PDF path is derived from that source. Both `source_index` and
`prepared_index` equal the physical page number minus one. `stage_status.json`
records execution status and metrics; Stage 01 publishes no manifest or snapshot.

Page JSON records extractable text provenance as `prepared` (or `none` when no
text is extracted). An existing PDF alone cannot establish whether its text was
native or OCR, so Stage 02 does not invent that distinction.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<source.pdf>" --workspace "<workspace>" --config tools/bootstrap/pdf-to-markdown/config.yaml --page-ranges "1-5" --from-stage 01 --to-stage 02
```

Restart at 02 retains the shared preparation and needs no source path.
Legacy workspace-local compact PDFs are not copied into the shared location;
restart at 01 with the original source to prepare or reuse the full PDF.

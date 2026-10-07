# Stage 00: Shared PDF Text Layer

Stage 00 prepares `<source stem>-ocr.pdf` beside the original source PDF.
It always includes every source page in its original order, regardless of
`--page-ranges`. Stage 01 reads this shared PDF and extracts only the selected
pages. The original source is never overwritten. Stage 00 uses local Tesseract
through PyMuPDF and does not call a model.

If the sibling `*-ocr.pdf` already exists, Stage 00 skips page processing,
Tesseract setup and PDF writing. It only registers the shared file in the
attempt's `00_text_layer/text_layer_manifest.json` and completion record.
File existence controls reuse; changes to the source, OCR configuration or
procedure do not automatically regenerate an existing PDF. To regenerate it,
remove the sibling prepared PDF explicitly and restart at 00 with the source.
Restart cleanup affects workspace outputs, leaving this shared PDF intact.

## OCR configuration

Configure locally installed Tesseract language data:

```yaml
ocr:
  language: "eng"
  tessdata: null
```

`language` accepts Tesseract identifiers, including combinations such as
`eng+pol`. `tessdata` can point to a directory containing the selected
`.traineddata` files. When null, PyMuPDF uses `TESSDATA_PREFIX` or detects the
installed directory. Keep host-specific overrides outside committed files.
Missing language data or OCR errors stop new preparation without a model fallback.
Existing prepared PDFs need no Tesseract setup.

## Text insertion and publication

On first preparation, existing native spans are retained regardless of length.
Every textless page is rendered in RGB at `render.dpi` and passed to Tesseract.
The original OCR PDF's text operators and font resources are overlaid at their
original scale. The OCR raster becomes transparent because the source page
already owns its graphics. Rotation maps the displayed-page frame without
resizing text. No margin filter, coordinate normalization, text reconstruction
or size-based schematic exclusion is applied.

Stage 00 saves a candidate beside the source and publishes it by rename after
all pages finish. It does not reopen the result, compare renders or validate
text geometry. Tesseract can miss text; no recognized text does not establish
that a page is semantically graphic-only. The user assesses the PDF.

## Recovery and downstream handoff

Workspace-local recovery retains the original per-page OCR PDFs and their
request identities. Compatible records can resume an interrupted preparation.
Identity includes source selection, image bytes, geometry, format, procedure,
language-data hashes and PyMuPDF version. Recovery does not perform additional
artifact integrity checks.

The local manifest records the source-relative shared PDF path, full source
coverage and page mapping. Both `source_index` and `prepared_index` equal the
physical page number minus one, so the same page keeps the same index in every
attempt. Completion records hash the external shared PDF as well as local
artifacts; later stages reject a modified retained PDF. Paths remain relative.

Page JSON records extractable text provenance as `prepared` (or `none` when no
text is extracted). An existing PDF alone cannot establish whether its text was
native or OCR, so Stage 01 does not invent that distinction.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<source.pdf>" --workspace "<workspace>" --config tools/bootstrap/pdf-to-markdown/config.yaml --page-ranges "1-5" --from-stage 00 --to-stage 01
```

Restart at 01 retains the shared preparation and needs no source path.
Legacy workspace-local compact PDFs are not copied into the shared location;
restart at 00 with the original source to register or prepare the full PDF.

# Stage 01: Positioned Text and PNGs

Stage 01 deterministically renders and extracts text from the separate, prepared
[Stage 00 PDF](../00_text_layer/README.md). It performs no OCR and needs no model
selection. PDF-library and filesystem failures stop execution; preprocessing never falls back to the original PDF.

For each selected physical page it writes `01_preprocess/page_XXXX.png` and
`page_XXXX.json`, including legitimate blank pages. `XXXX` remains the physical
1-based source page number; `source_index` is zero-based in the original source.
`prepared_index` equals `source_index`: the sibling prepared PDF contains every
source page in the original order. Stage 01 applies the current `--page-ranges`;
page workers through 02.82 also apply it when reading their own predecessors.
JSON contains schema
version, stable page ID, dimensions, rotation, MediaBox,
CropBox, `prepared`/`none` text provenance, extraction-based page classification
and ordered text blocks with
IDs, text, `bbox` and `bbox_norm`. Only text blocks enter this collection; the
worker does not claim word-level geometry or reconstruct semantic reading order.
OCR text is extracted from the reopened PDF, never copied from cached model JSON.

Bounding boxes use PDF points, top-left origin, x right/y down, `[x0,y0,x1,y1]`
in the displayed-page frame. `geometry.extraction_to_display` records rotation
from PyMuPDF's crop-relative, unrotated extraction frame; `pdf_to_extraction`
records its PDF transformation. `raster.display_to_pixels` records the actual
render transform, raster origin and outward floor/ceil pixel bounds. Actual PNG
dimensions are persisted; exported coordinates are not rounded. Existing extraction clipping is retained
without rectangle bounds, structure or text-content validation.

Stage 01 publishes only page PNG/JSON files. Downstream stages enumerate known
filenames in numeric physical-page order and read them directly. No coverage,
pair-presence, identity, geometry, dimensions or PDF-text comparison checks run.
Execution status and metrics belong to `stage_status.json`.

Run preparation and preprocessing together for a new workspace:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config tools/bootstrap/pdf-to-markdown/config.yaml --page-ranges "1-5,7" --from-stage 00 --to-stage 01
```

For an attempt with successful Stage 00, restart with `--from-stage 01 --to-stage 01`.
Omitting the source argument retains its configured path. Omitting `--page-ranges`
selects all pages; an explicit selection applies to this run only and can reuse
the full-source sibling PDF without repeating OCR. Legacy `input.pages` is ignored.
Legacy attempts use the separate migration utility described in the [converter README](../../README.md).

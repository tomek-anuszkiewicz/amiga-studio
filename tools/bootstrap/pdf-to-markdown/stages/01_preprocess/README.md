# Stage 01: Positioned Text and PNGs

Stage 01 deterministically renders and extracts text from the separate, prepared
[Stage 00 PDF](../00_text_layer/README.md). It performs no OCR and needs no model
selection. Missing, incomplete, modified or incompatible Stage 00 artifacts stop
execution; preprocessing never falls back to the original PDF.

For each selected physical page it writes `01_preprocess/page_XXXX.png` and
`page_XXXX.json`, including legitimate blank pages. `XXXX` remains the physical
1-based source page number; `source_index` is zero-based in the original source.
`prepared_index` equals `source_index`: the sibling prepared PDF contains every
source page in the original order. Only Stage 01 applies `--page-ranges`.
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
dimensions are persisted; exported coordinates are not rounded. Nonfinite,
inverted or materially out-of-page boxes fail validation.

`pages_manifest.json` records document versus selected page counts, exact
selection, shared prepared PDF path and relative PNG/JSON paths with hashes.
It is published only when every required pair passes coverage, content, geometry
and image-dimension validation. Completion snapshots it through shared lineage;
downstream conversion validates pairs before cleanup or requests.

Run preparation and preprocessing together for a new workspace:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config tools/bootstrap/pdf-to-markdown/config.yaml --page-ranges "1-5,7" --from-stage 00 --to-stage 01
```

For a workspace with a completed Stage 00, restart with `--from-stage 01
--to-stage 01`, or run `--run-deterministic` when 01 is the next ready stage.
Stage 00 and the original source are retained. A different attempt selection
requires restart at 00 or a new workspace; Stage 00 reuses the existing sibling
PDF without repeating OCR. Legacy workspaces require restart at 00 with the
source PDF to register the shared full-document preparation.

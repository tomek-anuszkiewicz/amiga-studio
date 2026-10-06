# Stage 00: PDF Text Layer

Stage 00 uses local Tesseract through PyMuPDF to publish
`00_text_layer/<source stem>-ocr.pdf` and `text_layer_manifest.json`.
Stage 01 consumes this validated PDF exclusively. The source stays outside the
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

Existing native spans are retained even below 20 characters. Invalid native
text or geometry stops preparation rather than overwriting a partial layer.
For selected textless pages, the worker renders RGB at `render.dpi`, runs
Tesseract on that displayed-page image and extracts complete OCR lines. Each line
becomes a normalized text block passed to the shared invisible-text insertion
mechanism. No fixed margin filter or size-based schematic exclusion is applied.

`classification_basis: inferred_from_tesseract_lines` records extraction evidence.
Recognized lines use `text_page`; no recognized lines use the legacy
`pure_graphic` value with `provenance: none`. This does not certify that the image
contains no text: Tesseract can miss text and does not perform semantic page
classification. Native classification remains inferred from extracted spans.

## Text insertion and publication

OCR line coordinates are clamped to the displayed page bounds before conversion
to the integer 0-1000 schema. Zero width and height are accepted, including extents
collapsed by clipping or rounding. Reversed corners and nonfinite coordinates
still fail. The stage does not replace unreadable markers or omit marker-only
blocks. Invalid OCR input text stops preparation. Zero extents are passed to the
PDF writer without checking whether they remain extractable afterward.

The worker adds invisible lines (`render_mode=3`) using embedded Unicode fonts.
Each line is fitted to its recognized box using serialized font metrics, mapped
from displayed-page coordinates through rotation into the PDF frame. Stage 00
trusts PyMuPDF to store the inserted text; it does not compare reopened OCR line
counts, text or positions with the recognition result. The user assesses the PDF,
and Stage 01 consumes its actual extractable text. Unsupported characters fail
instead of being silently replaced. Native text remains unchanged.

If no text is added, the separate PDF is a byte-for-byte copy. Otherwise a
candidate is saved and reopened. Validation checks the complete page tree and
geometry, retained original streams in order, unchanged native content, identical
selected-page renders and the original source hash. Publication happens only
after those checks; failure leaves no consumable manifest.

Fragment preparation retains all physical pages but certifies only selected
pages. Unselected pages remain unchanged. Expanding selection requires restart
at 00 with the original source or another workspace.

## Recovery and downstream handoff

Stage 00 publishes the prepared PDF and its validation manifest. Stage 01
extracts per-page positioned text JSON from that PDF; Stage 00 does not write
separate native/OCR inspection JSON. Invalid OCR responses never become recovery
records or a consumable PDF. Recovery reuse validates source, selection, image
bytes, geometry, schema, procedure, language-data hashes and
PyMuPDF version. Old Codex recovery records are incompatible and are regenerated.

The manifest records the prepared PDF path/hash, coverage, page geometry,
provenance, inferred classification evidence, inserted line positions, extracted
text digests and publication checks.

Run through the pipeline, for example:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<source.pdf>" --workspace "<workspace>" --from-stage 00 --to-stage 01
```

`--run-deterministic` can execute Stage 00 when it is the next ready stage.
Restart at 01 retains validated preparation. Existing workspaces using the old
backend or output name require explicit restart at 00 with the original PDF.

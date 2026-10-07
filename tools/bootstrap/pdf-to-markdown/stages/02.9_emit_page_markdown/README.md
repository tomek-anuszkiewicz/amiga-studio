# Stage 02.9: Page Markdown and Crop Assets

Emit one `02.9_emit_page_markdown/document.md` for retained physical pages,
including disjoint ranges in source order. Its sibling `assets/` directory always
exists. Inputs are Stage 02.8 filtered object JSON and the exact original-resolution
Stage 01 PNGs; review PNGs, OCR blocks and source PDFs do not supply crop content.

Text objects contribute their decoded `md_text` unchanged, with blank lines between
objects. Internal whitespace, escapes, code, math, captions, footnotes
and source-visible TOC text remain in their Stage 02 array order. No text correction,
TOC regeneration, further filtering, continuation inference or cross-page merging occurs.
Stage 02.8 has already removed headers/footers, pre-TOC objects when a boundary
exists, and entire list/index pages. Removed objects produce no text or crop assets.
An empty filtered input still emits document metadata and an empty assets directory.
Minimal YAML frontmatter and one document heading use the source filename stem;
upstream source identity currently provides no publication title or other metadata.

Each `table`, `graphic` and `cover` becomes `<segment_id>.png`, using its exact
integer `[x0, y0, x1, y1]` pixel rectangle: top-left origin, exclusive upper bounds,
no padding or rescaling. A neutral relative image link occupies that object's
position, such as `![Table page_0005_seg_003](assets/page_0005_seg_003.png)`.
Multi-page objects remain separate crops; existing continuation text is preserved.
Tables and illustrations remain raster assets; reconstruction and descriptions
belong to later work. No model requests are made and no inference configuration
entry is needed.

Run through the [orchestrator](../../README.md), including a standalone interval:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.9 --to-stage 02.9
```

The worker writes crops and Markdown into a temporary bundle and replaces prior
output only after the writes finish. Replacement removes stale assets; filesystem
and image-library failures stop with page/object context. There is no object,
Markdown, asset-inventory, PNG-format or dimension validation. Execution requires
successful Stage 01/02.8 statuses and reads their files directly, without falling
back to Stage 02. The source filename
comes from the attempt configuration. There are no per-page Markdown files or metadata sidecars.

Restart 01, 02, 02.5 or 02.8 clears this bundle and all later stages. Restart 02.9
clears this bundle and all later stages, even for a single-stage interval.
Stage 03 also consumes Stage 02.8; cleanup follows execution order. Existing
02.9/03 successes require regeneration starting at 02.8 to reflect filtering.
Stage 14 publication remains separate; `--publish` does not export this bundle.
Successful execution does not establish source fidelity; the user assesses the
selected fragment and its reading order, text and crop boundaries.

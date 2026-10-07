# Stage 02.9: Page Markdown and Crop Assets

Emit one `02.9_emit_page_markdown/document.md` for every selected physical page,
including disjoint ranges in source order. Its sibling `assets/` directory always
exists. Inputs are validated Stage 02 object JSON and the exact original-resolution
Stage 01 PNGs; review PNGs, OCR blocks and source PDFs do not supply crop content.

Text objects contribute their decoded `md_text` unchanged, with blank lines between
objects. Internal whitespace, escapes, code, math, captions, footnotes, page furniture
and source-visible TOC text remain in their Stage 02 array order. No text correction,
TOC regeneration, filtering, continuation inference or cross-page merging occurs.
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

The worker validates all selected objects before writing a temporary bundle. It
checks ordered document content, exact asset coverage, PNG readability/dimensions
and every generated asset link before replacing the output. Replacement removes
stale assets; failures stop with page/object context. Completion is recorded only
after runtime validation. Execution requires completed Stage 01/02 and their files.
There are no per-page Markdown files or metadata sidecars.

Restart 01, 02 or 02.5 clears this bundle and all later stages. Restart 02.9
clears this bundle and all later stages, even for a single-stage interval.
Stage 03 still consumes Stage 02 directly; cleanup follows execution order.
Stage 14 publication remains separate; `--publish` does not export this bundle.
Technical validation does not establish source fidelity; the user assesses the
selected fragment and its reading order, text and crop boundaries.

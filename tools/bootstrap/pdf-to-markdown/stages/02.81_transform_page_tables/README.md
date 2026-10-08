# Stage 02.81: Transform Page Tables

Read completed Stage 02.80 page JSONs and Stage 01 original PNGs. For each `table`,
make one Codex request using its exact unpadded pixel crop. Replace only the
table's `md_text` with the returned Markdown or HTML and record `table_format`.
Preserve identity, geometry, classification, continuation, object order and all
other source objects, including separate captions, notes and legends.

Save a full page JSON only when a table transformation changes that page's data.
Pages without tables and unchanged table pages remain solely in Stage 02.80.
The orchestrator records this invocation's page range in the 02.81 status entry;
Stage 02.90 overlays the saved replacements on that selected filtered dataset.
An empty output directory is valid and does not remove unchanged pages from export.

The [prompt](prompt.md) uses Markdown where possible and HTML with `rowspan` and
`colspan` for merged cells. An `unconverted` result preserves the original content
and raster fallback. Transport, authentication and JSON parsing failures stop the stage.
The configured model/effort, original image detail, ChatGPT authentication and
shared request cache remain in use.

Crops are temporary request inputs and are removed after execution. No persistent
source crops, group assignments, descriptions, RAG text or `<details>` companions
are generated. Retired table companion fields are omitted from converted output.
Stage 02.90 emits the saved table markup directly, without a companion image.
Description and grouping work belongs to separately requested book processing.

Stage 02.80 is the fixed predecessor; missing filtered pages never fall back to
Stage 02. Restart through the orchestrator clears this stage and later outputs,
preserving predecessors and request cache.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.81 --to-stage 02.81
```

Use a named attempt and only the fragment selected by the user. Existing saved
outputs require explicit regeneration to adopt this change. See
[page export](../02.90_emit_page_markdown/README.md) and the
[conversion contract](../../../reference-conversion-contract.md).

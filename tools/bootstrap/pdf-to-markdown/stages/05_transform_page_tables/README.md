# Stage 05: Transform Page Tables

Read completed Stage 04 page JSONs and Stage 02 original PNGs. For each `table`,
make one Codex request using its exact unpadded pixel crop. Replace only the
table's `md_text` with the returned Markdown or HTML and record `table_format`.
Preserve identity, geometry, classification, continuation, object order and all
other source objects, including separate captions, notes and legends.

Save a full page JSON only when a table transformation changes that page's data.
Pages without tables and unchanged table pages remain solely in Stage 04.
The orchestrator records this invocation's page range in the 05 status entry;
Future book processing overlays the saved replacements on that selected filtered
dataset. An empty output directory is valid; unchanged pages remain in Stage 04.
Console progress is flushed immediately: each table logs its identity, position
within the page, request start, returned format and elapsed time. Each page logs
whether it was saved, unchanged or skipped because it contains no tables.

The [prompt](prompt.md) uses Markdown where possible and HTML with `rowspan` and
`colspan` for merged cells. An `unconverted` result preserves the original content
and raster fallback. Transport, authentication and JSON parsing failures stop the stage.
The configured model/effort, original image detail, ChatGPT authentication and
shared request cache remain in use.

Crops are temporary request inputs and are removed after execution. No persistent
source crops, group assignments, descriptions, RAG text or `<details>` companions
are generated. Retired table companion fields are omitted from converted output.
Markdown generation, description and grouping work belongs to separately
requested book processing.

Stage 04 is the fixed predecessor; missing filtered pages never fall back to
Stage 03. Restart through the orchestrator clears this stage and later outputs,
preserving predecessors and request cache.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 05 --to-stage 05
```

Use a named attempt and only the fragment selected by the user. Existing saved
outputs require explicit regeneration to adopt this change. See
[book processing](../../../book-to-markdown/README.md) and the
[conversion contract](../../../reference-conversion-contract.md).

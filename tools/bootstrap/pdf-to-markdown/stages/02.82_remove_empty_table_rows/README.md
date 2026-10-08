# Stage 02.82: Remove Empty Table Rows

Read completed Stage 01/02.81 page artifacts and remove rows whose cells are all
empty from converted Markdown and HTML tables. This deterministic stage makes
no model requests. Markdown headers and separator rows remain; partially empty
rows remain. Whitespace and HTML nonbreaking spaces count as empty. HTML cells
containing images or other embedded content remain, as do rows occupied by
nonempty cells spanning from earlier rows. Adjust affected rowspans when removing
empty rows, preserving column spans.

Write full selected page JSONs and copy saved table crops into
`02.82_remove_empty_table_rows/`. Preserve other objects, source geometry, group
metadata, Markdown/HTML RAG companions and unconverted tables. Stage 02.81 artifacts stay
unchanged. The invocation's `--page-ranges` selects physical pages.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.82 --to-stage 02.82
```

Restart clears 02.82 and all later outputs, preserving Stage 02.81 transcription.
[Review 02.83](../02.83_table_conversion_review/README.md),
[page export 02.90](../02.90_emit_page_markdown/README.md) and
[stream 03](../03_build_raw_stream/README.md) require completed Stage 02.82;
they do not fall back to uncleaned tables. Existing attempts require this stage
and explicit regeneration of the desired downstream artifacts.

# Stage 02.81: Transform Page Tables

Read completed Stage 02.8 page JSONs and Stage 01 original PNGs. Only `table`
objects are transcribed. Preserve full pages, identities, geometry, classifications,
continuation flags and reading order. Save exact unpadded original-resolution crops
once in `assets/<segment_id>.png`, referenced by stage-relative `table_source_asset`.
All outcomes retain their crops. Restart through the orchestrator rebuilds the stage
and clears all later outputs while preserving predecessors and request cache.

One Codex request per crop returns `format` (`markdown`, `html`, `unconverted`)
and `md_text`. Converted tables retain `type: table`, with markup in `md_text`
and the result in `table_format`. Unconverted tables retain original content and
raster behavior. Transport, authentication and JSON parsing failures stop the stage.
The minimal [table prompt](prompt.md) uses configured `gpt-6.1-sol`, medium effort,
original image detail, ChatGPT authentication and the shared request cache.

Collect adjacent captions above the table and the adjacent mixed run of captions,
footnotes and table legends below it. Stop at unrelated content or another table;
do not join pages. Record source-ordered `table_group_segment_ids`. Warn about
repeated associated types, captions on both sides and ambiguous shared ownership.
Keep every source block once in the visible document, including anomalous blocks.

HTML results require a separate [group inference](group_prompt.md) with the original
crop, HTML and all associated texts in source order. Save its complete nonempty
text in `table_rag_text` before completing. Markdown/unconverted results skip it.
Changed group inputs change the cache identity. Export and review reuse this text,
without reinference, summarization or appended copies of associated blocks.

After the last group block, HTML export adds a collapsed `Table text for RAG`
details section with a literal fenced text block, then a collapsed `Original table
image` section referencing the bundled crop. Markdown tables publish no crop;
unconverted tables retain their visible raster. A conservative local HTML heuristic
warns about regular grids without effective merges or complex cell content, including
cache results; it does not change markup, retry inference or certify fidelity.

Default predecessor is `02.8`. Only when filtering was deliberately skipped for the
entire selected input, explicitly restart with `--table-predecessor 02`. The selection
is persisted as `table_conversion.predecessor`; its completed status is required.
Missing filtered pages never fall back individually to Stage 02.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.81 --to-stage 02.81
```

Use a named attempt and only the fragment selected by the user. Do not rewrite
an existing attempt's model settings or continue beyond the requested end stage.
See [review](../02.82_table_conversion_review/README.md),
[page export](../02.9_emit_page_markdown/README.md) and
[conversion contract](../../../reference-conversion-contract.md).

The explicit whole-input predecessor `02` shares the 02/02.4 page resolver with
review and filtering: enumerate Stage 02 filenames, select same-name overrides
only after successful 02.4, and block running/failed 02.4. An absent optional
status means skipped. Default 02.8 input remains authoritative, including page
omissions. Table groups and companions are inferred from corrected objects here.

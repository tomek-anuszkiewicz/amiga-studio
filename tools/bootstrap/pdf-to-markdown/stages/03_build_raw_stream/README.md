# Stage 03: Build Raw Stream

Consume completed Stage 02.81 page objects and Stage 01 page geometry. Preserve
physical-page/segment order, identities, types, heading levels, continuation flags,
`md_text` and integer `bbox_pixels`. There is no fallback to earlier JSON or review
PNG input. Empty input produces an empty stream and assets directory.

Copy Markdown into `raw_text`; normalized and point-based boxes support existing
stream consumers. Converted tables initialize `rendered_markdown` with their markup.
Source captions, footnotes and legends also retain their Markdown for final assembly.
Carry `table_format`, `table_rag_text`, `table_group_segment_ids` and
`table_source_asset` unchanged. Stage 04 keeps converted table fragments separate,
Stage 06 does not join them and Stage 07 does not reconvert or hide them.

Copy saved Stage 02.81 crops for HTML companions and unconverted tables into
`assets/asset_<node_id>.png`; Markdown tables publish no crop. Graphics, covers and
code blocks still use the existing Stage 01 cropper with padding and neighbor limits.
Asset paths move forward with stage bundles. Final Stage 11 uses the same group
assembly as page export and review, appending both collapsed HTML companions after
the last associated block exactly once, without reinference or appended group text.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 03 --to-stage 03
```

Completed Stage 01/02.81 statuses are required even when reviews or 02.9 have not
run. Restart cleanup follows execution order. Existing streams require explicit
regeneration through 02.81 to adopt table conversion. Success establishes execution,
not transcription fidelity or RAG ingestion behavior.

Stage 02.4 replacement `source_segment_ids` and `replacement_stage` are carried
in node metadata alongside unchanged callout roles and text. The stream contains
replacement objects once; consumed predecessor objects are not reintroduced.

Optional graphic `rotation` from 02.44 survives on each stream node and in its
metadata. Missing decisions remain absent. Asset crops use unchanged source pixels;
applying the saved clockwise correction is deferred.

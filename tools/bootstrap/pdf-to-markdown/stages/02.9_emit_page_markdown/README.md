# Stage 02.9: Page Markdown and Assets

Emit one `document.md` and an always-present `assets/` bundle from completed
Stage 02.81 full page JSONs and Stage 01 original PNGs. Preserve physical-page
and segment-array order, unchanged textual Markdown, captions, notes and legends.
Minimal YAML and a heading use the source filename stem. Empty input remains valid.

Converted tables emit `md_text` at their original position. HTML tables append
one collapsed `Table text for RAG` section containing the saved complete group text
in a literal fenced text block after the last associated block, followed by a
collapsed `Original table image` section. Associated source blocks remain visible
once in their original positions. Markdown tables have neither companion nor
published crop. Unconverted tables retain their visible raster.

Copy required table crops from Stage 02.81 `table_source_asset`; never recrop them.
Graphics and covers still use exact integer Stage 01 pixel boxes, without padding
or rescaling. All links are relative under `assets/`, named by segment ID.
No inference, rewriting, joining, filtering or RAG-text regeneration occurs.
See [Stage 02.81](../02.81_transform_page_tables/README.md) for group collection.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.9 --to-stage 02.9
```

Write a temporary bundle before replacing old output, removing obsolete assets.
IO failures stop the stage. Completed Stage 01/02.81 statuses are required, without
fallback to 02/02.8 or dependency on review PNGs. Restart clears this stage and all
later results. Stage 14 publication remains separate; `--publish` does not publish
this intermediate bundle. The user assesses source fidelity on the selected fragment.

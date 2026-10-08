# Stage 02.90: Page Markdown and Assets

Emit one `document.md` and an always-present `assets/` bundle from completed
Stage 02.81 full page JSONs and Stage 01 original PNGs. Preserve physical-page
and segment-array order, unchanged textual Markdown, captions, notes and legends.
Minimal YAML and a heading use the source filename stem. Empty input remains valid.

Converted tables emit their saved Markdown or HTML directly at their original
position. No `<details>` sections, RAG text, group descriptions or source-image
companions are appended. Captions, notes and legends remain separate source
blocks in their original order. Unconverted tables retain a visible raster,
cropped from their exact Stage 01 pixel box.

Graphics and covers still use exact integer Stage 01 pixel boxes, without padding
or rescaling. All links are relative under `assets/`, named by segment ID.
No inference, rewriting, joining, filtering or RAG-text regeneration occurs.
See [Stage 02.81](../02.81_transform_page_tables/README.md) for group collection.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.90 --to-stage 02.90
```

Write a temporary bundle before replacing old output, removing obsolete assets.
IO failures stop the stage. Completed Stage 01/02.81 statuses are required, without
fallback to 02/02.80 or dependency on review PNGs. Restart clears this stage and all
later results. `--publish` publishes this completed bundle to the protected sibling
book destination. The user assesses source fidelity on the selected fragment.

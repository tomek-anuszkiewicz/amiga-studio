# Stage 07: Transform Tables

## Objective
Pass through Stage 02.81 Markdown/HTML tables, saved group text and source assets
unchanged. Converted continuation fragments remain separate and visible. The
legacy transformation below applies only to unconverted table blocks:

Specialized worker for table blocks across all chapter streams:
1. Skips child continuation nodes (which are synthesized into their respective head nodes).
2. For standalone or head tables, combines multi-page crops and `raw_text` from the chapter JSON nodes.
3. Formats complex merged-cell tables into semantic HTML table markup (preserving `colspan`, `rowspan`, and hierarchical headers).
4. Formats simple tabular data into GitHub-Flavored Markdown tables (preserving 4-column book structures, Unicode arrows, and KaTeX math).
5. Preserves SVG vector embeds for tables exceeding formatting limits.

## Inputs
- `workspace/06_detect_continuations/{index:02d}_{slug}.json`: Chapter streams with continuation metadata from Stage 06.
- `workspace/06_detect_continuations/assets/`: Visual crops (`.svg`, `.png`).
- `stages/07_transform_tables/prompt.md`: Prioritized table formatting prompt (HTML vs GFM).
- `stages/07_transform_tables/prompt_markdown_table.md`: GFM formatting prompt.
- `stages/07_transform_tables/prompt_html_table.md`: HTML table formatting prompt.

## Outputs
- `workspace/07_transform_tables/{index:02d}_{slug}.json`: Chapter streams with rendered table markup in `node.rendered_markdown`.

## Invocation Through the Orchestrator
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 07 --to-stage 07
```

Chapter JSON includes `index`, `slug`, `title`, `target_md_file` and `nodes`.
This automatic stage transforms nodes while preserving metadata and prior-stage files.

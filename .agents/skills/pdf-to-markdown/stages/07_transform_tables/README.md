# Stage 07: Transform Tables

## Objective
Specialized worker for table blocks across all chapter streams:
1. Skips child continuation nodes (which are synthesized into their respective head nodes).
2. For standalone or head tables, combines multi-page crops and raw text files.
3. Formats complex merged-cell tables into semantic HTML table markup (preserving `colspan`, `rowspan`, and hierarchical headers).
4. Formats simple tabular data into GitHub-Flavored Markdown tables (preserving 4-column book structures, Unicode arrows, and KaTeX math).
5. Preserves SVG vector embeds for tables exceeding formatting limits.

## Inputs
- `workspace/06_detect_continuations/{index:02d}_{slug}.json`: Chapter streams with continuation metadata from Stage 06.
- `workspace/assets/`: Visual crops (`.svg`, `.png`) and underlying text dumps (`.txt`).
- `stages/07_transform_tables/prompt.md`: Prioritized table formatting prompt (HTML vs GFM).
- `stages/07_transform_tables/prompt_markdown_table.md`: GFM formatting prompt.
- `stages/07_transform_tables/prompt_html_table.md`: HTML table formatting prompt.

## Outputs
- `workspace/07_transform_tables/{index:02d}_{slug}.json`: Chapter streams with rendered table markup in `node.rendered_markdown`.

## Standalone Invocation
```powershell
python stages/07_transform_tables/transform_tables.py --workspace "workspace"
```

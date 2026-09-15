# Stage 07: Transform Tables

## Objective
Specialized worker for table blocks across all chapter streams:
1. Skips child continuation nodes (which are synthesized into their respective head nodes).
2. For standalone or head tables, combines multi-page crops and raw text files.
3. Formats simple tabular data into GitHub-Flavored Markdown tables (preserving 4-column book structures, Unicode arrows, and KaTeX math).
4. Formats complex merged-cell tables into semantic HTML table markup.
5. Preserves SVG vector embeds for tables exceeding formatting limits.

## Inputs
- `workspace/chapters/*.json`: Partitioned section stream files.
- `stages/07_transform_tables/prompt_markdown_table.md`: GFM formatting prompt.
- `stages/07_transform_tables/prompt_html_table.md`: HTML table formatting prompt.

## Outputs
- Updated `workspace/chapters/*.json` with formatted table markup in `node.rendered_markdown`.

## Standalone Invocation
```powershell
python stages/07_transform_tables/transform_tables.py --workspace "workspace"
```

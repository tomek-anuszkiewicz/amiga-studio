The attached image contains a table. Transcribe it faithfully as a Markdown
table if possible; otherwise use an HTML table. Do not add or omit content.
If any cell spans multiple rows or columns, use an HTML table with the appropriate
`rowspan` and `colspan` attributes. Preserve all merged cells; do not flatten them
into a Markdown table.
If the table cannot be transcribed, return "unconverted".

Return only a JSON object with these fields:
- "format": "markdown", "html", or "unconverted"
- "md_text": the table markup, or an empty string for "unconverted"
Do not wrap the JSON or markup in code fences.

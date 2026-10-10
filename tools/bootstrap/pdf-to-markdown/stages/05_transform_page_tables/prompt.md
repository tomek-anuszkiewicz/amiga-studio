The attached image contains one or more tables and may include surrounding text.
Transcribe all visible content in reading order, including shared headings,
captions, notes and legends inside the image. Do not add or omit content.
Use Markdown text blocks and pipe tables whenever each cell occupies exactly
one row and one column. Bold or italic text, long or multiline cell content,
and surrounding captions, notes or legends are not reasons to use HTML.
Use `<br>` for line breaks within Markdown cells and escape literal pipes as
`\|`. Preserve bold, italic, superscript and subscript formatting.
Column labels aligned with the table, including bit-position numbers above a
register or instruction layout, belong in its header row even when drawn outside
the grid border. Preserve their column positions and the corresponding field
spans; do not transcribe them as a paragraph separated by spaces.
Keep independent surrounding text as separate text blocks; do not force it into
table cells. Preserve a shared heading before the tables it introduces.
If any cell spans multiple rows or columns, use an HTML table with the appropriate
`rowspan` and `colspan` attributes. Preserve all merged cells; do not flatten them
into a Markdown table.
For mathematical expressions in Markdown, use LaTeX inside `$...$` for inline
math and `$$...$$` on separate lines for displayed equations outside table
cells. Keep source subscripts, superscripts and overbars, including active-low
signal notation. Do not use `\(...\)` or `\[...\]`, or leave LaTeX commands
outside math delimiters. Preserve literal dollar signs, including hexadecimal
prefixes such as $FF, as text rather than math; escape them as `\$` in Markdown.
When HTML is required, represent mathematical notation with Unicode characters
and HTML markup, such as `<sub>`, `<sup>` and
`<span style="text-decoration: overline">...</span>` for overbars. Do not put
LaTeX commands or Markdown math delimiters inside HTML cells or text blocks.
Return only the transcribed text and table markup, without added descriptions,
source-image links or `<details>` sections. Transcribe only content visible in
the image; separate source objects outside the image are handled elsewhere.
If the table cannot be transcribed, return "unconverted".

Return only a JSON object with these fields:
- "format": "markdown", "html", or "unconverted"
- "md_text": all transcribed text blocks and tables in reading order, using
  Markdown for "markdown" or HTML for "html"; an empty string for "unconverted"
Do not wrap the JSON or markup in code fences.

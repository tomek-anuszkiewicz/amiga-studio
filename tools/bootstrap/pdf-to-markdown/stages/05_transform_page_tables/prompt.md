The attached image contains one or more tables and may include surrounding text.
Transcribe all visible content in reading order, including shared headings,
captions, notes and legends inside the image. Do not add or omit content.
Use Markdown text blocks and pipe tables whenever each cell occupies exactly
one row and one column. Bold or italic text, long or multiline cell content,
and surrounding captions, notes or legends are not reasons to use HTML.
Use `<br>` for line breaks within Markdown cells and escape literal pipes as
`\|`. Preserve bold, italic, superscript and subscript formatting.
Escape literal angle-bracket notation from the source as HTML entities inside
cells: `#<data>` must be written as `#&lt;data&gt;`. Keep actual formatting tags
such as `<br>`, `<sub>` and `<sup>` as markup.
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
If the table cannot be transcribed, use the "unconverted" format with empty
"md_text" in the JSON object below.

Return only a JSON object with these fields:
- "format": "markdown", "html", or "unconverted"
- "md_text": all transcribed text blocks and tables in reading order, using
  Markdown for "markdown" or HTML for "html"; an empty string for "unconverted"
Do not wrap the JSON or markup in code fences.

The entire response must be valid JSON, starting with `{` and ending with `}`,
with exactly one "format" field and one "md_text" field. Use one pair of double
quotes around each field name, followed by a colon. In particular, write
"md_text": without an extra quote before the field name. Do not include comments,
trailing commas, explanations or text outside the object.
Encode the transcribed markup as one JSON string: use `\n` for line breaks,
`\"` for quotes inside the text and `\\` for every literal backslash.
These JSON escapes also apply to Markdown and LaTeX: Markdown `\|` and `\$`
become `\\|` and `\\$` inside the JSON string, and LaTeX `\overline`
becomes `\\overline`. After decoding JSON, the text must retain its intended
Markdown or HTML syntax. Check the complete object's JSON syntax before returning.

Response examples (illustrative syntax only; transcribe the attached image's
content rather than copying these sample cells):

Markdown table:
{"format":"markdown","md_text":"| Bit | Signal |\n| --- | --- |\n| 0 | START |"}

HTML table with a merged cell and quoted attributes:
{"format":"html","md_text":"<table><tr><th colspan=\"2\">Signals</th></tr><tr><td>0</td><td>START</td></tr></table>"}

Markdown with literal pipes, hexadecimal notation, quotes and LaTeX:
{"format":"markdown","md_text":"| Label | Value |\n| --- | --- |\n| A\\|B | \\$FF |\n| \"READY\" | $\\overline{RESET}$ |"}

Unconverted result:
{"format":"unconverted","md_text":""}

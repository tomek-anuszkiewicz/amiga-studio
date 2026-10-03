# Stage 12: Convert Tables & Classify Images Prompt

You are an expert technical document transcriber.

Your task in Stage 12 is to inspect each cropped visual asset (`_clip_final.png`):
1. If the asset visually represents tabular data (simple data tables, standard rectangular grids, tables with `colspan`/`rowspan`, multiline cells, dual alignments, or opcode summaries), transcribe it into a clean, semantic HTML `<table>` paired with a collapsed fixed-width text table view for RAG.
2. If the asset is NOT a table (e.g. register bitfield diagrams, cycle allocation maps, signal routing diagrams, electronic circuits, waveforms, block diagrams, flowcharts, or pinout drawings), do NOT generate table markup. Classify it directly as an `image` for Stage 14 image conversion.

---

## Decision Rule

```
[ Visual Asset Crop (_clip_final.png) ]
       │
       ▼
Is this asset visibly a data table?
       ├── YES ──► Transcribe as HTML Table
       │           - Save to build/01_page_layout/assets/<asset_id>_html.md
       │           - Queue: "detected_type": "table_html", "status": "checked", "conversion_status": "completed", "conversion_stage": 12
       │
       └── NO  ──► Classify Directly as Image (for Stage 14)
                   - Do NOT generate any file
                   - Queue: "detected_type": "image", "status": "checked", "conversion_status": "pending", "conversion_stage": null
```

---

## What to Convert as HTML Table

Convert the asset if it presents tabular data:
- Standard rectangular data tables: bordered or borderless tables with clear columns and rows (e.g. instruction execution time tables, addressing mode tables, numeric lookup tables, register summaries).
- Tables with grouped headers or spanning cells using `colspan` or `rowspan`.
- Multi-tier parameter tables, opcode summaries, condition code listings, or memory address ranges that have clear rows and columns.
- Tabular signal / pin lists (e.g. `Pin Number | Signal Name | Description`).
- Ignore outer publisher typesetting decorative framing boxes around the table.

## What to Classify as Image (Ready for Stage 14)

Do NOT transcribe as a table if the asset represents a non-tabular graphic or complex hardware diagram:
- **Register bitfield diagrams** (boxes showing individual bit positions 15..0 or 31..0 with callout lines).
- **Cycle allocation grids** and bus timing allocation maps.
- **Signal routing diagrams** and pin-to-wire connection maps.
- **Visual schematics, waveforms, state machines, block diagrams, flowcharts, or chip pinout drawings.**
- Any non-tabular graphic.

Classify these assets as `image` in your completion contract. Stage 14 will synthesize full architectural breakdowns and RAG text indexing for them.

---

## Output Target: `build/01_page_layout/assets/<asset_id>_html.md`

When converting as a table, write the transcribed content directly to:
`build/01_page_layout/assets/<asset_id>_html.md`

Output ONLY the raw content. Do not include outer triple backtick blocks (```markdown), conversational filler, or commentary. Do NOT generate external `.txt` files for tables (external `.txt` files are strictly for Stage 14 images).

### Structural Requirements
Each generated file must contain two components in sequence:
1. **Semantic HTML Table (`<table border="1">`):**
   - **Enclosed Technical Grid (`<table border="1">`):** Always use `<table border="1">` as the opening tag so that the table renders with an outer bounding box and complete vertical/horizontal cell dividers matching original technical reference manuals across all Markdown previewers and HTML renderers.
   - Use standard tags: `<thead>`, `<tbody>`, `<tr>`, `<th>`, `<td>`.
   - Use `colspan="..."` and `rowspan="..."` where appropriate.
   - Do NOT use inline styles, colors, or presentation attributes unless strictly necessary for semantic alignment (such as dual left/right alignment).
   - **Dual Left/Right Text Alignment in Table Cells:** When transcribing tables where a single column contains split or dual-aligned content (e.g. operand/sign symbols `+` / `-` aligned to the left and numeric values or infinity indicators aligned to the right within the same cell, such as in arithmetic operation tables like `FDIV Operation Table`), use CSS inline styles or spans (e.g. `<span style="float: left;">+</span><span style="float: right;">+0.0</span>`) to preserve both alignments faithfully without merging or losing positional layout.
   - **Unrolling Side-by-Side Space-Saving Printed Columns:** When a reference table is split into multiple parallel side-by-side column blocks on the printed page purely to conserve vertical space (e.g. 4 side-by-side column blocks representing effective addressing modes split into two pairs), unroll the parallel blocks into continuous single-column tables (e.g. 2 continuous single-column tables) to preserve clean vertical reading flow in Markdown.
   - **Preserve Cell Text Formatting & Notation:** Maintain exact notation inside cells including parentheses, slashes, and numbers (e.g. execution time pairs `0(0/0)`, `8(2/0)`, `48(11/1)`), addressing mode specifications (`(xxx).W`, `(xxx).L`, `(d16, An)`, `(d8, An, Xn)*`), and category subheaders (`Register`, `Memory`).
   - **Plain Text Addressing Modes (NO Subscript Tags):** Transcribe displacement and index notations strictly as plain monospace/text strings without HTML `<sub>` or `<sup>` tags (e.g. write `(d16, An)`, `(d8, An, Xn)`, `(d16, PC)`, `(d8, PC, Xn)*` — NEVER `(d<sub>16</sub>, An)`). Never inject styling tags (`<sub>`, `<sup>`, `<font>`, or inline styles) into table cells unless strictly needed for dual left/right alignment.
   - **Category Subheaders & Column Spans (`colspan`):** When a table row represents an internal category or section header dividing rows (e.g. `Register`, `Memory`), use `<td colspan="N"><strong>Register</strong></td>` (or `<th>`) across all data columns rather than uneven empty cells.
   - **Escape HTML Entities in Cells:** Always escape literal `<` and `>` in cell text as `&lt;` and `&gt;` (e.g. `#&lt;data&gt;` or `<code>#&lt;data&gt;</code>`) so that browsers and parsers do not treat them as unclosed HTML tags.
   - **Document Scope Isolation (Zero Cross-Manual Reads):** Rely strictly and exclusively on the rules, schemas, and specifications in this prompt. NEVER search the workspace, browse other directories, or inspect table files from other manuals to find formatting examples.
2. **Collapsed RAG Text Table View (`<details>`):**
   - Must contain a clean, fixed-width, space-aligned plain text representation of the table columns wrapped in a fenced text code block (` ```text ... ``` `).
   - Do NOT use Markdown table pipes (`|`) or Markdown formatting inside the text block.
   ```html
   <details>
   <summary>RAG</summary>

   ```text
   Header 1      Header 2
   ------------------------
   Value 1       Value 2
   ```

   </details>
   ```

---

## Output Protocol

In the response turn immediately following visual inspection of `assets/<asset_id>_clip_final.png`:
- **If converted as HTML Table:**
  1. `write_to_file`: Save `build/01_page_layout/assets/<asset_id>_html.md` (Semantic HTML `<table>` + collapsed `<details>` fixed-width text table view).
  2. Include the asset ID in `converted_tables` in your completion summary contract. Do NOT edit queue files on disk.
- **If classified as Image (Complex Hardware / Graphical Diagram):**
  - Do NOT generate any file. Include the asset ID in `classified_images` in your completion summary contract. Proceed immediately to the next asset.

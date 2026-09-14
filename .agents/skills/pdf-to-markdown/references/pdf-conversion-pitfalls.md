# PDF to Markdown: Technical Pitfalls & Proven Solutions

This document details the 13 known technical pitfalls encountered when converting complex technical manuals and hardware books from PDF to Markdown for Obsidian, along with their authoritative solutions derived from real-world conversion projects (*A500 A2000 Technical Reference Manual*, *68000 User's Manual*, *68000 Programmer's Reference Manual*).

---

## 1. Multi-Page Split Tables

### Problem
In printed books, large tables span across multiple physical pages (e.g. Table 6-1 Pin Descriptions, Memory Maps, Register Lists). When performing page-by-page transcription, the table is split into multiple separate Markdown tables across consecutive page files:
- The second page often repeats the header row and separator (`| --- | --- |`).
- A table row may be split mid-sentence or mid-cell across the page boundary.
- Multiple separate tables disrupt reading flow and prevent unified sorting or searching in Obsidian.

### Solution & Rules
1. **Identify Continuations**: When Page $N+1$ begins with a table whose column headers or column count match the table ending Page $N$, treat it as a continuation.
2. **Strip Repeating Headers**: Retain the header and separator row (`| --- |`) *only* from the first page's table. Strip the repeated header row and separator line from the second page.
3. **Join Interrupted Rows**: If the last row of Page $N$ was incomplete (e.g. an unfinished sentence in the description column), concatenate the continuation text from the first row of Page $N+1$ into the preceding cell rather than creating a broken row.
4. **Resulting Output**: A single unified Markdown table in the compiled chapter file.

---

## 2. Cut-Off Images, Pins, and Waveform Labels

### Problem
When diagrams are cropped from page PNGs or converted to SVG:
- Bounding boxes calculated too tightly truncate outer pin names (e.g. `IPL0`, `DTACK`, `A23`), chip borders, signal clock numbers, or captions.
- In vector SVGs, restrictive `viewBox` coordinates or tight `<clipPath>` definitions clip outer drawing elements when rendered in Obsidian.

### Solution & Rules
1. **Safety Margin on Crops**: Always add **10% to 15% padding** (typically 15-25 pixels) around visual diagram boundaries when specifying `<crop>` bounding boxes.
2. **SVG viewBox Expansion**: In SVG files, verify that `viewBox="min-x min-y width height"` covers the full graphical extent plus a 20px buffer. If paths lie outside `viewBox`, expand the width/height and shift `min-x`/`min-y`.
3. **Audit `<clipPath>`**: Disable or expand any `<clipPath>` definitions that truncate signal lines or pin callouts at the edges of the canvas.
4. **Verification**: Visually confirm that all four corners of the border, all pin labels, and all timing tick marks are fully enclosed within the image frame.

---

## 3. Text Duplicated in Image AND Markdown

### Problem
Technical illustrations often contain embedded explanatory paragraphs, numbered "Notes:", or parameter legends (e.g. DMA time slot allocation notes, bus cycle timing explanations). LLMs frequently transcribe the text into Markdown prose *and* leave the exact same text inside the cropped image, creating jarring visual duplication.

### Solution: The Clean Separation Principle
1. **Transcribe Notes to Markdown**: Extract all explanatory prose, legends, footnotes, and "Notes:" into clean Markdown text immediately below the figure, formatted as an Obsidian callout:
   ```markdown
   > [!NOTE] DMA Time Slot Notes
   > 1. If divide by zero occurs, an exception occurs.
   > 2. If overflow occurs, neither operand is affected.
   ```
2. **Clean the Image Asset**: Crop or mask the image so it contains *only* the schematic, waveform, or block diagram. Never include paragraphs of explanatory text inside the graphic asset.

---

## 4. Disappearing / Dropped Content (Omissions)

### Problem
Dense technical pages contain fine print, register bit sub-notes, footnote markers (`*`), or small sidebars that LLM vision passes accidentally omit or summarize away.

### Solution & Rules
1. **Cardinal Content Rule**: Reformatting layout is encouraged; **omitting or summarizing text is strictly forbidden**.
2. **Visual QA Check**: Run a double-check pass comparing the transcribed Markdown against the original rendered page PNG. Specifically verify:
   - Footnotes at the bottom of the page.
   - Column values in dense tables.
   - Fine print below equations.
   - Pre-conditions or error code notes.

---

## 5. ASCII Art vs Table vs Monotone Text vs Image

### Problem
Monospace ASCII art often deforms across different fonts, zooms, and screen widths, becoming unreadable or ugly.

### Solution: Decision Hierarchy
- **Tabular Data** (registers, memory maps, pinouts) $\rightarrow$ **Markdown Table**.
- **Fixed-width representation** (memory hex dumps, terminal output) $\rightarrow$ **Monotone Text Block (` ```text `)**.
- **Programming code** $\rightarrow$ **Code Block (` ```m68k `, ` ```c `)** with standardized indentations.
- **Flowcharts / State Machines** $\rightarrow$ **Mermaid (` ```mermaid `)**.
- **Complex Schematics / Waveforms / Circuitry** $\rightarrow$ **PNG Crop or clean SVG** in `assets/`. Never attempt to recreate detailed analog or bus waveforms as ASCII art.

---

## 6. The `<crop>` Tag Standard

During page-by-page vision transcription, images are marked with a standardized XML-style crop tag:

```markdown
<crop xmin="120" ymin="340" xmax="950" ymax="780" label="Figure 6-2. Fat Agnus Block Diagram" />
```

### Attributes:
- `xmin`, `ymin`, `xmax`, `ymax`: Integer pixel coordinates on the rendered page PNG (at 150 or 200 DPI).
- `label`: Full human-readable figure title and caption.
- `alias` *(optional)*: Short alias name if referenced under alternative names.

Post-processing scripts extract the crop, apply safety padding, save to `assets/`, and replace the `<crop>` tag with:
```markdown
![Figure 6-2. Fat Agnus Block Diagram](assets/section_06_figure_6-2_fat_agnus_block_diagram.png)
```

---

## 7. Table of Contents (TOC) Formatting

- File location: `00 - Table of Contents and Front Matter.md`.
- Include document metadata callout (`> [!NOTE]` with Author, Version, Date).
- Hierarchical bulleted list with relative URL-encoded Markdown links to each chapter:
  ```markdown
  ## Contents
  - [Section 1: Summary of Differences](01%20-%20Section%201%20Summary%20of%20Differences.md)
  - [Section 2: System Block Diagrams](02%20-%20Section%202%20System%20Block%20Diagrams.md)
    - [2.1 Fat Agnus](02%20-%20Section%202%20System%20Block%20Diagrams.md#2.1%20Fat%20Agnus)
  ```
- Exclude obsolete print front-matter: remove "List of Tables" and "List of Figures".

---

## 8. File Naming Standard

All chapter files follow the two-digit zero-padded index format:
```text
NN - Section X - Title.md   (or NN - Chapter X - Title.md)
```
Examples:
- `00 - Table of Contents and Front Matter.md`
- `01 - Section 1 - Summary of Differences.md`
- `02 - Section 2 - System Block Diagrams.md`
- `11 - Section 6 - Fat Agnus Chip.md`

This guarantees natural alphanumeric sorting across all operating systems, git repositories, and Obsidian file trees.

---

## 9. Asset Storage & Naming Standard

- **Storage Location**: Stored in a local `assets/` subfolder directly adjacent to the markdown files:
  `Obsidian/Amiga/Reference/<DocumentName>/assets/`
- **Naming Pattern**: All lowercase, words separated by underscores, prefixed by section number and figure ID:
  `section_<two_digit_sec>_figure_<id>_<descriptive_slug>.<ext>`
- Examples:
  - `section_06_figure_6-2_fat_agnus_block_diagram.svg`
  - `section_06_figure_6-7_8520_read_timing_diagram.png`
  - `section_01_centronics_db25_connector.svg`

---

## 10. Math in HTML Tables

### Problem
In CommonMark and Obsidian, Markdown syntax (including LaTeX `$..$` delimiters) inside raw HTML tags (`<table><tr><td>...</td></tr></table>`) is **not parsed by default**. If you write `<td>$2^{10}$</td>`, it displays as literal text `$2^{10}$`.

### Solution
1. **Prefer Standard Markdown Tables**: Whenever possible, use standard Markdown tables where `$math$` renders natively.
2. **If HTML Table is Mandatory** (for complex `colspan`/`rowspan`):
   - Use pure HTML and Unicode entities for superscripts, subscripts, and math symbols:
     - $2^{10}$ $\rightarrow$ `2<sup>10</sup>`
     - $T_{CLK}$ $\rightarrow$ `T<sub>CLK</sub>`
     - $\pm 5\%$ $\rightarrow$ `&plusmn;5%`
     - $\Omega$ $\rightarrow$ `&Omega;`
     - $\mu s$ $\rightarrow$ `&mu;s`
     - $\times$ $\rightarrow$ `&times;`
   - This ensures 100% reliable, beautiful typography without depending on LaTeX parsers inside HTML blocks.

---

## 11. Motorola Hex Escaping vs KaTeX Math Corruption

### Problem
In Motorola 68000 assembly and Amiga hardware reference manuals, hexadecimal addresses and constants are written with a dollar sign prefix: `$00000004`, `$DFF000`, `$FFFF`, `$C00000`.
In Markdown and KaTeX, two unescaped dollar signs on the same line or paragraph trigger KaTeX math mode:
```text
The vector table spans from $00000000 to $00000400 in Chip RAM.
```
KaTeX attempts to parse `00000000 to 00000400` as a mathematical expression, producing mangled italics and rendering errors!

### Solution: Strict Hex Backtick Rule
- **Never leave hexadecimal values as bare `$HEX` in prose.**
- **Always enclose hexadecimal values in inline code backticks**:
  ```markdown
  The vector table spans from `$00000000` to `$00000400` in Chip RAM.
  Custom chip registers begin at `$DFF000`.
  ```
- Reserve `$math$` exclusively for actual mathematical formulas (e.g. `$2^{32} - 1$`, `$f = \frac{1}{2\pi RC}$`).

---

## 12. Advisory, Warning, and Error Blocks (Obsidian Callouts vs Plain Text)

### Problem
Hardware manuals, processor programming guides, and system reference books frequently highlight crucial notices, silicon warnings, programming tips, and error conditions using graphical elements:
- Shaded boxes or double-ruled borders
- Margin warning icons (danger triangles, stop signs)
- Bold prefixes (`NOTE:`, `WARNING:`, `CAUTION:`, `IMPORTANT:`, `PROGRAMMING TIP:`)

When transcribed as flat paragraphs or standard blockquotes (`> text`), these vital notices lose their visual distinction, blend into body text, and fail to leverage Obsidian's semantic callout rendering.

### Solution: Map to Native Obsidian Callouts
Always convert advisory and alert blocks into their corresponding Obsidian callout types:

| Source PDF Styling / Marker | Obsidian Callout Target | Semantic Role |
|---|---|---|
| `Note:`, `Notice:`, `Info:`, boxed text | `> [!NOTE]` or `> [!INFO]` | General technical notes, secondary explanations, hardware background |
| `Tip:`, `Hint:`, `Programming Tip:` | `> [!TIP]` | Performance suggestions, coding tricks, cycle-saving register usage |
| `Important:`, `Attention:`, `Critical:` | `> [!IMPORTANT]` | Prerequisites, non-negotiable register bit requirements, memory alignment |
| `Warning:`, `Caution:`, `Alert:` | `> [!WARNING]` or `> [!CAUTION]` | Hardware risks, bus contention hazards, undefined register bit behaviors |
| `Error:`, `Danger:`, `Fatal:`, `Bug:` | `> [!DANGER]` or `> [!ERROR]` / `> [!BUG]` | Destructive operations, silicon errata, fatal CPU exceptions, bus lockups |

### Implementation Rules:
1. **Preserve Complete Content**: Never summarize or truncate the warning/note. Retain all parameters, hex values, and references.
2. **Specific Titles**: If the box has a specific title, put it on the declaration line: `> [!WARNING] Blitter Bus Contention`.
3. **Multi-Line & Code**: Prefix every line with `>`, including empty lines between paragraphs and code block fences:
   ```markdown
   > [!IMPORTANT] Address Alignment
   > Word and longword memory accesses on the 68000 must be aligned to even byte addresses. An odd address access triggers an Address Error exception (Vector 3).
   >
   > ```m68k
   > move.w  $0003,d0    ; Triggers Address Error!
   > move.w  $0004,d0    ; Valid aligned read
   > ```
   ```

---

## 13. Blind Bitmap Tracing vs Native PDF Vector Extraction

### Problem
When converting architectural diagrams, block schematics, or bus timing charts to SVG, agents often follow a naive raster-first pipeline: render the page to a PNG bitmap $\rightarrow$ crop the bounding box $\rightarrow$ run an automatic bitmap tracer (like `potrace`, an online converter, or LLM SVG re-generation).
However, many technical PDF documents were originally generated digitally from PostScript, FrameMaker, troff, or CAD vector packages. In these PDFs, diagrams and schematics **are already stored as mathematical vector primitives (Bézier curves, lines, polygons) and true font text objects**.
Blindly rasterizing to a bitmap and then attempting to re-trace:
- Degrades clean straight lines into wobbly or rounded approximations.
- Converts sharp, selectable typography into unsearchable bitmap pixels or deformed vector paths.
- Wastes effort redrawing or approximating a graphic that already exists in pristine vector form inside the PDF file.

### Solution: The "Check PDF First" Principle
Always inspect the source PDF page before attempting to trace or redraw an SVG:

1. **Inspect the Source PDF for Native Non-Bitmap Content**:
   - Check if vector drawing operations (`m`, `l`, `c`, `re`) and text objects exist on the page:
     ```python
     import fitz  # PyMuPDF
     doc = fitz.open("path/to/manual.pdf")
     page = doc[page_num]
     drawings = page.get_drawings()
     print(f"Found {len(drawings)} native vector paths on page {page_num}")
     ```
2. **Direct Vector Extraction**:
   - If native vector content exists, extract the SVG directly from the PDF page using tools such as:
     - `pdf2svg "manual.pdf" page_number "output.svg"`
     - `mutool draw -F svg -o "output.svg" "manual.pdf" page_number`
     - PyMuPDF: `page.get_svg_image()`
   - Once exported, crop the SVG's `viewBox` to the specific diagram coordinates and remove surrounding page elements.
   - Result: 100% mathematical precision, crisp vector rendering at any zoom level, and true selectable/searchable text.
3. **Fallback to Bitmap Tracing**:
   - Only use raster-to-SVG vector tracing (or manual reconstruction) when the source PDF is verified to be a pure scanned paper scan (e.g. vintage 1980s microfiche or scanner raster images with zero embedded vector operators).

---

## 14. Carriage Return (`\r`) Table Row Splitting in LaTeX Formulas

### Problem
When generating Markdown programmatically or transcribing LaTeX formulas like `$\rightarrow$`, `$\rho$`, or `$\right.`:
- In Python, JavaScript, and shell string literals, `\r` is interpreted as a **carriage return** character (`ASCII 13`).
- A string like `"Straightforward block copy (A $\rightarrow$ D)"` is evaluated as `"Straightforward block copy (A $" + "\r" + "ightarrow$ D)"`.
- The carriage return introduces an unintended line break right in the middle of a Markdown table row!
- Because Markdown table syntax strictly requires an entire row to reside on a single uninterrupted physical line bounded by `|`, the broken line collapses the table rendering completely in editors (VS Code, Obsidian, GitHub).

### Solution
1. **Raw Strings in Code**: Always use raw string literals in Python (`r"$\rightarrow$"` or `r"""..."""`) or double-escape backslashes (`"$\\rightarrow$"`).
2. **Unicode Arrows in Prose**: For simple directional arrows in tables and prose, prefer native Unicode characters (`A → D` or `A ⇒ D`) rather than LaTeX math blocks (`$\rightarrow$`).
3. **Automated Audit**: `audit_conversion.py` validates that lines containing pipe characters `|` do not contain unescaped carriage returns (`\r`) or orphaned table cells.

---

## 15. Dual-Column Parallel Reference Tables (Compact 4-Column Layouts)

### Problem
In printed hardware manuals, dense reference tables (e.g. *Table 6-1: Table of Common Minterm Values*, register maps, opcode tables) frequently use a **dual-column parallel layout** (e.g. `Selected Equation | LF Code | Selected Equation | LF Code`) to fit 30+ items onto a single printed page without splitting across pages.
When converting to Markdown, LLMs often make two mistakes:
1. They linearize the table into a single massive, 30-row 2-column table that requires excessive scrolling.
2. They invent arbitrary artificial columns (e.g. adding `Function Name`, `Minterms Included`, `Common Graphics Use Case`), diverging from the source book's authoritative specification.

### Solution: Strict 1:1 Parallel Layout Preservation
1. **Match the Physical Table Topology**: Preserve the exact 4-column parallel structure from the manual:
   ```markdown
   ### Table 6-1: Table of Common Minterm Values

   | Selected Equation | `BLTCON0` LF Code | Selected Equation | `BLTCON0` LF Code |
   | :--- | :---: | :--- | :---: |
   | $D = A$ | `$F0` | $D = AB$ | `$C0` |
   | $D = \overline{A}$ | `$0F` | $D = A\overline{B}$ | `$30` |
   | $D = B$ | `$CC` | $D = \overline{A}B$ | `$0C` |
   | $D = \overline{B}$ | `$33` | $D = \overline{A}\overline{B}$ | `$03` |
   ```
2. **KaTeX Math Precision**: Use `\overline{...}` for negation overlines ($\overline{A}$, $\overline{B}$).
3. **Backtick Hex Codes**: All hex constants must be backticked (`` `$F0` ``) to prevent KaTeX math collision.
4. **Empty Trailing Cells**: If the left side has more entries than the right side (odd total count), pad the final right-side cells with empty spaces (`| | |`).

---

## 16. Obsidian Collapsible Callouts vs Broken Raw HTML `<details>`

### Problem
In Obsidian (particularly in Live Preview mode powered by CodeMirror 6), enclosing Markdown code blocks (```` ```text ````) inside raw HTML `<details>` and `<summary>` tags breaks CommonMark parsing:
- The HTML tags are rendered as literal text with syntax-highlighted red labels (`<details>`, `<summary>`).
- The collapsible disclosure triangle fails to render.
- The monospace text block inside leaks out uncollapsed.

### Solution: Native Obsidian Foldable Callout Syntax
Obsidian natively supports collapsible callouts using `-` (collapsed by default) or `+` (expanded by default):
```markdown
> [!NOTE]- Click to view Text / ASCII Diagram
> ```text
>        +-----+   [percntrld]
>        | 100 | ------------+
>        +-----+             |
> ...
> ```
```
- **Live Preview & Reading View**: Renders with an interactive fold toggle arrow natively across all themes.
- **Zero Raw HTML Leaks**: Eliminates all raw `<details>` and `<summary>` tag issues.

---

## 17. Multi-Tier Representation for Dense Hardware FSM / State Diagrams

### Problem
Complex physical state machines (e.g. *Figure 5-8: Audio State Diagram*) contain multiple states and transition arrows annotated with multi-line Boolean conditions and hardware action triggers (e.g. `(perfin · (AUDxON + AUDxIP)) [pbufld, AUDxDR if napnav, percntrld]`).
Attempting to force every single condition and action onto raw Mermaid arrow labels:
- Crushes the graph into an illegible horizontal spaghetti line.
- Text labels overlap into unreadable grey smudges.
- Fails both human readability and technical utility.

### Solution: The 4-Tier Representation Standard
1. **Macro Flowchart in Mermaid (`flowchart TD`)**:
   Top-down layout with isolated subgraphs (e.g. `Recovery States`), showing high-level state progression and primary enable/strobe signals (`AUDxON`, `AUDxDAT`, `perfin`).
2. **Detailed State Transition & Action Matrix Table**:
   An exhaustive Markdown table immediately following the diagram detailing every condition, qualification, and hardware action trigger.
3. **Text / ASCII Fallback inside Native Callout**:
   Monospace circuit sketch folded inside `> [!NOTE]- Click to view Text / ASCII Diagram`.
4. **Cropped High-Res Raster Scan + `.txt` Technical Sidecar**:
   Cropped figure from the original print scan (`assets/figure_XX_<slug>.png`) paired with a comprehensive `.txt` sidecar per `asset-descriptions.md` for offline Amiga RAG indexing.


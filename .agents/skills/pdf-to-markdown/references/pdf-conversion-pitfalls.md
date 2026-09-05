# PDF to Markdown: Technical Pitfalls & Proven Solutions

This document details the 11 known technical pitfalls encountered when converting complex technical manuals and hardware books from PDF to Markdown for Obsidian, along with their authoritative solutions derived from real-world conversion projects (*A500 A2000 Technical Reference Manual*, *68000 User's Manual*, *68000 Programmer's Reference Manual*).

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

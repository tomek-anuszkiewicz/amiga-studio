# Non-Text & Formatted Content Conversion Hierarchy

When converting PDF pages to Markdown, you will encounter diverse non-prose elements: programming code, register summaries, memory dumps, circuit schematics, pinout diagrams, and bus timing waveforms.

Apply this strict 7-level priority ladder to choose the best representation.

---

## The 8-Level Priority Ladder

```text
Priority 1: Code Block (```c, ```m68k)       -> For programming code; standardize indentations
Priority 2: Markdown Table                   -> For structured tabular data and register bitfield summaries
Priority 3: Native Diagram (Mermaid + ASCII)  -> For block diagrams, state machines, queues; NO redundant images
Priority 4: HTML Table                       -> For complex tables requiring cell spans (colspan/rowspan)
Priority 5: Monotone Text Block (```text)    -> For memory dumps, hex bytes, raw data alignment
Priority 6: ASCII Art (Only if Readable)     -> Fallback for simple diagrams inside <details>
Priority 7: Crop to High-Res PNG + Sidecar   -> For complex schematics, waveforms, pinouts (requires .txt sidecar)
Priority 8: Native Vector Extraction & SVG -> For scalable block diagrams when native vector paths exist in PDF
```

---

## 1. Priority 1: Code Blocks (` ```c `, ` ```m68k `, ` ```asm `)

### When to Use
Any programming code listings, functions, macros, or system calls in C, Motorola 68000 Assembly, ARexx, or shell scripts.

### Rules & Reformatting
- Always specify the explicit language tag (e.g. ```` ```m68k ```` or ```` ```c ````).
- **Standardize Indentation**: OCR often generates uneven indentation (e.g. 1 space, 5 spaces). Standardize to 4 spaces (or 2 spaces for nested structs).
- **Column Alignment for Assembly**: Cleanly align assembly columns:
  ```m68k
  Start:      move.l  #$00000004,a6       ; AbsExecBase
              moveq   #0,d0               ; Clear return code
              rts                         ; Return to caller
  ```

---

## 2. Priority 2: Standard Markdown Tables

### When to Use
Structured information with well-defined columns and rows:
- Register summaries (`Offset`, `Name`, `Read/Write`, `Description`).
- Pinout tables (`Pin #`, `Signal Name`, `Type`, `Function`).
- Instruction summary charts (`Mnemonic`, `Operation`, `Condition Codes`).
- Memory address maps (`Start`, `End`, `Size`, `Device`).

### Example
```markdown
| Address | Name | Access | Description |
|---|---|---|---|
| `$DFF000` | `BLTCON0` | W | Blitter control register 0 |
| `$DFF002` | `BLTCON1` | W | Blitter control register 1 |
| `$DFF004` | `BLTAFWM` | W | Blitter first word mask for source A |
```

### Register Bitfield Tables (Replacing Messy ASCII Art)
When encountering ASCII register bitfield boxes in manuals (e.g. `| 15 | 14 | ... | 0 |`), **convert them directly to standard Markdown tables**. Markdown tables reflow properly across devices and are immediately searchable:
```markdown
| Bit(s) | Name | Function |
| :--- | :--- | :--- |
| 15-12 | `ASH0-3` | Shift value for Blitter channel A |
| 11 | `USEA` | Enable Blitter source channel A |
| 10 | `USEB` | Enable Blitter source channel B |
| 9 | `USEC` | Enable Blitter source channel C |
| 8 | `USED` | Enable Blitter destination channel D |
| 7-0 | `LF0-7` | Minterm logic function selector |
```

### Dual-Column Parallel Reference Tables (Compact 4-Column Layouts)
Dense reference tables in printed manuals (e.g. *Table 6-1: Table of Common Minterm Values*, register maps, opcode tables) frequently use a **4-column parallel layout** (`Selected Equation | LF Code | Selected Equation | LF Code`) to fit 30+ items onto a single printed page.
- **Preserve Topology**: Never collapse into an endlessly long 2-column list or invent extraneous columns.
- **KaTeX & Backticks**: Format Boolean algebra using KaTeX ($\overline{A}$, $D = A\overline{B}$) and backtick all hex codes (`` `$F0` ``).
- **LaTeX Carriage Return (`\r`) Invariant**: Never permit raw `$\rightarrow$` inside table rows to evaluate as carriage returns (`\r`), which splits markdown table lines. Prefer Unicode arrows (`A → D`).

```markdown
| Selected Equation | `BLTCON0` LF Code | Selected Equation | `BLTCON0` LF Code |
| :--- | :---: | :--- | :---: |
| $D = A$ | `$F0` | $D = AB$ | `$C0` |
| $D = \overline{A}$ | `$0F` | $D = A\overline{B}$ | `$30` |
| $D = B$ | `$CC` | $D = \overline{A}B$ | `$0C` |
| $D = \overline{B}$ | `$33` | $D = \overline{A}\overline{B}$ | `$03` |
```

---

## 3. Priority 3: Native Diagrams (Mermaid + ASCII Fallback)

### When to Use
State machines, architectural flowcharts, block diagrams, pipeline queues, and bus handshakes.

### Rules & Reformatting
1. **Render Native Mermaid**: Use `flowchart TD` / `flowchart LR` or `sequenceDiagram`.
2. **ASCII Fallback inside Native Callout**: Provide a compact text/ASCII diagram folded inside a native Obsidian collapsible callout (`> [!NOTE]-`):
   ```markdown
   ```mermaid
   flowchart TD
       Fetch --> Decode --> Execute
   ```

   > [!NOTE]- Click to view Text / ASCII Diagram
   > ```text
   > +-------+     +--------+     +---------+
   > | Fetch | --> | Decode | --> | Execute |
   > +-------+     +--------+     +---------+
   > ```
   ```
3. **Prohibition of Raw HTML `<details>`**: Never use raw HTML `<details><summary>` tags around Markdown code blocks, as this breaks CommonMark parsing in Obsidian Live Preview and renders raw tags.
4. **No Redundant Images**: **Do NOT embed a raster image if a Mermaid diagram is generated to represent it.** Generating both an image and a Mermaid graph creates redundant visual clutter.

---

## 4. Priority 4: HTML Tables

### When to Use
When tables require complex merged cells (`colspan` or `rowspan`), multiple text blocks inside a single cell, or styled sub-headers that standard Markdown tables cannot represent.

### Critical Math Rules for HTML Tables
- Do not use standard `$math$` delimiters inside `<td>` tags (Markdown engines do not parse inline math inside HTML blocks).
- Use HTML/Unicode formatting: `2<sup>10</sup>`, `T<sub>CLK</sub>`, `&plusmn;`, `&Omega;`.

---

## 5. Priority 5: Monotone Text Blocks (` ```text `)

### When to Use
Data where exact fixed-width character alignment is essential, but which does not fit table syntax:
- Memory hex dumps (hex bytes with ASCII representation on the right).
- Interactive terminal/CLI sessions.
- Binary packet structures.

### Example
```text
00000000: 48 65 6c 6c 6f 20 57 6f  72 6c 64 21 0a 00 00 00  Hello World!....
00000010: 11 22 33 44 55 66 77 88  99 aa bb cc dd ee ff 00  ."3DUfw.........
```

---

## 6. Priority 6: ASCII Art (Only if Strictly Readable)

### When to Use
Simple compact monospace diagrams that cannot easily be converted to Mermaid and remain strictly legible within 80 columns. If it wraps or is unaligned, convert to a table or crop to an image.

---

## 7. Priority 7: Crop to High-Res PNG + Mandatory Sidecar (`.txt`)

### When to Use
Complex visual hardware illustrations that cannot be represented in Mermaid or text:
- Physical IC pin configuration diagrams.
- Analog oscillograms and bus cycle timing diagrams.
- Dense system schematics and PCB connector pinouts.

### Rules
- Render at **150 to 200 DPI**.
- Add a **10-15% safety padding margin** to the bounding box.
- Name canonically: `assets/figure_XX_<slug>.png`.
- **Mandatory Sidecar (`.txt`)**: Every image asset MUST have a companion `.txt` technical sidecar (e.g. `figure_XX_<slug>.png.txt`) containing the technical circuit/waveform description per `asset-descriptions.md` for offline Amiga RAG vector search.

---

## 7. Priority 7: Native Vector Extraction & SVG Optimization

### When to Use
High-importance architectural diagrams, block diagrams, logic schematics, and state machines where vector scalability significantly enhances readability in Obsidian:
- System block diagrams (e.g. A500 / B2000 System Architecture).
- Chip functional diagrams (e.g. Fat Agnus internal registers, 8520 CIA functional blocks).
- Bus state and timing diagrams (Write Cycle, Read Cycle).

### Critical Rule: Inspect Source PDF for Native Vectors First
Before attempting to trace a cropped PNG bitmap or manually drawing SVG:
1. **Check for Native Vector Content**:
   - Inspect whether the diagram in the source PDF consists of **native vector drawing commands (paths, lines, polygons, beziers)** and **embedded font text**.
   - Modern digital manuals, FrameMaker exports, and CAD datasheets embed vector primitives natively.
2. **Direct Vector Extraction**:
   - If native vector primitives exist, extract the diagram directly as SVG using tools like `pdf2svg`, `mutool draw -F svg`, or PyMuPDF (`page.get_svg_image()`).
   - Direct extraction preserves mathematical line precision, keeps true selectable text, and avoids fuzzy raster tracing artifacts entirely.
3. **Fallback to Bitmap Tracing**:
   - Only fall back to PNG-to-SVG tracing (or manual redrawing) if the source PDF is verified to be a scanned bitmap scan with no native vector operators.
4. **ViewBox & ClipPath Rules**:
   - Ensure `viewBox` covers all outer signal lines and labels plus a 20px safety buffer.
   - Audit and expand `<clipPath>` to prevent clipped pin numbers or text.
   - Save as clean `.svg` in `assets/`.


---

## 8. Advisory & Callout Blocks (Notes, Warnings, and Errors)

### When to Use
Any bordered note box, shaded sidebar, developer tip, hardware caution, or errata callout in the source manual.

### Rules
- Never leave advisory blocks as unstructured body text or plain `> quote` formatting.
- Map them directly to native Obsidian Callout types:
  - `Note:`, `Notice:`, `Info:` $\rightarrow$ `> [!NOTE]` or `> [!INFO]`
  - `Tip:`, `Hint:`, `Programming Tip:` $\rightarrow$ `> [!TIP]`
  - `Important:`, `Attention:`, `Critical:` $\rightarrow$ `> [!IMPORTANT]`
  - `Warning:`, `Caution:`, `Alert:` $\rightarrow$ `> [!WARNING]` or `> [!CAUTION]`
  - `Error:`, `Danger:`, `Fatal:`, `Bug:` $\rightarrow$ `> [!DANGER]` or `> [!ERROR]`
- Retain 100% of text and parameters. Prefix every line (including blank lines and code blocks) with `>`.


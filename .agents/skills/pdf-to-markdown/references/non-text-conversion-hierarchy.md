# Non-Text & Formatted Content Conversion Hierarchy

When converting PDF pages to Markdown, you will encounter diverse non-prose elements: programming code, register summaries, memory dumps, circuit schematics, pinout diagrams, and bus timing waveforms.

Apply this strict 7-level priority ladder to choose the best representation.

---

## The 7-Level Priority Ladder

```text
Priority 1: Code Block (```c, ```m68k)       -> For programming code; standardize indentations
Priority 2: Markdown Table                   -> For structured tabular data; clean and searchable
Priority 3: Monotone Text Block (```text)    -> For memory dumps, hex bytes, raw data alignment
Priority 4: ASCII Art (Only if Readable)     -> For simple register bitfield diagrams
Priority 5: HTML Table                       -> For complex tables requiring cell spans (colspan/rowspan)
Priority 6: Crop to High-Res PNG             -> For complex schematics, waveforms, pinouts
Priority 7: PNG -> SVG Vectorization         -> For scalable block diagrams, logic, and timing charts
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

---

## 3. Priority 3: Monotone Text Blocks (` ```text `)

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

## 4. Priority 4: ASCII Art (Only if Strictly Readable)

### When to Use
Simple single-line register bitfield allocations or horizontal signal boxes where monospace text is clean, compact, and immediately legible.

### Criteria
- **Must be strictly aligned**: If character widths vary or the diagram wraps on standard screens, do not use ASCII art.
- If it cannot be formatted cleanly in under 80 columns, convert to a Markdown table or crop as an image.

### Example (32-Bit Register Layout)
```text
 31            24 23            16 15             8 7              0
+----------------+----------------+----------------+----------------+
|      HOB       |      MHB       |      MLB       |      LOB       |
+----------------+----------------+----------------+----------------+
```

---

## 5. Priority 5: HTML Tables

### When to Use
When tables require complex merged cells (`colspan` or `rowspan`), multiple text blocks inside a single cell, or styled sub-headers that standard Markdown tables cannot represent.

### Critical Math Rules for HTML Tables
- Do not use standard `$math$` delimiters inside `<td>` tags (Markdown engines do not parse inline math inside HTML blocks).
- Use HTML/Unicode formatting: `2<sup>10</sup>`, `T<sub>CLK</sub>`, `&plusmn;`, `&Omega;`.

---

## 6. Priority 6: Crop to High-Res PNG

### When to Use
Complex visual hardware illustrations that cannot be represented in text:
- Physical IC pin configuration diagrams.
- Analog oscillograms and bus cycle timing diagrams.
- Dense system schematics and PCB connector pinouts.

### Rules
- Render at **150 to 200 DPI**.
- Add a **10-15% safety padding margin** to the bounding box.
- Name canonically: `assets/section_XX_figure_X-Y_<slug>.png`.

---

## 7. Priority 7: PNG $\rightarrow$ SVG Vectorization

### When to Use
High-importance architectural diagrams, block diagrams, and state machines where vector scalability significantly enhances readability in Obsidian:
- System block diagrams (e.g. A500 / B2000 System Architecture).
- Chip functional diagrams (e.g. Fat Agnus internal registers, 8520 CIA functional blocks).
- Bus state and timing diagrams (Write Cycle, Read Cycle).

### Rules
- Ensure `viewBox` covers all outer signal lines and labels plus a 20px buffer.
- Audit and expand `<clipPath>` to prevent clipped text.
- Save as clean `.svg` in `assets/`.

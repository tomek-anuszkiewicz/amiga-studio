# Technical Guidelines: Vision-First Transcription & Conversion Rules

These authoritative formatting rules are adapted from `.agents/skills/pdf-to-markdown/` for the HTML-to-Markdown vision transcription pipeline.

---

## 1. Motorola Hex Address Escaping vs KaTeX Math Corruption

### The Issue
In Motorola 68000 assembly and Amiga hardware documentation, hexadecimal addresses and immediate values use a dollar prefix: `$00000004`, `$DFF000`, `$FFFF`, `$C00000`.
In Markdown and KaTeX, two unescaped dollar signs on the same line or paragraph trigger KaTeX math mode:
```text
The interrupt vector table spans from $00000000 to $00000400 in Chip RAM.
```
KaTeX attempts to parse `00000000 to 00000400` as a mathematical expression, causing broken formatting and rendering errors.

### The Strict Hex Backtick Rule
- **Never leave hexadecimal values as bare `$HEX` in prose.**
- **Always enclose hexadecimal values in inline code backticks**:
  ```markdown
  The interrupt vector table spans from `$00000000` to `$00000400` in Chip RAM.
  Custom chip registers begin at `$DFF000`.
  ```
- Reserve `$math$` exclusively for actual mathematical expressions (e.g. `$2^{32} - 1$`, `$f = \frac{1}{2\pi RC}$`).

---

## 2. Table Conversion Hierarchy

When converting tabular data from rendered page PNGs:

### Priority 1: Standard GitHub-Flavored Markdown (GFM) Tables
- Use for all regular tables with consistent rows and columns:
  - Register lists (`Offset`, `Name`, `Access`, `Description`).
  - Pinout tables (`Pin`, `Signal`, `Type`, `Description`).
  - Instruction classification and timing tables.
- Example:
  ```markdown
  | Address | Register | Access | Description |
  | :--- | :--- | :--- | :--- |
  | `$DFF000` | `BLTCON0` | W | Blitter control register 0 |
  | `$DFF002` | `BLTCON1` | W | Blitter control register 1 |
  ```

### Priority 2: Clean HTML Tables (Complex Spans)
- Used only when complex cell spans (`colspan` or `rowspan`) or multi-line cell structures cannot be represented in GFM.
- **Math Rule for HTML Tables:**
  - CommonMark parsers do **not** evaluate `$math$` delimiters inside `<td>` tags. Writing `<td>$2^{16}$</td>` renders as literal text.
  - Use pure HTML and Unicode entities:
    - $2^{16}$ $\rightarrow$ `2<sup>16</sup>`
    - $T_{CLK}$ $\rightarrow$ `T<sub>CLK</sub>`
    - $\pm 5\%$ $\rightarrow$ `&plusmn;5%`
    - $\Omega$ $\rightarrow$ `&Omega;`
    - $\mu s$ $\rightarrow$ `&mu;s`
    - $\times$ $\rightarrow$ `&times;`

### Priority 3: Monotone Text Blocks (```text)
- For memory hex dumps and raw data alignment where fixed-width characters are mandatory.

---

## 3. Mathematical Equations

- Format genuine mathematical expressions using standard KaTeX delimiters:
  - Inline formulas: `$T_{cycle} = \frac{1}{f_{CCK}}$`
  - Display equations:
    ```markdown
    $$
    t_{access} = (N_{wait} + 2) \times t_{CL}
    $$
    ```

---

## 4. Advisory, Notice, and Alert Blocks (Obsidian Callouts)

Convert all advisory text, boxed notes, and warning markers into native Obsidian callouts:

| Source Document Styling / Marker | Obsidian Callout Target | Semantic Role |
| :--- | :--- | :--- |
| `Note:`, `Notice:`, `Info:` | `> [!NOTE]` or `> [!INFO]` | General technical notes, secondary explanations |
| `Tip:`, `Hint:` | `> [!TIP]` | Performance suggestions, coding tricks |
| `Important:`, `Attention:`, `Prerequisite:` | `> [!IMPORTANT]` | Prerequisites, mandatory register requirements |
| `Warning:`, `Caution:` | `> [!WARNING]` or `> [!CAUTION]` | Hardware risks, bus contention hazards |
| `Error:`, `Danger:`, `Fatal:` | `> [!DANGER]` or `> [!ERROR]` | Destructive operations, CPU exceptions, bus lockups |

### Formatting Rules:
1. Retain 100% of the original text and technical meaning without truncation.
2. Prefix every line of the callout (including blank lines) with `>`.

---

## 5. Image Placeholders & Asset Extraction

During vision transcription, diagrams, schematics, and waveforms must be marked with standardized placeholders:

### Option A: HTML Asset Reference Placeholder
When the source image file exists in the HTML asset directory (e.g. `./document_files/image001.gif`):
```markdown
<image placeholder src="document_files/image001.gif" alt="Figure 1: Instruction Prefetch Mechanism" />
```

### Option B: Bounding-Box Crop Placeholder
When the diagram is a visual element on the rendered page PNG:
```markdown
<crop page="4" xmin="120" ymin="340" xmax="950" ymax="780" label="Figure 1: Instruction Prefetch Mechanism" />
```

### Automated Replacement:
Run `replace_placeholders.py` to:
1. Copy or crop the image into the local `assets/` directory.
2. Author or verify the Git-tracked technical sidecar `<image_path>.txt`.
3. Replace the placeholder tag with standard Markdown: `![alt](assets/filename.png)`.

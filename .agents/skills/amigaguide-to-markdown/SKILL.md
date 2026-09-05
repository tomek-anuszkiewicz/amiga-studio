---
name: amigaguide-to-markdown
description: >-
  Use this skill when converting Commodore AmigaGuide hypertext documents (.guide) into clean, modern Markdown files (.md) optimized for Obsidian and GitHub. Covers node parsing, multi-chapter splitting, tag translation, paragraph unwrapping, ASCII table and diagram formatting, Obsidian TD() heading anchor resolution, and automated link validation.
---

# Recipe: Converting AmigaGuide Documents to Markdown

This skill provides a standardized, battle-tested procedure for converting AmigaGuide (`.guide`) documents into modern, publication-quality Markdown documents optimized for Obsidian vaults and GitHub documentation.

---

## 1. Toolchain & Directory Structure

All conversion scripts and references reside inside this skill directory:

```text
.agents/skills/amigaguide-to-markdown/
├── SKILL.md                               # This workflow recipe
├── scripts/
│   ├── convert_guide.py                   # Automated AmigaGuide parser and Markdown generator
│   ├── validate_links.py                  # Obsidian-exact TD() link & anchor validator
│   └── iff_to_png.py                      # Amiga IFF ILBM graphic to PNG converter
└── references/
    ├── amigaguide-spec.md                 # Complete AmigaGuide command and syntax reference
    └── obsidian-linking-quirks.md         # Obsidian heading anchor normalization and decodeURI rules
```

---

## 2. Step-by-Step Conversion Workflow

Follow these phases sequentially when processing an AmigaGuide file.

### Phase 1: Inspection & Structure Analysis

1. **Inspect Encoding**: AmigaGuide files are standard 8-bit text, almost universally encoded in **ISO-8859-1 (Latin-1)** or Topaz font ASCII. Always read input files using Latin-1 encoding:
   ```python
   raw_text = Path("input.guide").read_text(encoding="latin-1")
   ```
2. **Determine Document Architecture**:
   - **Single Manual / Utility Guide** (e.g. `Kickstart.guide`, `installer.guide`): Typically 5-15 short nodes. Convert as a single consolidated document using `--mode single`.
   - **Comprehensive Reference Manual / Book** (e.g. *Amiga Guru Book*, *680x0 Reference*): Many nodes, organized hierarchically by chapters and numbered sections. Convert as a multi-chapter folder with a top-level `00 - Table of Contents.md` using `--mode split`.
3. **Identify Media Assets**: Check for linked Amiga IFF graphic files (`.iff` or `.ilbm`).

---

### Phase 2: Automated Parsing & Markdown Generation

Run the bundled converter script:

#### Multi-Chapter Split Mode (Default for Books & Reference Manuals):
```bash
python .agents/skills/amigaguide-to-markdown/scripts/convert_guide.py \
  "path/to/document.guide" \
  --output-dir "Obsidian/Amiga/Reference/DocumentName" \
  --mode split
```
- Creates `00 - Table of Contents.md` containing document metadata (`@AUTHOR`, `@$VER:`, `@COPYRIGHT`) and an Obsidian-compatible navigation list.
- Generates numbered chapter files: `01 - <Title>.md`, `02 - <Title>.md`, etc.
- Injects a return footer in each chapter: `[⬅ Return to Table of Contents](00%20-%20Table%20of%20Contents.md)`.

#### Single-Document Consolidation Mode (For Short Guides):
```bash
python .agents/skills/amigaguide-to-markdown/scripts/convert_guide.py \
  "path/to/document.guide" \
  --output-dir "Obsidian/Amiga/Reference" \
  --mode single
```
- Creates a single self-contained `<DocumentName>.md` file with a top-level Table of Contents and internal `#` section jumps.

---

### Phase 3: Layout Modernization & Content Polishing

When reviewing or enhancing the generated Markdown (either programmatically or via an LLM):

1. **Code Blocks & Assembly Listings**:
   - Enclose code in fenced blocks with explicit language identifiers: ```` ```c ````, ```` ```m68k ````, or ```` ```text ````.
2. **ASCII Tables**:
   - Convert space-aligned ASCII tables (especially those with `<u>...</u>` column headers) into standard Markdown tables (`| Col 1 | Col 2 |`).
   - Strip `<u>` and `</u>` tags when creating Markdown table header cells.
   - **Preserve empty cells**: If a row has a blank entry under a column (e.g. no value under the first header), keep the cell empty (`| |`) rather than shifting following columns to the left.
3. **ASCII Art & Diagrams**:
   - Flowcharts, hierarchy charts, and state machines $\rightarrow$ convert to Mermaid (```` ```mermaid ````).
   - Register bit layouts, memory maps, and binary diagrams $\rightarrow$ keep in ```` ```text ```` code blocks to preserve fixed-width character alignment.
4. **Mathematical Expressions**:
   - Use standard LaTeX/KaTeX math (`$2^{10}$`, `$2^{31}-1$`).
   - **CRITICAL**: Do **NOT** treat Motorola hexadecimal values (`$00000004`, `$FF`, `$C00000`) as LaTeX math! Enclose hexadecimal addresses in inline code backticks: `` `$00000004` ``.
5. **List Formatting**:
   - Convert AmigaGuide list bullets (`*` or `**` at start of line) to standard Markdown `-`.
   - Keep footnotes and literal asterisks properly escaped (`\*`).
6. **Heading Punctuation**:
   - Convert trailing colons in numbered headings to periods (e.g. `## 2.1.1: System Addresses` $\rightarrow$ `## 2.1.1. System Addresses`).

---

### Phase 4: Obsidian Link & Heading Anchor Rules

Obsidian does **not** use GitHub kebab-case slugification. Its internal engine normalizes headings using:
`SD = /[!"#$%&()*+,.:;<=>?@^\`{|}~/\\[\\]\\\r\n]/g`
`TD(e) = e.replace(SD, " ").replace(/\s+/g, " ").trim().toLowerCase()`

And crucially, Obsidian resolves links using JavaScript's `decodeURI()`, which **does NOT decode `%2C` (commas) or `%3A` (colons)**.

Always follow these rules when creating or rewriting links:
1. **Literal Punctuation**: Never percent-encode commas, colons, dots, or question marks in anchors.
   - ✅ `...#2.1.3.%20System%20addresses,%20jumps%20into%20ROM,%20and%20private%20data%20structures`
   - ❌ `...#2.1.3.%20System%20addresses%2C%20jumps%20into%20ROM%2C%20and%20private%20data%20structures` *(Breaks in Obsidian!)*
2. **Encoded Spaces**: Encode spaces as `%20`.
3. **Same-Document Links**: Omit the file name completely when linking within the same document:
   - ✅ `[Overview](#1.1.%20Data%20types)`
   - ❌ `[Overview](01%20-%20Data%20Types.md#1.1.%20Data%20types)` *(Causes Obsidian to reload or ignore the anchor)*
4. **Cross-Document Links**: Encode spaces in the target filename and append the anchor:
   - `[Section 17.1.71](17%20-%20Dos%20Functions.md#17.1.71.%20InternalLoadSeg%28%29%20%282.0%29)`

---

### Phase 5: Converting Amiga IFF Graphics to PNG

If the guide references Amiga IFF ILBM graphic files:
1. Run `iff_to_png.py`:
   ```bash
   python .agents/skills/amigaguide-to-markdown/scripts/iff_to_png.py \
     "path/to/image.iff" \
     "Obsidian/Amiga/Reference/DocumentName/assets/image.png"
   ```
2. Reference the converted image in Markdown using standard syntax:
   ```markdown
   ![Figure Title](assets/image.png)
   ```

---

### Phase 6: Automated Verification & Validation

Always run `validate_links.py` as the final quality gate:

```bash
python .agents/skills/amigaguide-to-markdown/scripts/validate_links.py \
  "Obsidian/Amiga/Reference/DocumentName"
```

The validator:
- Verifies that all target files exist.
- Simulates Obsidian's exact JavaScript `decodeURI()` and `TD()` heading lookup algorithm on all `#anchors`.
- Flags broken links, mismatched anchors, and improper same-document file references.
- Target: **100% PASS (0 failures, 0 broken anchors)**.

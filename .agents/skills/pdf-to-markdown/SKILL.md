---
name: pdf-to-markdown
description: >-
  Use this skill when converting technical PDF manuals, books, or documentation into clean, modern Markdown files (.md) optimized for Obsidian and GitHub. Covers PDF bookmark parsing, high-res page rendering, LLM vision transcription, crop asset extraction with safety padding, asset deduplication, SVG vectorization, split table stitching, running header/footer stripping, Obsidian callouts for notes/warnings/errors, Prev/TOC/Next navigation bars, and visual QA double-checks.
---

# Recipe: Converting Technical PDF Manuals to Markdown

This skill provides a standardized, battle-tested procedure for converting complex technical PDF books, architecture manuals, and hardware guides into publication-quality Markdown documents optimized for Obsidian vaults and GitHub documentation.

---

## 1. Toolchain & Directory Structure

All conversion scripts and references reside inside this skill directory:

```text
.agents/skills/pdf-to-markdown/
├── SKILL.md                               # This workflow recipe
├── scripts/
│   ├── pdf_to_pages.py                    # PyMuPDF-based PDF chapter splitter & page-to-PNG renderer
│   ├── extract_crops.py                   # Figure cropper with safety margins, deduplication, & asset naming
│   ├── merge_chapters.py                  # Page-to-chapter compiler, header/footer stripper, & navigation injector
│   ├── png_to_svg_helper.py               # Vectorization helper & SVG viewBox/clipPath safety auditor
│   └── verify_page_vision.py              # Visual double-check tool comparing markdown vs original page PNG
└── references/
    ├── pdf-conversion-pitfalls.md         # Detailed guide to the 11 known conversion pitfalls & fixes
    ├── non-text-conversion-hierarchy.md   # Decision matrix: Code vs Table vs Text Block vs HTML vs PNG vs SVG
    └── table-merging-heuristics.md        # Reference on stitching split tables across page boundaries
```

---

## 2. Cardinal Rule of Conversion

> [!IMPORTANT]
> **Content Fidelity & Meaning:**
> - **You may reformat and polish layout**, typography, indentations, and presentation.
> - **Preserve 100% of the original content and technical meaning.**
> - **Do NOT add new content** (no invented text, commentary, or assumed facts).
> - **Do NOT omit or summarize existing content** (no dropping footnotes, sidebars, or table columns).

---

## 3. Non-Text & Formatted Content Conversion Hierarchy

When encountering diagrams, code, tables, and visual figures in the PDF, apply this strict priority ladder:

1. **Priority 1: Code Blocks (` ```c `, ` ```m68k `, ` ```asm `)**:
   - For all programming code listings.
   - Enforce explicit language tags.
   - Standardize OCR indentations (2 or 4 spaces) and align assembly columns (`Label:    Mnemonic    Operands    ; Comments`).
2. **Priority 2: Standard Markdown Tables**:
   - First choice for structured tabular data: pinout tables, register lists, sector layouts, memory maps.
3. **Priority 3: Monotone Text Blocks (` ```text `)**:
   - Monospace blocks for content requiring strict fixed-width alignment where table syntax is unsuitable:
     - Memory dumps (hex addresses with ASCII sidebar)
     - Interactive terminal / CLI transcripts
     - Raw binary or data packet layouts
4. **Priority 4: ASCII Art (Only if Strictly Readable)**:
   - Monospace diagrams for simple register bitfield layouts *only if strictly aligned and immediately readable*. If unaligned across varying fonts or screen sizes, convert to a Markdown table or cropped image.
5. **Priority 5: HTML Tables**:
   - When complex cell spans (`colspan`, `rowspan`), multi-line cell entries, or nested structures are required.
   - **Math Rule**: Do not use `$math$` inside `<td>` tags; use pure HTML/Unicode (`2<sup>10</sup>`, `T<sub>CLK</sub>`, `&plusmn;`, `&Omega;`).
6. **Priority 6: Crop to High-Res PNG**:
   - For complex physical IC pinouts, oscilloscope waveforms, and dense schematics.
   - Render at 150-200 DPI with a 10-15% safety padding margin.
7. **Priority 7: Native Vector Extraction & SVG Optimization**:
   - For block diagrams, logic schematics, and digital timing charts to ensure crisp, scalable vector graphics in Obsidian.
   - **Check Source PDF First**: Always inspect the original PDF document to verify if **native non-bitmap content (vector paths, Bézier curves, shapes, and font text)** already exists on the page. If present, extract the native vector graphic directly (via tools like `mutool draw -F svg`, `pdf2svg`, or PyMuPDF) rather than lossy bitmap tracing.

---

## 4. The 8-Phase Conversion Pipeline

Follow these phases sequentially when processing a PDF document.

### Phase 1: PDF Analysis & Chapter Mapping

Run `pdf_to_pages.py` to extract bookmarks and map page ranges:

```bash
python .agents/skills/pdf-to-markdown/scripts/pdf_to_pages.py \
  "path/to/manual.pdf" \
  --output-dir "workspace/manual_staging" \
  --dpi 200
```

- Renders each page into high-resolution PNGs (`pages/page_001.png`, etc.).
- Auto-detects chapter page boundaries from PDF bookmarks.
- Automatically excludes obsolete print matter from chapters:
  - Alphabetical Index (digital search replaces it)
  - List of Tables
  - List of Figures
- Produces `manifest.json`.

---

### Phase 2: Page-by-Page LLM Vision Transcription

Transcribe each page PNG (`page_001.png` $\dots$) into a page-level Markdown file (`page_001.md`).

#### LLM Vision Transcription Guidelines:
1. **Full Fidelity**: Transcribe every sentence, table cell, and footnote. Do not summarize.
2. **Motorola Hex Addresses**: **Always enclose in backticks** (`` `$00000004` ``, `` `$DFF000` ``). Never leave bare `$HEX`, as multiple dollar signs corrupt KaTeX math rendering.
3. **Figure Crops**: Mark every diagram, schematic, or visual waveform with a `<crop>` tag:
   ```markdown
   <crop xmin="120" ymin="340" xmax="950" ymax="780" label="Figure 6-2. Fat Agnus Block Diagram" />
   ```
4. **Clean Note Separation**: If a diagram has embedded notes or legends, transcribe them into Markdown callouts below the figure rather than keeping text inside the image:
   ```markdown
   > [!NOTE] DMA Time Slot Notes
   > 1. If divide by zero occurs, an exception occurs.
   ```
5. **Obsidian Callouts for Notes, Warnings, and Errors**:
   - Whenever the source page contains an advisory block (e.g. boxed note, shaded warning, margin caution, or text beginning with `Note:`, `Notice:`, `Warning:`, `Caution:`, `Important:`, `Error:`, `Danger:`), convert it into an Obsidian callout rather than leaving it as plain text or an unstyled blockquote:
     - `Note:` / `Notice:` / `Info:` $\rightarrow$ `> [!NOTE]` or `> [!INFO]`
     - `Tip:` / `Hint:` $\rightarrow$ `> [!TIP]`
     - `Important:` / `Attention:` $\rightarrow$ `> [!IMPORTANT]`
     - `Warning:` / `Caution:` $\rightarrow$ `> [!WARNING]` or `> [!CAUTION]`
     - `Error:` / `Danger:` / `Bug:` $\rightarrow$ `> [!DANGER]` or `> [!ERROR]`
   - Example:
     ```markdown
     > [!WARNING] Bus Contention Risk
     > Never access custom chip registers during DMA cycles without asserting the bus grant signal.
     ```

---

### Phase 3: Asset Extraction & Deduplication

Run `extract_crops.py` to crop figures from page PNGs:

```bash
python .agents/skills/pdf-to-markdown/scripts/extract_crops.py \
  --markdown-dir "workspace/manual_staging/pages_md" \
  --pages-dir "workspace/manual_staging/pages" \
  --assets-dir "Obsidian/Amiga/Reference/ManualName/assets" \
  --padding 15
```

- Adds a 15px safety padding margin to prevent cut-off borders or truncated pin names.
- Deduplicates identical images across chapters using image content hashing.
- Standardizes asset naming: `assets/section_XX_figure_X-Y_<slug>.png`.
- Replaces `<crop>` tags with standard Markdown image links.

---

### Phase 4: Vector Extraction, SVG Vectorization & Boundary Audit

For block diagrams and timing charts:
1. **Audit Source PDF for Native Vectors First**:
   - Check if the source PDF contains native vector primitives and selectable font text rather than a flattened raster scan:
     ```python
     import fitz
     doc = fitz.open("path/to/manual.pdf")
     page = doc[page_num]
     drawings = page.get_drawings()  # Native vector paths
     print(f"Vector paths detected: {len(drawings)}")
     ```
   - **If native vectors exist**: Extract the diagram directly as SVG (e.g. via `mutool draw -F svg`, `pdf2svg`, or PyMuPDF's `page.get_svg_image()`). This yields 100% sharp lines and true selectable text without raster degradation.
   - **If the PDF is purely a scanned bitmap**: Vectorize/trace the cropped high-res PNG.
2. **Boundary & ViewBox Audit**:
   - Audit the SVG `viewBox` and `<clipPath>` using `png_to_svg_helper.py`:
     ```bash
     python .agents/skills/pdf-to-markdown/scripts/png_to_svg_helper.py \
       "Obsidian/Amiga/Reference/ManualName/assets" \
       --padding 20.0 \
       --apply
     ```
   - Ensures `viewBox` has an expanded safety buffer and that outer signal lines, pin labels, and text are not clipped.


---

### Phase 5: Chapter Merging & Header/Footer Stripping

Compile the page-level Markdowns into cohesive chapter files:

```bash
python .agents/skills/pdf-to-markdown/scripts/merge_chapters.py \
  --manifest "workspace/manual_staging/manifest.json" \
  --pages-md-dir "workspace/manual_staging/pages_md" \
  --output-dir "Obsidian/Amiga/Reference/ManualName"
```

- Strips repeating print running headers, running footers, and page numbers.
- Detects and stitches multi-page split tables into unified single tables.
- Injects standardized **Prev | TOC | Next** navigation bars at the top and bottom of every chapter:
  ```markdown
  [⬅ Previous: Section 1 - Summary](01%20-%20Section%201%20Summary.md) | [📑 Table of Contents](00%20-%20Table%20of%20Contents%20and%20Front%20Matter.md) | [Next: Section 3 - Architecture ➡](03%20-%20Section%203%20Architecture.md)
  ---
  ```
- Generates `00 - Table of Contents and Front Matter.md` with hierarchical Obsidian links.

---

### Phase 6: Visual Double-Check & QA Verification

Audit the compiled output using `verify_page_vision.py`:

```bash
python .agents/skills/pdf-to-markdown/scripts/verify_page_vision.py \
  --page-png "workspace/manual_staging/pages/page_042.png" \
  --markdown "workspace/manual_staging/pages_md/page_042.md"
```

Checklist to verify:
- [ ] **No Dropped Text**: All footnotes, sub-bullets, fine print, and sidebars present.
- [ ] **No Cut-Off Graphics**: All 4 borders, IC pin labels, and waveform marks intact.
- [ ] **No Duplicated Notes**: Explanatory text resides in Markdown, not inside image assets.
- [ ] **No Broken Tables**: Split multi-page tables cleanly merged without duplicate headers.
- [ ] **No Bare Hex Addresses**: All `$HEX` enclosed in backticks (` `$000004` `).

---

### Phase 7: Obsidian Navigation & Link Validation

Run `validate_links.py` (from `amigaguide-to-markdown/scripts/`) to confirm 100% link resolution:

```bash
python .agents/skills/amigaguide-to-markdown/scripts/validate_links.py \
  "Obsidian/Amiga/Reference/ManualName"
```

Target: **100% PASS (0 broken files, 0 broken anchors, 0 warnings)**.

---
name: pdf-to-markdown
description: Convert technical reference PDF manuals into modular Obsidian Markdown with extracted figures and link validation.
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
│   ├── extract_crops.py                   # Figure cropper with safety margins, deduplication, & .txt sidecars
│   ├── merge_chapters.py                  # Page-to-chapter compiler, header/footer stripper, & navigation injector
│   ├── png_to_svg_helper.py               # Vectorization helper & SVG viewBox/clipPath safety auditor
│   ├── verify_page_vision.py              # Visual double-check tool comparing markdown vs original page PNG
│   ├── audit_conversion.py                # Automated semantic sanity auditor (catches prose-in-code leaks)
│   └── validate_links.py                  # Anchor, image asset, and link integrity validator
└── references/
    ├── pdf-conversion-pitfalls.md         # Detailed guide to the 17 known conversion pitfalls & fixes
    ├── non-text-conversion-hierarchy.md   # Decision matrix: Code vs Table vs Mermaid vs Text vs Crop vs SVG
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
   - English prose sentences with punctuation must **NEVER** be enclosed in code blocks.
2. **Priority 2: Standard Markdown Tables**:
   - First choice for structured tabular data: pinout tables, register lists, sector layouts, memory maps.
   - **Register Bitfields**: When encountering ASCII register bitfield boxes (`| 15 | 14 | ... | 0 |`), convert them into clean Markdown tables (`| Bit(s) | Name | Function |`).
   - **Dual-Column Parallel Tables**: In printed hardware manuals, reference tables (e.g. Table 6-1 minterms, opcode charts) frequently use a **4-column parallel layout** (`Selected Equation | LF Code | Selected Equation | LF Code`). Strictly preserve the 4-column parallel topology matching the physical book. Never collapse or linearize into a single tall table or invent artificial columns.
   - **LaTeX Carriage Return (`\r`) Invariant**: Never allow `$\rightarrow$` or LaTeX commands with `\r` to be parsed as carriage return control characters (`ASCII 13`), which splits Markdown table rows onto two lines and corrupts table rendering. In tables and prose, prefer Unicode arrows (`A → D` or `A ⇒ D`) or properly escaped strings.
   - **KaTeX Math & Hex Codes**: Use `\overline{...}` for negation overlines ($\overline{A}$, $\overline{B}$) and always enclose hex codes in backticks (`` `$F0` ``) to prevent KaTeX math collision.
3. **Priority 3: Native Diagrams (Mermaid + ASCII Fallback)**:
   - For state machines, flowcharts, block diagrams, pipeline queues, and bus handshakes.
   - Generate a clean native Mermaid flowchart (`flowchart TD` or `sequenceDiagram`).
   - Provide a compact text/ASCII diagram inside a native Obsidian collapsible callout:
     ```markdown
     > [!NOTE]- Click to view Text / ASCII Diagram
     > ```text
     > +-------+     +--------+
     > | State | --> | Next   |
     > +-------+     +--------+
     > ```
     ```
   - **Prohibition of Raw HTML `<details>`**: Raw HTML `<details><summary>` tags break CommonMark formatting and render as raw red tags in Obsidian Live Preview. Always use native Obsidian folded callouts (`> [!NOTE]-`).
   - **No Redundant Images**: **Do NOT embed a raster image if a Mermaid diagram is generated to represent it.** Generating both an image and a Mermaid graph creates redundant visual clutter.
4. **Priority 4: HTML Tables**:
   - When complex cell spans (`colspan`, `rowspan`), multi-line cell entries, or nested structures are required.
   - **Math Rule**: Do not use `$math$` inside `<td>` tags; use pure HTML/Unicode (`2<sup>10</sup>`, `T<sub>CLK</sub>`, `&plusmn;`, `&Omega;`).
5. **Priority 5: Monotone Text Blocks (` ```text `)**:
   - Monospace blocks for content requiring strict fixed-width alignment where table syntax is unsuitable:
     - Memory dumps (hex addresses with ASCII sidebar).
     - Interactive terminal / CLI transcripts.
     - Raw binary or data packet layouts.
6. **Priority 6: ASCII Art (Only if Strictly Readable)**:
   - Monospace diagrams for simple structures *only if strictly aligned and immediately readable within 80 columns*. If unaligned across varying fonts or screen sizes, convert to a Markdown table, Mermaid diagram, or cropped image.
7. **Priority 7: Crop to High-Res PNG + Mandatory Sidecar (`.txt`)**:
   - For complex physical IC pinouts, oscilloscope waveforms, and dense analog schematics that Mermaid cannot model.
   - Render at 150-200 DPI with a 10-15% safety padding margin.
   - **Mandatory Technical Sidecars**: Every cropped image MUST have a companion `.txt` technical sidecar (e.g. `figure_XX_<slug>.png.txt`) containing technical circuit/waveform descriptions per `asset-descriptions.md` for offline Amiga RAG vector search.
8. **Priority 8: Native Vector Extraction & SVG Optimization**:
   - For block diagrams, logic schematics, and digital timing charts to ensure crisp, scalable vector graphics in Obsidian.
   - **Check Source PDF First**: Always inspect the original PDF document to verify if **native non-bitmap content (vector paths, Bézier curves, shapes, and font text)** already exists on the page. If present, extract the native vector graphic directly (via tools like `mutool draw -F svg`, `pdf2svg`, or PyMuPDF) rather than lossy bitmap tracing.

---

## 4. Obsidian Frontmatter, Properties & Tags Standard

All converted technical reference manuals must begin on Line 1 with active YAML frontmatter bounded by `---`:

```yaml
---
title: "Canonical Full Manual Title"
author: "Author Name or Commodore-Amiga, Inc."
source: "Original Publication Name / Manual"
date: "YYYY"
tags:
  - amiga
  - hardware
  - chipset
  - reference
properties:
  author: "Author Name or Commodore-Amiga, Inc."
  source: "Original Publication Name / Manual"
  archive_date: "YYYY"
---
```

---

## 5. The Monolithic PDF Conversion Pipeline

The input is typically **one single monolithic PDF book** (200-600+ pages). The conversion does not require splitting the source PDF into multiple physical PDF files; instead, it utilizes structured page mapping and chapter chunking.

### Phase 1: Structural Discovery & Chapter Mapping (`manifest.json`)

Run `pdf_to_pages.py` to inspect the monolithic PDF:

```bash
python .agents/skills/pdf-to-markdown/scripts/pdf_to_pages.py \
  "path/to/manual.pdf" \
  --output-dir "workspace/manual_staging" \
  --dpi 200
```

- **Bookmark Discovery**: Extracts internal PDF bookmarks (`doc.get_toc()`).
- **Scanned Books Fallback**: If the PDF is a flat scan without bookmarks, inspect the book's printed Table of Contents pages (typically pages 1-15) and provide a custom manifest via `--custom-manifest manifest.json` mapping chapter titles to their `start_page` and `end_page`.
- **Excludes Obsolete Print Matter**:
  - Alphabetical Index (digital search replaces it)
  - List of Tables
  - List of Figures
- Produces `manifest.json` defining all chapters, page ranges, and metadata.

---

### Phase 2: High-Resolution Page Rendering

`pdf_to_pages.py` automatically renders all pages (or a specified `--start-page` / `--end-page` range) into 200 DPI PNGs in `workspace/manual_staging/pages/` (`page_001.png`, `page_002.png`, etc.).

---

### Phase 3: Chapter-Level Multimodal Vision Transcription

Rather than transcribing single disconnected pages that fragment paragraphs, the LLM transcribes by **chapter chunks** (e.g. pages 15-32):

1. **Continuous Context**: The LLM reads the page sequence for that chapter, unifying paragraphs split across page boundaries.
2. **Table Continuity**: Tables spanning multiple pages are consolidated into a single Markdown table, stripping redundant repeated print headers.
3. **Motorola Hex Addresses**: **Always enclose in backticks** (`` `$00000004` ``, `` `$DFF000` ``) to prevent KaTeX math rendering collisions.
4. **Figure Placeholders**: Mark diagrams that cannot be modeled in Mermaid with `<crop>` tags:
   ```markdown
   <crop page="17" xmin="120" ymin="340" xmax="950" ymax="780" label="Figure 6-2. Fat Agnus Timing Waveform" />
   ```
5. **Obsidian Callouts**: Convert note boxes, tips, cautions, and warnings into native callouts (`> [!NOTE]`, `> [!WARNING]`, `> [!IMPORTANT]`).

---

### Phase 4: Asset Extraction, Deduplication & Sidecars

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
- Standardizes asset naming: `assets/figure_XX_<slug>.png`.
- Replaces `<crop>` tags with standard Markdown image links.
- **Generates `.txt` Technical Sidecars**: For every extracted asset, creates a companion `assets/figure_XX_<slug>.png.txt` file per `asset-descriptions.md` for offline Amiga RAG vector indexing.

---

### Phase 5: Vector Extraction, SVG Vectorization & Boundary Audit

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

### Phase 6: Chapter Assembly & Navigation Injection

Compile the transcribed Markdowns into cohesive chapter files:

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

### Phase 7: Automated Sanity Audit

Run `audit_conversion.py` to catch prose-in-code leaks, unbackticked hex addresses, and fragmented blocks:

```bash
python .agents/skills/pdf-to-markdown/scripts/audit_conversion.py \
  "Obsidian/Amiga/Reference/ManualName"
```

Checklist to verify:
- [ ] **Zero Prose Leaks**: Normal English text is not mistakenly placed inside code blocks.
- [ ] **Zero Bare Hex Addresses**: All `$HEX` addresses are backticked (`` `$DFF000` ``) to prevent KaTeX math collision.
- [ ] **No Fragmented Fences**: Adjacent code fences are unified.

---

### Phase 8: Link Integrity & Asset Audit

Run `validate_links.py` to confirm 100% link resolution:

```bash
python .agents/skills/pdf-to-markdown/scripts/validate_links.py \
  "Obsidian/Amiga/Reference/ManualName"
```

Target: **100% PASS (0 broken files, 0 broken anchors, 0 warnings)**.

---

## Execution Mode: Subagent Delegation

- **Execution Host:** **Isolated Subagent** (child context sandbox).
- **Model Tier:** `Gemini Flash (Multimodal Vision)`
- **Context Savings:** Absorbs 50,000+ multimodal vision tokens, 200 DPI raster page PNGs, figure bounding box coordinates, and multi-step chapter stitching from the main conversation.
- **Subagent Task Template:**
  - `TaskName`: "PDF Conversion: <manual_name>"
  - `TaskSummary`: "Executes 7-phase multimodal PDF transcription into Obsidian markdown with figure crops and link validation."
  - `Prompt`:
    ```markdown
    Convert reference manual PDF: <PDF_PATH> into Obsidian Markdown under `Obsidian/Amiga/Reference/<MANUAL_NAME>/`.
    Follow .agents/skills/pdf-to-markdown/SKILL.md:
    1. Phase 1: Render 200 DPI pages with `pdf_to_pages.py`.
    2. Phase 2: Page-by-page LLM vision transcription (`page_XXX.png` -> `page_XXX.md`). Enclose all hex in backticks (`$HEX`).
    3. Phase 3: Extract figure crops with `extract_crops.py`.
    4. Phase 4: Vectorize bounding boxes with `png_to_svg_helper.py`.
    5. Phase 5: Merge chapters and stitch tables with `merge_chapters.py`.
    6. Phase 6: Run visual QA audits with `verify_page_vision.py`.
    7. Phase 7: Validate links with `validate_links.py`.
    8. Return strictly the PDF Conversion Report below.
    ```
- **Return Contract (Mandatory Structured Output):**
  The subagent must conclude with this exact markdown block:
  ```markdown
  ### 📄 PDF Reference Manual Conversion Report
  - **Manual Name:** `<manual_name>`
  - **Destination Path:** [`Obsidian/Amiga/Reference/<manual_name>/`](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Reference/<manual_name>/)
  - **Pages Converted:** `<total_pages>` pages across `<total_chapters>` chapters
  - **Assets Extracted:** `<num_png>` cropped PNG figures, `<num_svg>` vectorized SVGs
  - **Page Confidence & Anomaly Table:**
    | Chapter / Page | Quality / Complexity | Notes / Handled Elements |
    | :--- | :--- | :--- |
    | Chapter 4 / Page 82 | Complex Waveform | Transcribed split timing table, cropped Fig 4-3 |
  - **Link Integrity:** `validate_links.py` 100% PASS (0 broken files, 0 broken anchors).
  ```

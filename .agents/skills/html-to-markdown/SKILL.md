---
name: html-to-markdown
description: Convert legacy Word HTML, web documentation, and technical HTML articles into clean, publication-grade Obsidian Markdown using LLM transcription, asset downloading, and link validation.
---

# Recipe: Converting Technical HTML Documents to Markdown

This skill provides a standardized, vision-and-LLM-driven workflow for converting arbitrary HTML documents (Microsoft Word exports, vintage 1990s web pages, multi-page technical manuals) into publication-grade Markdown optimized for Obsidian vaults and GitHub documentation.

> [!IMPORTANT]
> **No Mechanical Conversion Scripts for Text:**
> Arbitrary HTML documents vary wildly in layout tables, non-standard CSS, and nested tags. Writing deterministic DOM parsing scripts for text conversion is fragile and unsustainable.
> 
> **The LLM-Driven Pipeline:**
> 1. Download/extract all referenced images to `assets/` and create technical sidecars.
> 2. Pass the HTML source (or rendered page PNGs) directly to the LLM with the unified instruction prompt in `references/llm-transcription-prompt.md`.
> 3. Resolve image placeholders and audit links.

---

## 1. Toolchain & Directory Structure

All conversion scripts and references reside inside this skill directory:

```text
.agents/skills/html-to-markdown/
├── SKILL.md                               # This workflow recipe
├── scripts/
│   ├── download_assets.py                 # Scans HTML, copies/downloads images to assets/, creates .txt sidecars
│   ├── html_to_pages.py                   # Headless Chrome HTML-to-PDF & PyMuPDF page PNG rasterizer
│   ├── replace_placeholders.py            # Resolves <image placeholder> & <crop> tags, formats Markdown links
│   ├── render_comparison.py               # Headless browser side-by-side visual comparison renderer
│   ├── audit_conversion.py                # Automated semantic sanity auditor (catches prose-in-code leaks)
│   ├── diff_reference.py                  # Structural AST comparator against reference ground truth
│   └── validate_links.py                  # Anchor, image asset, and link integrity validator
└── references/
    ├── llm-transcription-prompt.md        # Comprehensive LLM prompt (Cardinal rules, Non-text hierarchy, TOC rules)
    ├── pdf-to-markdown-guidelines.md      # KaTeX hex escaping, table hierarchy, callouts & placeholder rules
    ├── html-sanitization-heuristics.md     # Reference on MSO Word HTML quirks, CP1252, and entity cleanup
    └── non-text-conversion-hierarchy.md   # Priority ladder: Assembly blocks, GFM tables, diagrams
```

---

## 2. Cardinal Rule of Conversion

> [!IMPORTANT]
> **Content Fidelity & Meaning:**
> - **You may reformat and polish layout**, typography, indentations, and presentation.
> - **Preserve 100% of the original content and technical meaning.**
> - **Do NOT add new content** (no invented text, commentary, or unverified claims).
> - **Do NOT omit or summarize existing content** (no dropping footnotes, technical sidebars, or instruction tables).

---

## 3. Non-Text & Formatted Content Conversion Hierarchy

When converting formatted elements, follow the strict priority ladder in `references/llm-transcription-prompt.md`:

1. **Priority 1: Code Blocks (` ```m68k `, ` ```assembly `, ` ```c `)**:
   - For all programming listings, opcode sequences, and memory maps.
   - Enforce explicit language tags (e.g. `assembly` or `m68k`).
   - Cleanly align assembly columns (`Label:    Mnemonic    Operands    ; Comment`).
   - Regular English prose sentences with punctuation must **NEVER** be enclosed in code blocks.
2. **Priority 2: Standard GitHub-Flavored Markdown (GFM) Tables**:
   - First choice for structured tabular data: register breakdowns, bus cycle sequences, instruction classification tables, and timing specifications.
3. **Priority 3: HTML Tables (Complex Spans)**:
   - Used only when complex cell spans (`colspan` or `rowspan`) or multi-line cell entries cannot be represented in GFM.
   - **Math Rule:** CommonMark parsers do not evaluate `$math$` inside `<td>`. Use pure HTML/Unicode (`2<sup>16</sup>`, `&plusmn;`, `&Omega;`).
4. **Priority 4: Monotone Text Blocks (` ```text `)**:
   - For memory hex dumps and raw byte alignment where fixed-width spacing is mandatory.
5. **Priority 5: ASCII Art (Only if Strictly Readable)**:
   - For simple register bitfield layouts *only if strictly aligned and immediately readable*. If unaligned across varying fonts or screen sizes, convert to a Markdown table or cropped image.
6. **Priority 6: Image Placeholders & Asset References**:
   - Insert standardized placeholders for diagrams, circuit schematics, or register drawings:
     - For existing HTML assets: `<image placeholder src="folder/image.gif" alt="Figure Description" />`
     - For visual page regions: `<crop page="N" xmin="X1" ymin="Y1" xmax="X2" ymax="Y2" label="Figure Description" />`

---

## 4. Table of Contents (TOC) Architecture

The LLM must construct a clean, hierarchical Table of Contents placed immediately after the document title and metadata:

- **Heading:** `## Contents` (or `## Table of Contents`).
- **Hierarchy:** Nested bullet list matching heading depths (`##` -> `- [Title](#anchor)`, `###` -> `  - [Subtitle](#anchor)`).
- **Anchor Format:** Lowercase, hyphens for spaces, punctuation stripped.
- **100% Resolution:** Every TOC item must resolve to an exact heading in the body.

---

## 5. Operational Workflow

### Phase 1: Download & Prepare Assets
Run `download_assets.py` to extract, copy, or download all images referenced in the HTML document to `assets/` and generate technical `.txt` sidecars:
```powershell
python .agents/skills/html-to-markdown/scripts/download_assets.py `
  --html "Obsidian/Amiga/Reference/temp/DocFolder/doc.html" `
  --assets-dir "Obsidian/Amiga/Reference/temp/html-sandbox/assets"
```
*(Optional)* If visual page layout inspection is needed, render high-res page PNGs:
```powershell
python .agents/skills/html-to-markdown/scripts/html_to_pages.py `
  --input "Obsidian/Amiga/Reference/temp/DocFolder/doc.html" `
  --output-dir "Obsidian/Amiga/Reference/temp/html-sandbox/pages" `
  --resolution 200
```

### Phase 2: LLM Transcription
Instruct the LLM to perform the conversion by providing:
1. The full prompt instructions from [`references/llm-transcription-prompt.md`](references/llm-transcription-prompt.md).
2. The HTML source text (or page PNGs).

The LLM will produce clean Markdown with:
- Backticked Motorola hex values (`` `$00000004` ``) to protect KaTeX math rendering.
- GFM tables for structured data.
- Hierarchical TOC with exact matching anchors.
- Native Obsidian callouts (`> [!NOTE]`, `> [!WARNING]`, `> [!IMPORTANT]`).
- Image placeholders (`<image placeholder ...>` or `<crop ...>`).

### Phase 3: Resolve Image Placeholders
Run `replace_placeholders.py` to verify assets, update image links, and guarantee sidecars:
```powershell
python .agents/skills/html-to-markdown/scripts/replace_placeholders.py `
  --markdown "Obsidian/Amiga/Reference/temp/html-sandbox/doc.md" `
  --assets-dir "Obsidian/Amiga/Reference/temp/html-sandbox/assets" `
  --source-assets-dir "Obsidian/Amiga/Reference/temp/DocFolder" `
  --pages-dir "Obsidian/Amiga/Reference/temp/html-sandbox/pages"
```

### Phase 4: Headless Visual Comparison
Generate a side-by-side visual comparison between the original HTML rendering and the converted Markdown:
```powershell
python .agents/skills/html-to-markdown/scripts/render_comparison.py `
  --html "Obsidian/Amiga/Reference/temp/DocFolder/doc.html" `
  --markdown "Obsidian/Amiga/Reference/temp/html-sandbox/doc.md" `
  --output-dir "Obsidian/Amiga/Reference/temp/html-sandbox"
```
Open `visual_comparison.png` to visually audit typography, alignment, and diagram placement.

### Phase 5: Link Integrity & Asset Audit
Run `validate_links.py` to verify 100% link, anchor, and image resolution:
```powershell
python .agents/skills/html-to-markdown/scripts/validate_links.py `
  "Obsidian/Amiga/Reference/temp/html-sandbox/doc.md"
```
Target: **100% PASS (0 broken files, 0 broken anchors, 0 warnings)**.

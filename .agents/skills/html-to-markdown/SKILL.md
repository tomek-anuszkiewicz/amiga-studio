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
    └── llm-transcription-prompt.md        # Unified LLM prompt (Obsidian frontmatter, Multi-page crawl, Non-text hierarchy, TOC rules)
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
3. **Priority 3: Native Diagrams (Mermaid + ASCII Fallback)**:
   - For pipeline flows, queue architectures, bus handshakes, block diagrams, and state machines.
   - Generate a clean native Mermaid flowchart (e.g. `flowchart TD` or `sequenceDiagram`).
   - Provide a compact text/ASCII diagram inside `<details><summary>Click to view Text / ASCII Diagram</summary>...</details>`.
   - **No Redundant Images:** **Do NOT embed a raster image if a Mermaid diagram is generated to represent it.** Generating both an image and a Mermaid graph creates redundant visual clutter.
4. **Priority 4: HTML Tables (Complex Spans)**:
   - Used only when complex cell spans (`colspan` or `rowspan`) or multi-line cell entries cannot be represented in GFM.
   - **Math Rule:** CommonMark parsers do not evaluate `$math$` inside `<td>`. Use pure HTML/Unicode (`2<sup>16</sup>`, `&plusmn;`, `&Omega;`).
5. **Priority 5: Monotone Text Blocks (` ```text `)**:
   - For memory hex dumps and raw byte alignment where fixed-width spacing is mandatory.
6. **Priority 6: ASCII Art (Only if Strictly Readable)**:
   - For simple register bitfield layouts *only if strictly aligned and immediately readable*. If unaligned across varying fonts or screen sizes, convert to a Markdown table or cropped image.
7. **Priority 7: Image Placeholders & Asset References (When Mermaid Cannot Model)**:
   - Only for complex analog waveforms, physical IC chip pinouts, dense schematics, or photographs:
     - For existing HTML assets: `<image placeholder src="folder/image.gif" alt="Figure Description" />`
     - For visual page regions: `<crop page="N" xmin="X1" ymin="Y1" xmax="X2" ymax="Y2" label="Figure Description" />`

---

## 4. Obsidian Frontmatter, Properties & Tags Standard

All converted technical reference documents are targeted for Obsidian vaults and GitHub documentation. Transcriptions must begin on Line 1 with active YAML frontmatter bounded by `---`:

```yaml
---
title: "Canonical Full Document Title"
author: "Author Name or Handle"
source: "https://canonical-url.org/article"
original_site: "Original Publication or Website Name"
date: "YYYY"
tags:
  - amiga
  - hardware
  - chipset
  - reference
properties:
  author: "Author Name or Handle"
  source: "https://canonical-url.org/article"
  original_site: "Original Publication or Website Name"
  archive_date: "YYYY"
---
```

- **Tags:** Lowercase, domain-specific classification tags (e.g. `amiga`, `hardware`, `chipset`, `m68000`, `copper`, `sprites`, `denise`, `agnus`, `paula`, `reference`).
- **Properties Dictionary:** Mirrored key attributes under `properties:` for Obsidian Dataview and property graph evaluation.

---

## 5. Multi-Page HTML Crawl Consolidation Protocol

When the source document consists of multiple crawled HTML files (e.g. `index.html` plus linked chapter pages like `Chapter_1.html`, `What_is_this_all_about.html`, etc.):

1. **Topological Discovery & Canonical Sequencing:**
   - Inspect `index.html` (the site Table of Contents) or follow `[Next]` / `[Previous]` navigation links to determine the authoritative sequence of chapters.
2. **Prune Recurring Web Chrome & Boilerplate:**
   - Strip site navigation bars (`[Contents]`, `[Next]`, `[Previous]`, `[Home]`), header banners, breadcrumbs, and recurring site footer badges (copyright icons, webcounters, host branding).
   - Only preserve actual technical prose, code listings, tables, and notes.
3. **Convert Cross-Page Hyperlinks to Local Anchors:**
   - Remap inter-page links (e.g. `<a href="Copper.html#regchanges">` or `href="CD32_Controller.html"`) into document-local Markdown anchors (e.g. `[More register changes in a scanline](#21-more-register-changes-in-a-scanline)`).
   - Ensure every converted internal link points to a valid heading anchor in the unified document.
4. **Unified Document Hierarchy:**
   - Assemble all chapters under a single top-level H1 (`# Title`).
   - Map each individual HTML page to a sequential H2 (`## 1. Chapter Name`, `## 2. Chapter Name`) or H3 for subsections, preserving clear hierarchical numbering.
5. **Unified Table of Contents:**
   - Synthesize a comprehensive 2-level Table of Contents at the top of the document indexing all consolidated chapters and subsections.

---

## 6. Table of Contents (TOC) Architecture

The LLM must construct a clean, hierarchical Table of Contents placed immediately after the document title and metadata:

- **Heading:** `## Contents` (or `## Table of Contents`).
- **Hierarchy:** Nested bullet list matching heading depths:
  - Top-level (`##`): `- [[#Title|Title]]`
  - Subsections (`###`): `  - [[#Subtitle|Subtitle]]`
  - Sub-subsections (`####`): `    - [[#Topic|Topic]]`
- **Obsidian Heading Anchor Standard (No Broken Slugs):**
  - Standard GitHub kebab-case slugs (e.g. `[Title](#title-slug)`) **do not work in Obsidian** because Obsidian does not use kebab-case slugification; it resolves headings by exact title string.
  - In Obsidian vaults, use native Wikilink heading references:
    `- [[#Heading Title|Heading Title]]` (or `[[#Heading Title]]`).
  - Alternatively, if standard Markdown links are strictly required, use URL-encoded exact titles:
    `- [Heading Title](#Heading%20Title)`.
- **100% Resolution:** Every TOC item must resolve to an exact heading in the body.

---

## 7. Operational Workflow

### Phase 1: Download & Prepare Assets
Run `download_assets.py` to extract, copy, or download all images referenced in the HTML document (or all HTML files in a crawl directory) to `assets/` and generate technical `.txt` sidecars:
```powershell
# For single HTML file:
python .agents/skills/html-to-markdown/scripts/download_assets.py `
  --html "Obsidian/Amiga/Reference/temp/DocFolder/doc.html" `
  --assets-dir "Obsidian/Amiga/Reference/temp/html-sandbox/assets"

# For multi-page HTML directory crawl:
Get-ChildItem "Obsidian/Amiga/Reference/temp/DocFolder/*.html" | ForEach-Object {
  python .agents/skills/html-to-markdown/scripts/download_assets.py `
    --html $_.FullName `
    --assets-dir "Obsidian/Amiga/Reference/temp/html-sandbox/assets"
}
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
2. The HTML source text (or page PNGs). For multi-page crawls, follow **Section 5 (Multi-Page HTML Crawl Consolidation Protocol)** to consolidate chapters sequentially into a single reference document.

The LLM will produce clean Markdown with:
- Line 1 Obsidian Properties (YAML frontmatter with tags and metadata).
- Backticked Motorola hex values (`` `$00000004` ``) to protect KaTeX math rendering.
- GFM tables for structured data.
- Hierarchical 2-level TOC with exact matching anchors.
- Native Obsidian callouts (`> [!NOTE]`, `> [!WARNING]`, `> [!IMPORTANT]`).
- Image placeholders (`<image placeholder ...>` or `<crop ...>`) when Mermaid cannot model the graphic.

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

---
name: html-to-markdown
description: Convert legacy Word HTML, web documentation, and technical HTML articles into clean, publication-grade Obsidian Markdown using vision-first page rendering, placeholder replacement, and link validation.
---

# Recipe: Converting Technical HTML Documents to Markdown

This skill provides a standardized, vision-first workflow for converting arbitrary HTML documents (Microsoft Word exports, vintage 1990s web pages, multi-page technical manuals) into publication-grade Markdown optimized for Obsidian vaults and GitHub documentation.

Because arbitrary HTML variations (nested layout tables, proprietary MSO classes, broken spans) make direct DOM-to-Markdown scripting fragile, this skill uses a **vision-first pipeline**:
1. Normalizes HTML into high-resolution page PNGs using headless Chrome.
2. Employs LLM Multimodal Vision for full-fidelity transcription with image placeholders.
3. Resolves and replaces image placeholders automatically via script.
4. Audits visual layout and link integrity.

---

## 1. Toolchain & Directory Structure

All conversion scripts and references reside inside this skill directory:

```text
.agents/skills/html-to-markdown/
├── SKILL.md                               # This workflow recipe
├── scripts/
│   ├── html_to_pages.py                   # Headless Chrome HTML-to-PDF & PyMuPDF page PNG rasterizer
│   ├── replace_placeholders.py            # Resolves <image placeholder> & <crop> tags, extracts assets & sidecars
│   ├── render_comparison.py               # Headless browser side-by-side visual comparison renderer
│   ├── audit_conversion.py                # Automated semantic sanity auditor (catches prose-in-code leaks)
│   ├── diff_reference.py                  # Structural AST comparator against reference ground truth
│   └── validate_links.py                  # Anchor, image asset, and link integrity validator
└── references/
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
> - **Do NOT omit or summarize existing content** (no dropping footnotes, technical sidebars, or instruction classes).

---

## 3. Formatting & Vision Transcription Guidelines

When performing multimodal vision transcription on rendered page PNGs, follow the guidelines in `references/pdf-to-markdown-guidelines.md`:

1. **Motorola Hex in Backticks:**
   - **Always enclose hexadecimal addresses and constants in code backticks**: `` `$00000004` ``, `` `$DFF000` ``.
   - Never leave bare `$HEX` in prose, as consecutive dollar signs corrupt KaTeX math rendering.
2. **Tables:**
   - Transcribe structured tabular data into standard GitHub-Flavored Markdown (GFM) tables.
   - For complex tables requiring merged cells (`colspan`/`rowspan`), use clean HTML tables with Unicode entities (`2<sup>16</sup>`, `&plusmn;`, `&Omega;`). Never use `$math$` inside `<td>`.
3. **Advisory Blocks (Obsidian Callouts):**
   - Convert `Note:`, `Notice:`, `Info:` into `> [!NOTE]`.
   - Convert `Tip:`, `Hint:` into `> [!TIP]`.
   - Convert `Important:`, `Warning:`, `Caution:` into `> [!IMPORTANT]` or `> [!WARNING]`.
4. **Code Listings:**
   - Enforce explicit language tags (e.g. ```` ```m68k ```` or ```` ```assembly ````).
   - Align assembly columns: `Label:    Mnemonic    Operands    ; Comments`.
5. **Image Placeholders:**
   - Insert standardized placeholders for diagrams, circuit schematics, or register drawings:
     - For existing HTML assets: `<image placeholder src="folder/image.gif" alt="Figure Description" />`
     - For visual page regions: `<crop page="N" xmin="X1" ymin="Y1" xmax="X2" ymax="Y2" label="Figure Description" />`

---

## 4. Operational Workflow

### Phase 1: Render HTML to High-Resolution Page PNGs
Run `html_to_pages.py` to print the HTML document to PDF and rasterize pages at 200 DPI:
```powershell
python .agents/skills/html-to-markdown/scripts/html_to_pages.py `
  --input "Obsidian/Amiga/Reference/temp/DocFolder/doc.html" `
  --output-dir "Obsidian/Amiga/Reference/temp/html-sandbox/pages" `
  --resolution 200
```
This produces `pages/page_001.png`, `pages/page_002.png`, etc., and `manifest.json`.

### Phase 2: Page-by-Page LLM Vision Transcription
Transcribe each page PNG into Markdown following Section 3 guidelines:
- Ensure all Motorola hex values are backticked.
- Format tables as GFM.
- Insert `<image placeholder ... />` tags for diagrams.
- Merge the transcribed pages into the target Markdown document (e.g. `doc.md`).

### Phase 3: Replace Image Placeholders & Author Sidecars
Run `replace_placeholders.py` to resolve placeholders, copy/crop image assets to `assets/`, create Git-tracked `.txt` technical sidecars, and replace tags with canonical links:
```powershell
python .agents/skills/html-to-markdown/scripts/replace_placeholders.py `
  --markdown "Obsidian/Amiga/Reference/temp/html-sandbox/doc.md" `
  --assets-dir "Obsidian/Amiga/Reference/temp/html-sandbox/assets" `
  --source-assets-dir "Obsidian/Amiga/Reference/temp/DocFolder" `
  --pages-dir "Obsidian/Amiga/Reference/temp/html-sandbox/pages"
```

### Phase 4: Headless Visual Comparison & Quality Gate
Generate a side-by-side visual comparison between the original HTML rendering and the converted Markdown:
```powershell
python .agents/skills/html-to-markdown/scripts/render_comparison.py `
  --html "Obsidian/Amiga/Reference/temp/DocFolder/doc.html" `
  --markdown "Obsidian/Amiga/Reference/temp/html-sandbox/doc.md" `
  --output-dir "Obsidian/Amiga/Reference/temp/html-sandbox"
```
- Open `visual_comparison.png` to audit layout alignment, section headings, and diagram placement.
- Run `audit_conversion.py` to ensure no prose text leaked into code blocks.

### Phase 5: Link Integrity & Asset Audit
Run `validate_links.py` to verify that all image assets, anchors, and references resolve cleanly:
```powershell
python .agents/skills/html-to-markdown/scripts/validate_links.py `
  "Obsidian/Amiga/Reference/temp/html-sandbox/doc.md"
```
Target: **100% PASS (0 broken files, 0 broken anchors, 0 warnings)**.

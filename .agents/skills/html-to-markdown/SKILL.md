---
name: html-to-markdown
description: Convert legacy Word HTML, web documentation, and technical HTML articles into clean, publication-grade Obsidian Markdown with asset extraction and link validation.
---

# Recipe: Converting Technical HTML Documents to Markdown

This skill provides a standardized, automated procedure for converting legacy HTML exports (such as Microsoft Word HTML), vintage technical articles, and multi-page web documentation into publication-quality Markdown documents optimized for Obsidian vaults and GitHub documentation.

---

## 1. Toolchain & Directory Structure

All conversion scripts and references reside inside this skill directory:

```text
.agents/skills/html-to-markdown/
├── SKILL.md                               # This workflow recipe
├── scripts/
│   ├── convert_html.py                    # Main HTML-to-Markdown parser, sanitizer & compiler
│   ├── diff_reference.py                  # Structural AST & metric comparator against ground truth
│   └── validate_links.py                  # Anchor, image asset, and link integrity validator
└── references/
    ├── html-sanitization-heuristics.md     # Handling MSO Word HTML quirks, CP1252, and entity cleanup
    └── non-text-conversion-hierarchy.md   # Priority ladder: Assembly blocks, GFM tables, Mermaid diagrams
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

## 3. Non-Text & Formatted Content Conversion Hierarchy

When encountering diagrams, code, tables, and visual figures in HTML documents, apply this strict priority ladder:

1. **Priority 1: Code Blocks (` ```assembly `, ` ```c `, ` ```text `)**:
   - For all programming listings, opcode sequences, and memory maps.
   - Enforce explicit language tags (e.g. `assembly` for M68000).
   - Standardize indentations (aligned columns: `Label:  Mnemonic  Operands  ; Comment`).
2. **Priority 2: Standard Markdown Tables**:
   - First choice for structured tabular data: register breakdowns, bus cycle sequences, instruction classification tables, and timing specifications.
   - Layout-only tables (e.g. single-cell wrapping tables used purely for image centering) must be unwrapped.
3. **Priority 3: Diagrams (Mermaid + ASCII Fallback)**:
   - For pipeline flows, bus handshakes, state machines, and register block diagrams.
   - Provide clean Mermaid flowcharts (`flowchart TD` or `sequenceDiagram`).
   - Where helpful, provide a text/ASCII diagram inside a `<details>` block.
4. **Priority 4: Image Assets & Sidecars**:
   - For complex circuit schematics and non-trivial figures.
   - Download or copy image files to an adjacent `assets/` or `_files/` directory.
   - Author a mandatory Git-tracked sidecar description file (`<image_path>.txt`) per `.agents/rules/asset-descriptions.md`.

---

## 4. Operational Workflow

### Step 1: Analyze the HTML Source
Inspect the raw HTML for character encoding, generator signatures, and asset dependencies:
```powershell
python .agents/skills/html-to-markdown/scripts/convert_html.py --inspect "path/to/document.html"
```
Look for:
- Word/Office signatures (`xmlns:v`, `xmlns:o`, `ProgId: Word.Document`).
- Character encodings (`windows-1252`, `iso-8859-1`, `utf-8`).
- Referenced image assets (`<img>` tags, `./*_files/`).

### Step 2: Fetch Missing Assets & Author Sidecars
Ensure referenced image assets are saved locally. For each raster asset (`.png`, `.gif`, `.jpg`), generate the Git-tracked technical sidecar `<image_path>.txt`.

### Step 3: Run Baseline Conversion into Sandbox
Always run initial conversions into an isolated sandbox folder to protect existing documentation:
```powershell
python .agents/skills/html-to-markdown/scripts/convert_html.py `
  --input "Obsidian/Amiga/Reference/temp/DocFolder/doc.html" `
  --output "Obsidian/Amiga/Reference/temp/html-sandbox/doc.md" `
  --title "Document Title" `
  --author "Author Name" `
  --source-url "http://example.com/doc.html" `
  --toc
```

### Step 4: Compare Against Ground Truth Reference
If a verified reference exists, run the structural diff tool:
```powershell
python .agents/skills/html-to-markdown/scripts/diff_reference.py `
  "Obsidian/Amiga/Reference/temp/html-sandbox/doc.md" `
  "Obsidian/Amiga/Reference/doc.md"
```
Audit:
- Heading hierarchy (H1, H2, H3)
- Table cell retention
- Code block count and formatting
- Character encoding integrity (no `\ufffd` or missing quotes)

### Step 5: Validate Links and Assets
```powershell
python .agents/skills/html-to-markdown/scripts/validate_links.py `
  "Obsidian/Amiga/Reference/temp/html-sandbox/doc.md"
```

### Step 6: Review & Finalize
Once validated, the resulting document meets all Obsidian vault standards, including Line 1 YAML properties and dual-layer linking.

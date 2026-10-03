---
name: pdf-stage18-prepare-chapters
description: >-
  Stage 18: Deterministically bundles per-page Markdown documents into chapter drafts in build/02_detect_cont_chapters/ with <continuation-marker> between consecutive pages for Stage 19 inference.
---

# Stage 18: Prepare Chapters (Deterministic Bundling)

This skill executes the deterministic chapter preparation stage. It reads chapter boundary metadata from `build/<stem>_chapters.json` (Stage 4) and gathers per-page Markdown documents (`build/01_page_layout/page_XXXX-proofread.md` from Stage 17, falling back to `page_XXXX-embed.md` from Stage 16).

It concatenates pages chapter-by-chapter, inserting a dedicated continuation marker on its own line between pages:
```markdown
<continuation-marker>
```
The resulting draft chapter files are saved into:
`build/02_detect_cont_chapters/<filename>` (e.g. `build/02_detect_cont_chapters/03 - Section 1 - Overview.md`)

## Purpose & Scope
- **Deterministic Concatenation:** Groups pages belonging to each chapter into a single structured draft without altering content.
- **Explicit Page Boundary Demarcation:** Places `<continuation-marker>` on its own line between consecutive pages to guide Stage 19 LLM inference.
- **Asset Path Normalization:** Adjusts image and table asset links (`](../01_page_layout/assets/...)`) so assets render correctly from the `build/02_detect_cont_chapters/` directory.
- **Authoritative Filename Adoption:** Directly consumes the clean `filename` attribute from `_chapters.json` (`f"{order:02d} - {clean_title}.md"`).
- **Fast Execution:** Executes deterministically across all chapters in milliseconds.

## Input & Output
- **Input (Read-Only):**
  - `<manual_dir>/build/<stem>_chapters.json` (Chapter TOC map with clean `filename` attributes from Stage 4)
  - `build/01_page_layout/page_XXXX-proofread.md` (Stage 17 proofread per-page Markdown, or fallback `page_XXXX-embed.md` from Stage 16)
- **Output:**
  - `build/02_detect_cont_chapters/<filename>` (Draft chapter files with `<continuation-marker>`)

---

## How to Execute

### 1. Prepare All Chapters
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage18-prepare-chapters/scripts/stage18_prepare_chapters.py "path/to/manual"
```

### 2. Prepare Single Chapter
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage18-prepare-chapters/scripts/stage18_prepare_chapters.py "path/to/manual" --chapter 3
```

### 3. Check Status
```powershell
python .agents/plugins/pdf-pipeline/skills/pdf-stage18-prepare-chapters/scripts/stage18_prepare_chapters.py "path/to/manual" --status
```

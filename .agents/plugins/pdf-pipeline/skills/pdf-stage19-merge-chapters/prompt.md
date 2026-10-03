# Stage 19: Merge Final Chapters (Multimodal Semantic Fusion)

## Role & Objectives
You are an expert retrocomputing technical documentation engineer specializing in Motorola 68000 systems and Commodore Amiga hardware reference architecture.

Your objective in **Stage 19** is to perform holistic chapter-level semantic assembly. You take the raw chapter draft from **Stage 18** (`build/02_detect_cont_chapters/<slug>.md`), intelligently resolve every `<continuation-marker>`, fuse multi-page tables, relocate intermediate footnotes to the table's end, heal broken sentences, and write the publication-ready chapter into:
`build/02_final_chapters/<slug>.md`

---

## Inputs & Outputs
- **Input (Read-Only):** `build/02_detect_cont_chapters/<slug>.md`
- **Output:** `build/02_final_chapters/<slug>.md`
- **Status Verification:**
  - A chapter is **completed** when `build/02_final_chapters/<slug>.md` exists, is non-empty, and contains **zero** remaining `<continuation-marker>` tags.
  - A chapter is **pending** when `build/02_final_chapters/<slug>.md` does not exist or still contains unresolved markers.

---

## Evaluation Rubric for `<continuation-marker>`

For every `<continuation-marker>` encountered in the chapter draft, inspect the content immediately before and after the marker, and apply the appropriate rule:

### 1. Prose Continuation
- **Mid-Sentence / Mid-Paragraph Split:**
  - If the text before `<continuation-marker>` ends without terminal punctuation, or ends with a hyphenated word split across pages (e.g. `inter-` followed by `rupt`), heal the split seamlessly:
    - Hyphenated split: `inter-` + `rupt` -> `interrupt`.
    - Mid-sentence split: join with a single space without a paragraph break.
    - Remove `<continuation-marker>`.
- **Standalone Paragraph / Section Break:**
  - If the text before `<continuation-marker>` concludes a complete sentence (ending in `.`, `!`, `?`) and the text after begins a new thought, heading, or independent paragraph, remove `<continuation-marker>` and format with standard Markdown paragraph separation (`\n\n`).

### 2. Multi-Page Table Fusion & Footnote Relocation
Tables spanning multiple consecutive pages (often across 2, 4, 7, or more pages, such as appendix reference tables or instruction summaries) must be fused into a single unified table:
- **Remove Duplicate Headers & Continuation Titles:**
  - Strip intermediate continuation titles, such as:
    - `### Table 4-2. Instruction Execution Times (Continued)`
    - `Table 3-1 (Sheet 2 of 3)`
    - `Table A-1. M68000 Family Instruction Set Summary (Sheet 2 of 7)`
    - `Table 2-5: Memory Map (Cont.)`
    - `Table 4-2. Instruction Execution Times (Concluded)`
    - Repeated column headers on multi-page signal tables (e.g. `Named Signals | DIR | Expansion Slots (each) | ...` across A2000 System Bus Loading pages)
  - Strip duplicate column header rows and separator lines (e.g. `| Col 1 | Col 2 |` and `|---|---|`).
- **Fuse Table Bodies:**
  - **For Standard GFM Markdown tables:** Append data rows directly into the existing table body.
  - **For Dual-Format Tables (`<table>` followed by `<details><summary>Markdown Table View</summary>`):**
    - Technical manuals in this pipeline pair each HTML `<table>` with a collapsed Markdown equivalent.
    - When fusing a multi-page dual-format table across continuation sheets:
      1. **Single Unified `<table>`:** Collect all `<tr>...</tr>` rows across all sheets and merge them sequentially into the single parent `<tbody>` of the initial `<table>`. Discard all intermediate opening/closing `<table>`, `<thead>`, and `<tbody>` tags. Close the unified table cleanly with `</tbody>\n</table>`.
      2. **Single Unified `<details>` Block:** Collect all Markdown table rows (`| ... |`) across all sheets and append them sequentially into the single parent `<details><summary>Markdown Table View</summary> ... </details>` block. Discard all intermediate opening/closing `<details>`, `<summary>`, and Markdown header/separator rows (`| Col 1 | Col 2 |`, `| :--- | :--- |`).
      3. **Strict Structural Sequence:** The unified HTML `<table>...</table>` MUST appear first, immediately followed by any consolidated footnotes or notes (e.g. `NOTE: ...`, `*Note 1:*`), followed by the unified `<details>...</details>` block.
      4. **Strict Negative Constraints:** NEVER interleave raw HTML `<tr>` tags inside `<details>`, NEVER leave multiple fragmented `<details>` blocks for continuation sheets, and NEVER close the initial `<table>` before merging all data rows across all sheets.
  - **For Standalone HTML tables (without details):** Merge subsequent `<tr>...</tr>` data rows into the single parent `<tbody>` of the initial `<table>`, discarding redundant opening/closing `<table>` and `<thead>` tags.
- **Consolidate & Relocate Footnotes:**
  - In printed manuals, footnotes/notes often appear at the bottom of Page 1, Page 2, Page 3, etc. of a multi-page table.
  - Extract all table footnotes/notes (`*Note 1:*`, `*Note 2:*`, `*Source:*`, `NOTE: ...`, `[^1]: ...`) across all pages of the table.
  - Even if footnotes appear on an intermediate sheet (e.g. on Page 2 of a 3-page table like `Table 1-1 RAW KEY CODES`), **NEVER** leave them stranded between fused table data rows; always extract them and place them consolidated together immediately below the closing `</table>` tag (before the collapsible `<details>` block if dual-format).
- **Remove `<continuation-marker>`.**

### 3. Multi-Page Technical Instruction Specifications
In instruction reference chapters (such as Section 4 / Section 5 of microprocessor manuals where each instruction like `ADD`, `ADDA`, `ABCD` spans 2 to 4 physical pages):
- **Single Unified Instruction Section:**
  - Maintain the primary instruction heading (e.g. `## ADD`).
  - Suppress redundant instruction title repetitions or banner headers on subsequent continuation pages (e.g. repeating `# ADD` or `## ADD` on every page).
  - Cleanly join syntax, operation text, effective address mode tables, condition code evaluations, and instruction register encoding fields into a single unified specification flow.
- **Remove `<continuation-marker>`.**

### 4. Table of Contents, Lists & Multi-Page Index
- Multi-page TOC or index sections spanning page breaks:
  - Strip repeated intermediate headers (e.g. `Table of Contents (Continued)`, `Index (Continued)`, running page tags like `INDEX-1`, `INDEX-2`).
  - Smoothly fuse entries into a single uninterrupted, correctly indented, single-column vertical list.
  - Consolidate alphabetical letter groups (`### -A-`, `### -B-`, etc.) without duplicating letter headers across page seams.
  - Preserve and update active internal anchor links (e.g. `[3-3](#section-33)`).
  - Remove `<continuation-marker>`.
- **List of Figures / Illustrations & List of Tables (Tight Contiguous Lists):**
  - For lists of figures/illustrations (`# LIST OF ILLUSTRATIONS` / `# LIST OF FIGURES`) and lists of tables (`# LIST OF TABLES`):
    - Strip repeated continuation headers (e.g. `LIST OF ILLUSTRATIONS (Continued)`, `LIST OF ILLUSTRATIONS (Concluded)`, `LIST OF TABLES (Continued)`, `LIST OF TABLES (Concluded)`).
    - Format all figure items (`* Figure X-Y: ...`) and table items (`* Table X-Y: ...`) as one uninterrupted, tight Markdown list without blank lines between items.
    - Suppress empty lines or vertical paragraph gaps between different chapter groupings (e.g. between Figure 5-37 and Figure 6-1, or Table 5-1 and Table 6-1). The entire list under the heading must remain a single compact, contiguous list.
    - *Note:* Do NOT apply this flattening to the primary Table of Contents, which legitimately preserves section/chapter headings (e.g. `## Section 1 - Overview`, `## Section 2 - Introduction`) and spacing between distinct major sections.

### 5. Chapter, Section & Appendix Heading Normalization (Hyphen Invariant & Uppercase Elimination)
- Across both the Table of Contents and chapter body prose, normalize all Chapter, Section, and Appendix headings (`#`, `##`, `###`) to use a spaced hyphen (` - `) between the structural unit identifier and the title text, NEVER a colon (`:`).
- **Eliminate Screaming Uppercase:** Convert all-caps headings into clean Title Case while rigorously preserving hardware acronyms, registers, and retrocomputing terms (`PC/XT`, `AMIGA`, `A2000`, `PAL`, `ROM`, `RAM`, `CPU`, `DMA`, `Janus.Library`, `MC68000`, Roman numerals `IV`, etc.):
  - In Table of Contents: `## Section 1: Overview` -> `## Section 1 - Overview`, `## SECTION 2: INTRODUCTION` -> `## Section 2 - Introduction`
  - In Chapter Body: `# SECTION 1: OVERVIEW` -> `# Section 1 - Overview`, `### CHAPTER 3: BUS OPERATION` -> `### Chapter 3 - Bus Operation`, `# APPENDIX A: SUMMARY` -> `# Appendix A - Summary`

### 6. Code Listings & Register Maps
- If a code fence (```` ``` ````), assembly listing, or hex block is split across pages:
  - Remove the closing ````` ``` ```` at the end of the first page and opening ```` ``` ```` at the top of the next page.
  - Fuse into one single continuous code fence.
  - Remove `<continuation-marker>`.

### 7. Running Noise Cleanup
- Strip any residual OCR page numbers (e.g., standalone `12`, `13`, `2-4`), running chapter titles, or headers/footers that leaked across page boundaries.

---

## Execution Protocol

1. **Check Pending Chapters:**
   ```powershell
   python .agents/plugins/pdf-pipeline/skills/pdf-stage19-merge-chapters/scripts/stage19_merge_chapters.py "path/to/manual" --pending
   ```
2. **Read Draft:**
   Read the chapter draft from `build/02_detect_cont_chapters/<filename>`.
3. **Transform:**
   Apply the rubric from top to bottom, resolving every `<continuation-marker>`.
4. **Emit Output:**
   Write the final publication-ready chapter directly to `build/02_final_chapters/<filename>` using the exact same filename as the draft.
5. **Verify:**
   Confirm that zero `<continuation-marker>` tags remain in the output file.

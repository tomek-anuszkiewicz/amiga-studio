---
name: pdf-stage19-merge-chapters
description: >-
  Stage 19: Multimodal semantic fusion of chapter drafts into publication-ready chapter documents resolving continuation markers, tables, and footnotes.
---

# Stage 19: Merge Final Chapters

This skill governs the final semantic assembly and chapter fusion of technical manual documentation.

It takes raw chapter drafts from Stage 18 (`build/02_detect_cont_chapters/<filename>`) containing `<continuation-marker>` delimiters, resolves every marker, heals split prose, fuses multi-page tables (Markdown or HTML), consolidates scattered footnotes to the end of tables, and compiles clean, publication-ready chapters in:
`build/02_final_chapters/<filename>` (e.g. `build/02_final_chapters/03 - Section 1 - Overview.md`)

## Purpose & Scope
- **Semantic Continuation Resolution:**
  - Evaluates each `<continuation-marker>` in context.
  - Fuses interrupted sentences, heals hyphenated word splits (`inter-` + `rupt` -> `interrupt`), or preserves clean paragraph breaks.
- **Multi-Page Table Consolidation:**
  - Fuses multi-page tables (GFM Markdown or dual-format semantic HTML `<table>` paired with collapsed `<details>` blocks) into a single continuous table.
  - Strips intermediate headers (`Table X. Title (Continued)`, `Table X. Title (Concluded)`, `Sheet 2 of 3`).
  - Strips repeated table column headers.
  - Gathers all footnotes/notes from all pages of the table and positions them consolidated immediately below the unified table (before any collapsible `<details>` block).
- **TOC & Index Hierarchy:**
  - Fuses multi-page Table of Contents and alphabetical indexes into seamless continuous structures.
- **Heading Normalization:**
  - Enforces spaced hyphens (` - `) and eliminates screaming uppercase in top-level headings.
- **Running Artifact Removal:**
  - Strips leftover running headers, footers, and OCR page numbering.

## Input & Output
- **Input (Read-Only):**
  - `build/02_detect_cont_chapters/<filename>` (Draft chapter documents with `<continuation-marker>`)
- **Output:**
  - `build/02_final_chapters/<filename>` (Publication-ready continuous chapter documents)

## CLI Verification & Helper Tool

Inspect chapter assembly status or deterministically verify finalized chapters on disk:
```bash
# Check overall chapter assembly progress
python .agents/plugins/pdf-pipeline/skills/pdf-stage19-merge-chapters/scripts/stage19_merge_chapters.py "<manual_dir>" --status

# List pending chapters requiring agent assembly
python .agents/plugins/pdf-pipeline/skills/pdf-stage19-merge-chapters/scripts/stage19_merge_chapters.py "<manual_dir>" --pending

# Deterministically verify that specific chapters exist, are non-empty, and contain 0 continuation markers (used by Stage Lead)
python .agents/plugins/pdf-pipeline/skills/pdf-stage19-merge-chapters/scripts/stage19_merge_chapters.py "<manual_dir>" --verify-chapters "<chapter1.md>" "<chapter2.md>"
```

---

## Execution Protocol

All prompt instructions and continuation rubrics are defined in:
👉 **[`prompt.md`](./prompt.md)**

For each assigned chapter in the payload:
1. **Read Draft Chapter:** Read `build/02_detect_cont_chapters/<slug>.md`.
2. **Execute Semantic Chapter Fusion:**
   - Following [`prompt.md`](./prompt.md), resolve every `<continuation-marker>`.
   - Heal broken sentences and hyphenated words across page boundaries.
   - Fuse multi-page tables (GFM Markdown or semantic HTML `<table>`) into single unified tables, stripping redundant headers and consolidating footnotes to the end.
   - Clean up running headers, footers, and page numbers.
3. **Save Publication-Ready Chapter:** Call `write_to_file` to write clean Markdown directly to `build/02_final_chapters/<slug>.md`.
4. **Completion Contract:** When the chapter fusion is complete, terminate and return the compact JSON status contract:
   ```json
   {
     "status": "completed",
     "stage": 19,
     "chapter_id": "<CHAPTER_ID>",
     "slug": "<slug>",
     "output_file": "build/02_final_chapters/<slug>.md"
   }
   ```
   Do not echo chapter content in the return message.

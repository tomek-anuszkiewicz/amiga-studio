---
name: pdf-to-markdown
description: Convert technical PDF manuals and reference books into publication-grade Obsidian Markdown using a modular 12-stage stream-based pipeline.
---

# Recipe: Modular PDF-to-Markdown Conversion Pipeline

This skill converts complex technical PDF documents (such as Amiga hardware reference manuals, hardware schematics, and Motorola 68000 PRMs) into publication-grade Obsidian Markdown.

It is architected around a decoupled **node-stream abstraction**:
`raw_stream.json` $\to$ `reduced_stream.json` $\to$ `chapters/*.json` $\to$ continuation detection $\to$ specialized workers (tables, graphics, prose) $\to$ Markdown emission $\to$ fuzzy TOC linking $\to$ title refinement.

---

## 1. Directory Structure

```text
.agents/skills/pdf-to-markdown/
├── SKILL.md                                 # This workflow manual
├── pipeline.py                              # Master CLI orchestrator managing stage execution
├── config.yaml                              # Global configuration (DPI, paths, LLM model settings)
└── stages/
    ├── 01_preprocess/
    │   ├── preprocess.py                    # Splits PDF -> page_XXXX.pdf, 300 DPI PNG, text blocks JSON
    │   └── README.md
    │
    ├── 02_page_segmentation/
    │   ├── segment_page.py                  # Vertical banding analysis -> page_XXXX_segments.json
    │   ├── prompt.md                        # Vision LLM prompt: header, footer, heading, prose, code_block, table, graphic, toc, toc_header
    │   └── README.md
    │
    ├── 03_build_raw_stream/
    │   ├── build_stream.py                  # Merges segmented pages into global raw_stream.json
    │   ├── extract_initial_assets.py        # Extracts SVG/PNG clips (10% margin) + raw text asset per table/graphic
    │   └── README.md
    │
    ├── 04_stream_reduction/
    │   ├── reduce_stream.py                 # Normalizes stream: suppresses headers/footers, fuses prose
    │   ├── prompt_seam.md                   # Lightweight LLM prompt for de-hyphenation & paragraph continuation
    │   └── README.md
    │
    ├── 05_chapter_partition/
    │   ├── partition_chapters.py            # Splits reduced stream into {idx:02d}_{slug}.json; prepends preamble to file 01
    │   └── README.md
    │
    ├── 06_detect_continuations/
    │   ├── detect_continuations.py          # Scans adjacent table/graphic blocks; links multi-page continuations in JSON
    │   ├── prompt_continuation.md           # LLM prompt: evaluates column headers, row flow, or diagram continuation
    │   └── README.md
    │
    ├── 07_transform_tables/
    │   ├── transform_tables.py              # Worker for tables (consumes continuation groups -> unified GFM/HTML table)
    │   ├── prompt_markdown_table.md         # LLM prompt: 4-column layout, Unicode arrows, math
    │   ├── prompt_html_table.md             # LLM prompt: colspan/rowspan tables
    │   └── README.md
    │
    ├── 08_transform_graphics/
    │   ├── transform_graphics.py            # Worker for graphics: Mermaid + ASCII callout vs SVG + RAG sidecars
    │   ├── prompt_mermaid.md                # LLM prompt: state machines & flowcharts -> Mermaid + ASCII callout
    │   ├── prompt_rag_sidecar.md            # LLM prompt: detailed signal/timing breakdown for RAG (.png.txt)
    │   └── README.md
    │
    ├── 09_transform_prose/
    │   ├── format_prose.py                  # Worker for prose, code_block, and toc (wraps TOC in TOC34534 delimiters)
    │   ├── prompt.md                        # LLM prompt for clean structural Markdown formatting
    │   └── README.md
    │
    ├── 10_emit_markdown/
    │   ├── emit_markdown.py                 # Emits one .md file per partition ({index:02d}_{slug}.md); ignores toc_header
    │   └── README.md
    │
    ├── 11_link_toc/
    │   ├── link_toc.py                      # Fuzzy header matcher across all .md files; converts TOC lines to wikilinks; strips markers
    │   └── README.md
    │
    └── 12_refine_first_chapter_name/
        ├── refine_name.py                   # LLM worker: inspects first chapter content & current name to determine canonical title/slug
        ├── prompt.md                        # LLM prompt: suggests clean chapter title and filename slug (e.g. Table of Contents)
        └── README.md
```

---

## 2. Cardinal Execution Principles

1. **Independent Subfolder Stages**: Every step in `stages/` is fully runnable in isolation using Python CLI arguments.
2. **Intermediate Data Contracts**:
   - `workspace/pages/`: Atomic single-page vector PDFs, 300 DPI PNGs, and text geometry JSONs.
   - `workspace/segments/`: Visual zone nodes with explicit bounding boxes and types.
   - `workspace/raw_stream.json`: Flat, sequentially ordered stream with pre-extracted asset paths (`.svg`, `.png`, `.txt`).
   - `workspace/reduced_stream.json`: Welded paragraphs, de-hyphenated text, and stripped running headers/footers.
   - `workspace/chapters/*.json`: Partitioned section streams prefixed numerically (`00_...json`, `01_...json`).
3. **Table & Diagram Triage**:
   - Simple tables $\to$ GFM tables (4-column book layouts, Unicode arrows `→`).
   - Complex tables $\to$ Semantic HTML `<table>` with `colspan`/`rowspan`.
   - Diagrams & flowcharts $\to$ Mermaid + collapsible ASCII callout (`> [!NOTE]-`).
   - Complex schematics $\to$ High-res SVG/PNG embed + engineering sidecar (`.png.txt`) for offline vector RAG search.
4. **TOC Delimitation & Fuzzy Linking**:
   - Stage 09 encloses the TOC in `<!-- TOC34534 -->` and `<!-- /TOC34534 -->`.
   - Stage 10 ignores `toc_header` banners and emits Markdown files.
   - Stage 11 parses the `TOC34534` block, fuzzy matches each line against all headers in the output directory, transforms entries into Obsidian wikilinks (`[[02_the_copper#Copper Registers|Copper Registers]]`), and cleans up the markers.
5. **Canonical Front-Matter Renaming**:
   - Stage 12 inspects the first emitted Markdown file and uses an LLM to assign the canonical title and slug (e.g. `00_table_of_contents.md`).

---

## 3. CLI Usage

### Running End-to-End
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --pdf "path/to/manual.pdf" --output-dir "Obsidian/Amiga/Reference/Manual"
```

### Running Specific Stages
```powershell
# Run only Stage 01 (Preprocess)
python .agents/skills/pdf-to-markdown/pipeline.py --pdf "manual.pdf" --stage 01

# Run Stages 06 through 08 (Continuations & Transformations)
python .agents/skills/pdf-to-markdown/pipeline.py --pdf "manual.pdf" --from-stage 06 --to-stage 08

# Resume from the last incomplete stage
python .agents/skills/pdf-to-markdown/pipeline.py --pdf "manual.pdf" --resume
```

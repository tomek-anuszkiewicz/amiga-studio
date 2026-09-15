---
name: pdf-to-markdown
description: Convert technical PDF manuals and reference books into publication-grade Obsidian Markdown using a modular 12-stage stream-based pipeline driven by the Agent.
---

# Recipe: Modular PDF-to-Markdown Conversion Pipeline (Agent-Driven)

This skill converts complex technical PDF documents (such as Amiga hardware reference manuals, hardware schematics, and Motorola 68000 PRMs) into publication-grade Obsidian Markdown.

It is architected around an **Agent-Driven Hybrid Model**:
- **Deterministic Python Scripts** handle mechanical tasks (page extraction, 300 DPI rendering, text geometry, asset slicing with 10% margins, stream stitching, chapter partitioning, Markdown emission, and TOC link resolution).
- **The Agent** acts as the cognitive engine and orchestrator (segmentation validation, multi-page continuation reasoning, table formatting, flowchart-to-Mermaid transcription, RAG sidecar authorship, prose polish, and opening title refinement), eliminating any requirement for external API keys (`GEMINI_API_KEY`).

---

## 1. Directory Structure

```text
.agents/skills/pdf-to-markdown/
├── SKILL.md                                 # This workflow manual
├── pipeline.py                              # Master CLI orchestrator & task manager
├── config.yaml                              # Global configuration (DPI, paths, heuristics)
└── stages/
    ├── 01_preprocess/
    │   ├── preprocess.py                    # Splits PDF -> page_XXXX.pdf, 300 DPI PNG, text blocks JSON
    │   └── README.md
    │
    ├── 02_page_segmentation/
    │   ├── segment_page.py                  # Vertical banding analysis -> page_XXXX_segments.json
    │   ├── prompt.md                        # Vision guidelines: header, footer, heading, prose, code_block, table, graphic, toc, toc_header
    │   └── README.md
    │
    ├── 03_build_raw_stream/
    │   ├── build_stream.py                  # Merges segmented pages into global raw_stream.json
    │   ├── extract_initial_assets.py        # Extracts SVG/PNG clips (10% margin) + raw text asset per table/graphic
    │   └── README.md
    │
    ├── 04_stream_reduction/
    │   ├── reduce_stream.py                 # Normalizes stream: suppresses headers/footers, fuses prose
    │   ├── prompt_seam.md                   # De-hyphenation & paragraph continuation guidelines
    │   └── README.md
    │
    ├── 05_chapter_partition/
    │   ├── partition_chapters.py            # Splits reduced stream into {idx:02d}_{slug}.json; partitions front matter into 00_toc
    │   └── README.md
    │
    ├── 06_detect_continuations/
    │   ├── detect_continuations.py          # Scans adjacent table/graphic blocks; prepares & applies continuation groups
    │   ├── prompt_continuation.md           # Continuation evaluation rules
    │   └── README.md
    │
    ├── 07_transform_tables/
    │   ├── transform_tables.py              # Worker for tables (prepares workspace/tasks/tables/ & applies back)
    │   ├── prompt_markdown_table.md         # 4-column layout, Unicode arrows, math
    │   ├── prompt_html_table.md             # Colspan/rowspan tables
    │   └── README.md
    │
    ├── 08_transform_graphics/
    │   ├── transform_graphics.py            # Worker for graphics (prepares workspace/tasks/graphics/ & applies back)
    │   ├── prompt_mermaid.md                # Flowcharts & state machines -> Mermaid + ASCII callout
    │   ├── prompt_rag_sidecar.md            # Technical signal/timing breakdown for RAG (.png.txt)
    │   └── README.md
    │
    ├── 09_transform_prose/
    │   ├── format_prose.py                  # Worker for prose, code_block, and toc (wraps TOC in TOC34534 delimiters)
    │   ├── prompt.md                        # Structural Markdown formatting rules
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
        ├── refine_name.py                   # Inspects first chapter content & sets canonical title/slug
        ├── prompt.md                        # Evaluation guidelines for opening sections
        └── README.md
```

---

## 2. Agent Execution Workflow

When running a conversion task, the Agent executes the pipeline through 5 distinct phases:

### Phase A: Ingestion & Mechanical Stream Building (Stages 01 – 05)
Run the deterministic pipeline steps:
```powershell
# 1. Preprocess PDF (optionally limit pages with --max-pages for testing)
python .agents/skills/pdf-to-markdown/stages/01_preprocess/preprocess.py --pdf "<PATH_TO_PDF>" --workspace workspace

# 2. Generate initial segments
python .agents/skills/pdf-to-markdown/stages/02_page_segmentation/segment_page.py --workspace workspace

# 3. Build raw stream and extract assets (SVG/PNG with 10% margin and raw text files)
python .agents/skills/pdf-to-markdown/stages/03_build_raw_stream/build_stream.py --workspace workspace
python .agents/skills/pdf-to-markdown/stages/03_build_raw_stream/extract_initial_assets.py --workspace workspace

# 4. Stream reduction (suppress headers/footers, weld prose, de-hyphenate)
python .agents/skills/pdf-to-markdown/stages/04_stream_reduction/reduce_stream.py --workspace workspace

# 5. Chapter partition (splits into chapters, partitions front matter into 00_toc.json)
python .agents/skills/pdf-to-markdown/stages/05_chapter_partition/partition_chapters.py --workspace workspace
```

### Phase B: Continuation Verification (Stage 06)
```powershell
# Prepare continuation candidates for review
python .agents/skills/pdf-to-markdown/pipeline.py --prepare-stage 06
```
1. Inspect `workspace/tasks/continuations/candidates.json` using `view_file`.
2. Confirm or adjust `is_continuation: true/false`.
3. Apply confirmed continuations back to chapter streams:
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --apply-stage 06
```

### Phase C: Cognitive Transformations (Stages 07 – 09)

#### 1. Tables (Stage 07)
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --prepare-stage 07
```
- For each `{node_id}.json` in `workspace/tasks/tables/`:
  - Inspect `raw_text` and image preview (`png_path`).
  - Edit or refine the table in `workspace/tasks/tables/{node_id}.md` (GFM or semantic HTML table).
- Apply tables:
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --apply-stage 07
```

#### 2. Graphics (Stage 08)
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --prepare-stage 08
```
- For each `{node_id}.json` in `workspace/tasks/graphics/`:
  - View image using `view_file` on `png_path`.
  - If it is a flowchart/state machine, write a Mermaid diagram with collapsible ASCII callout in `{node_id}.md`.
  - If it is a schematic/timing diagram, author a comprehensive technical description in `{node_id}.sidecar.txt` for RAG vector search.
- Apply graphics:
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --apply-stage 08
```

#### 3. Prose & TOC Delimiters (Stage 09)
```powershell
# Run heuristic prose formatter (or use --prepare / --apply for manual inspection)
python .agents/skills/pdf-to-markdown/stages/09_transform_prose/format_prose.py --workspace workspace
```

### Phase D: Emission & TOC Wikilinking (Stages 10 – 11)
```powershell
# 10. Emit Markdown per chapter (suppressing toc_header)
python .agents/skills/pdf-to-markdown/stages/10_emit_markdown/emit_markdown.py --workspace workspace --output-dir "<OUTPUT_DIR>"

# 11. Cross-file fuzzy TOC linking (converts TOC34534 to Obsidian wikilinks and removes delimiters)
python .agents/skills/pdf-to-markdown/stages/11_link_toc/link_toc.py --output-dir "<OUTPUT_DIR>"
```

### Phase E: Opening Section Title Refinement (Stage 12)
```powershell
# Inspect opening chapter preview and suggested title
python .agents/skills/pdf-to-markdown/stages/12_refine_first_chapter_name/refine_name.py --output-dir "<OUTPUT_DIR>" --inspect

# Apply canonical title and slug (e.g. Table of Contents)
python .agents/skills/pdf-to-markdown/stages/12_refine_first_chapter_name/refine_name.py --output-dir "<OUTPUT_DIR>" --title "Table of Contents" --slug "table_of_contents"
```

---

## 3. Monitoring & Status Check
Check pipeline progress and pending tasks at any time:
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --status
```


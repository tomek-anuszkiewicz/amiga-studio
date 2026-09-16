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
    │   ├── prompt.md                        # Vision guidelines: header, footer, chapter, heading, prose, code_block, table, graphic, toc, toc_header
    │   └── README.md
    │
    ├── 03_build_raw_stream/
    │   ├── build_stream.py                  # Merges segmented pages into global raw_stream.json
    │   ├── extract_initial_assets.py        # Extracts SVG/PNG clips (10% margin) + raw text asset per table/graphic
    │   └── README.md
    │
    ├── 04_stream_reduction/
    │   ├── reduce_stream.py                 # Normalizes stream: suppresses headers/footers, unifies contiguous graphics, fuses prose
    │   ├── prompt_seam.md                   # De-hyphenation & paragraph continuation guidelines
    │   ├── prompt_graphics_union.md         # Vision validation for contiguous graphic fragment union
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
    ├── 10_proofread_stream/
    │   ├── proofread_stream.py              # Proofreads manifest titles, slugs & streams with LLM
    │   ├── prompt.md                        # Technical proofreading guidelines (strict anti-hallucination rules)
    │   └── README.md
    │
    ├── 11_emit_markdown/
    │   ├── emit_markdown.py                 # Emits one .md file per partition ({index:02d}_{slug}.md); ignores toc_header
    │   └── README.md
    │
    ├── 12_refine_first_chapter_name/
    │   ├── refine_name.py                   # Inspects first chapter content & sets canonical title/slug
    │   ├── prompt.md                        # Evaluation guidelines for opening sections
    │   └── README.md
    │
    └── 13_link_toc/
        ├── link_toc.py                      # Fuzzy header matcher across all .md files; converts TOC lines to wikilinks; strips markers
        └── README.md
```

---

## 2. Core Execution Invariant: Master Orchestrator Mandate

> [!IMPORTANT]
> **NEVER invoke stage worker scripts directly** (e.g. `python stages/01_preprocess/preprocess.py` or `python stages/02_page_segmentation/segment_page.py`).
>
> **ALL execution MUST go through `pipeline.py`**.
> Directly executing sub-scripts bypasses:
> 1. Status progression tracking in `stage_status.json`
> 2. LLM call metrics logging in `.metrics.json`
> 3. Automatic downstream stage invalidation and cache cleanup
> 4. Standard argument and path normalization (`--workspace`, `--output-dir`, `--config`)
>
> ### Execution Rules:
> - **Full pipeline**: `python .agents/skills/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>"`
> - **Stage interval**: `python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --from-stage 03 --to-stage 05`
> - **Single stage**: Set `--from-stage` and `--to-stage` to the exact same stage number:
>   `python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --from-stage 02 --to-stage 02`
> - **Stage 01 with specific pages**:
>   `python .agents/skills/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --page-ranges "1-5, 7, 8, 10-15" --from-stage 01 --to-stage 01`
> - **Deterministic batch (01, 03, 04, 05, 10, 11)**:
>   `python .agents/skills/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --run-deterministic`
> - **Cognitive review stages (06, 07, 08, 09)**:
>   - Prepare task items: `python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --prepare-stage 07`
>   - Apply edited task items: `python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --apply-stage 07`

---

## 3. Agent Execution Workflow

When running a conversion task, the Agent executes the pipeline through 6 distinct phases via `pipeline.py`:

### Phase A: Ingestion & Mechanical Stream Building (Stages 01 – 05)
Run deterministic ingestion through the orchestrator:
```powershell
# Run Stages 01 to 05:
python .agents/skills/pdf-to-markdown/pipeline.py --pdf "<PATH_TO_PDF>" --workspace "<WORKSPACE>" --from-stage 01 --to-stage 05

# Or test a single stage / specific page ranges in Stage 01:
python .agents/skills/pdf-to-markdown/pipeline.py --pdf "<PATH_TO_PDF>" --workspace "<WORKSPACE>" --page-ranges "1-5, 7, 8, 10-15" --from-stage 01 --to-stage 01
```

### Phase B: Continuation Detection (Stage 06)
```powershell
# Detect multi-page table and graphic continuations
python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --from-stage 06 --to-stage 06
```

### Phase C: Structural Node Transformation (Stages 07 – 09)

#### 1. Tables (Stage 07)
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --prepare-stage 07
```
- For each `{node_id}.json` in `<WORKSPACE>/tasks/tables/`:
  - Inspect `raw_text` and image preview (`png_path`).
  - Edit or refine the table in `<WORKSPACE>/tasks/tables/{node_id}.md` (GFM or semantic HTML table).
- Apply tables:
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --apply-stage 07
```

#### 2. Graphics (Stage 08)
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --prepare-stage 08
```
- For each `{node_id}.json` in `<WORKSPACE>/tasks/graphics/`:
  - View image using `view_file` on `png_path`.
  - If it is a flowchart/state machine, write a Mermaid diagram with collapsible ASCII callout in `{node_id}.md`.
  - If it is a schematic/timing diagram, author a comprehensive technical description in `{node_id}.sidecar.txt` for RAG vector search.
- Apply graphics:
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --apply-stage 08
```

#### 3. Prose & TOC Delimiters (Stage 09)
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --from-stage 09 --to-stage 09
```

### Phase D: Stream Proofreading & Manifest Normalization (Stage 10)
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --from-stage 10 --to-stage 10
```

### Phase E: Markdown Emission & First Chapter Refinement (Stages 11 – 12)
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --from-stage 11 --to-stage 12
```

### Phase F: Final TOC Wikilinking (Stage 13)
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --output-dir "<OUTPUT_DIR>" --from-stage 13 --to-stage 13
```

---

## 4. Monitoring & Status Check
Check pipeline progress and pending tasks at any time:
```powershell
python .agents/skills/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --status
```


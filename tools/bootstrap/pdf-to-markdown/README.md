# PDF-to-Markdown Bootstrap Converter

This Python command-line program converts technical PDF documents (such as Amiga hardware reference manuals, hardware schematics, and Motorola 68000 PRMs) into Obsidian Markdown through a 14-stage pipeline. It prepares the initial reference knowledge base; subsequent project work uses the generated Markdown and assets.

`pipeline.py` controls execution, status tracking, and stage invalidation. The workflow is defined in Python, while LLM results are not guaranteed to be deterministic. Stage prompts remain runtime inputs in `stages/`.

## Quick Start

Run from the repository root. Install the pinned shared dependencies with `python -m pip install -r tools/bootstrap/conversion/requirements.txt` and authenticate with `codex login` using ChatGPT sign-in. API-key mode is rejected. Every inference stage has an explicit model and reasoning effort in `config.yaml`; the current baseline is `gpt-6.1-sol` / `medium`, concurrency 1.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py `
  --pdf "<PDF_FILE>" `
  --workspace "<WORKSPACE>" `
  --output-dir "<OUTPUT_DIRECTORY>" `
  --config "tools/bootstrap/pdf-to-markdown/config.yaml"
```

The workspace holds intermediate page images, JSON streams, task files, a configuration snapshot, stage status and `.conversion-state.json`. The output directory receives Markdown and assets. The shared `../conversion/` package owns configuration, Codex transport, response schemas, cache and predecessor validation. Use `--cache-dir` to select a disposable cache; the default is `.cache/codex`. Completed schema-validated responses are cached separately from legacy Gemini data.

Use `--page-ranges "19"` for a single physical PDF page. Keep the same source/page selection, workspace, output directory and configuration when continuing a stage interval. `--resume` validates completed predecessor artifacts and their source, procedure and stage model/effort identities before reuse. An incompatible or legacy workspace requires explicit regeneration from the reported stage. Prepared manual tasks also record their predecessor identity. Runtime/schema failures stop the pipeline without model or provider substitution.

## Processing Model

- **Deterministic Python Scripts** handle mechanical tasks (page extraction, 300 DPI rendering, text geometry, asset slicing with 10% margins, stream stitching, chapter partitioning, Markdown emission, and TOC link resolution).
- **Codex text and original-detail image input** handle OCR, segmentation, continuation decisions, table and graphic transcription, prose formatting, title normalization and properties generation through the shared stage-bound client. Stages requiring inference have no offline fallback. Stage 10 currently normalizes titles/slugs and copies rendered node text; it does not perform a separate body-proofreading request.

---

## 1. Directory Structure

```text
tools/bootstrap/pdf-to-markdown/
├── README.md                                # Program usage and stage workflow
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
    │   ├── transform_graphics.py            # Worker for graphics (Mermaid, clean ASCII bitfields, schematics)
    │   ├── prompt_triage.md                 # Triage classifier (Mermaid precedence vs clean ASCII bitfields vs schematics)
    │   ├── prompt_mermaid.md                # Dataflow & calculation trees -> Mermaid + collapsible ASCII callout
    │   ├── prompt_ascii_art.md              # Compact register bitfield boxes (zero leader lines) + structured tables/lists
    │   ├── prompt_rag_sidecar.md            # Technical signal/timing breakdown for RAG (.png.txt)
    │   └── README.md
    │
    ├── 09_transform_prose/
    │   ├── format_prose.py                  # Worker for prose, code_block, and toc (wraps TOC in TOC34534 delimiters)
    │   ├── prompt.md                        # Structural Markdown formatting rules
    │   └── README.md
    │
    ├── 10_proofread_stream/
    │   ├── proofread_stream.py              # Normalizes manifest titles/slugs and copies rendered streams
    │   ├── prompt.md                        # Technical proofreading guidelines (strict anti-hallucination rules)
    │   └── README.md
    │
    ├── 11_emit_markdown/
    │   ├── emit_markdown.py                 # Emits clean Markdown files per partition ({index:02d}_{slug}.md); ignores toc_header
    │   └── README.md
    │
    ├── 12_generate_properties/
    │   ├── generate_properties.py           # Generates publication-grade Obsidian YAML properties (title, book, chapter, tags) with LLM
    │   ├── prompt.md                        # Guidelines for inferring book title and chapter metadata
    │   └── README.md
    │
    ├── 13_refine_first_chapter_name/
    │   ├── refine_name.py                   # Inspects first chapter content & sets canonical title/slug
    │   ├── prompt.md                        # Evaluation guidelines for opening sections
    │   └── README.md
    │
    └── 14_link_toc/
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
> 2. LLM call metrics logging in `.metrics` (JSON content)
> 3. Validated predecessor lineage and downstream artifact invalidation
> 4. Standard argument and path normalization (`--workspace`, `--output-dir`, `--config`)
> 5. Hermetic configuration snapshotting to `<WORKSPACE>/config.yaml`
>
> ### Execution Rules:
> - **Full pipeline**: `python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>"`
> - **Stage interval**: `python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 03 --to-stage 05`
> - **Single stage**: Set `--from-stage` and `--to-stage` to the exact same stage number:
>   `python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02 --to-stage 02`
> - **Stage 01 with specific pages**:
>   `python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --page-ranges "1-5, 7, 8, 10-15" --from-stage 01 --to-stage 01`
> - **Next ready deterministic stage (03, 05, 11, 14)**: `--run-deterministic` runs one ready stage and rejects an inference stage.
>   `python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --run-deterministic`
> - **Optional manual review stages (06, 07, 08, 09)**:
>   - Prepare task items: `python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --prepare-stage 07`
>   - Apply edited task items: `python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --apply-stage 07`

---

## 3. Stage Execution and Optional Manual Correction

The full command above runs the pipeline through `pipeline.py`. For staged execution or inspection, use the six phases below. The `--prepare-stage` and `--apply-stage` commands expose task files for optional manual correction by an operator or agent.

### Phase A: Ingestion & Mechanical Stream Building (Stages 01 – 05)
Run ingestion through the orchestrator:
```powershell
# Run Stages 01 to 05:
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PATH_TO_PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 01 --to-stage 05

# Or test a single stage / specific page ranges in Stage 01:
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PATH_TO_PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --page-ranges "1-5, 7, 8, 10-15" --from-stage 01 --to-stage 01
```

### Phase B: Continuation Detection (Stage 06)
```powershell
# Detect multi-page table and graphic continuations
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 06 --to-stage 06
```

### Phase C: Structural Node Transformation (Stages 07 – 09)

#### 1. Tables (Stage 07)
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --prepare-stage 07
```
- For each `{node_id}.json` in `<WORKSPACE>/tasks/tables/`:
  - Inspect `raw_text` and image preview (`png_path`).
  - Edit or refine the table in `<WORKSPACE>/tasks/tables/{node_id}.md` (GFM or semantic HTML table).
- Apply tables:
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --apply-stage 07
```

#### 2. Graphics (Stage 08)
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --prepare-stage 08
```
- For each `{node_id}.json` in `<WORKSPACE>/tasks/graphics/`:
  - Open the image identified by `png_path`.
  - If it is a calculation tree, dataflow, address generation graph, or state machine: use Mermaid with collapsible ASCII callout in `{node_id}.md`.
  - If it is a register bitfield: use a compact 3–4 line ASCII box (zero leader lines) followed by a structured Markdown table or list.
  - If it is an electrical schematic, waveform, or pinout: author a comprehensive technical description in `{node_id}.sidecar.txt` for RAG vector search and preserve the image embed.
- Apply graphics:
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --apply-stage 08
```

#### 3. Prose & TOC Delimiters (Stage 09)
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 09 --to-stage 09
```

### Phase D: Stream Proofreading & Manifest Normalization (Stage 10)
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 10 --to-stage 10
```

### Phase E: Markdown Emission & Properties Generation (Stages 11 – 12)
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 11 --to-stage 12
```

### Phase F: First Chapter Refinement & Final TOC Wikilinking (Stages 13 – 14)
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --output-dir "<OUTPUT_DIR>" --config "<CONFIG>" --from-stage 13 --to-stage 14
```

---

## 4. Monitoring & Status Check
Check pipeline progress and pending tasks at any time:
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --status
```


## Validated pilot scope

Physical page 19 of the 160-page test book passed all 14 stages using the shared Codex client. The page exercised native extraction, segmentation, table transcription, prose formatting, properties and naming; stages with no matching work still executed. Eight live requests were needed across the pilot, and completed-run resume validated all records without inference. This verifies pipeline integration for one page. Transcription quality, scanned OCR, graphics, real TOC linking, multi-page continuations and manual recovery remain separate validation work; no full-book conversion or indexing ran.

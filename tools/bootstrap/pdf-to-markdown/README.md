# PDF-to-Markdown Bootstrap Converter

This Python command-line program converts technical PDF documents (such as Amiga hardware reference manuals, hardware schematics, and Motorola 68000 PRMs) into Obsidian Markdown through stages 00-14. It prepares the initial reference knowledge base; subsequent project work uses the generated Markdown and assets.

`pipeline.py` controls execution, status tracking, and stage invalidation. The workflow is defined in Python, while LLM results are not guaranteed to be deterministic. Stage prompts remain runtime inputs in `stages/`.

For development, follow the [developer-led workflow](../reference-conversion-contract.md#development-workflow): make the requested change, convert the fragment selected by the user when requested, and let the user assess the output. Automatic tests follow the [bootstrap converter scope](../../../.agents/rules/unit-testing-policy.md#bootstrap-converter-scope).

## Quick Start

Run from the repository root. Install the pinned shared dependencies with `python -m pip install -r tools/bootstrap/conversion/requirements.txt` and authenticate with `codex login` using ChatGPT sign-in. API-key mode is rejected. Every inference stage has an explicit model and reasoning effort in `config.yaml`; the current baseline is `gpt-6.1-sol` / `medium`, concurrency 1.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py `
  --pdf "<PDF_FILE>" `
  --workspace "<PDF_DIRECTORY>/workspace/attempt-01" `
  --config "tools/bootstrap/pdf-to-markdown/config.yaml"
```

The workspace holds intermediate page images, JSON streams, task files, a configuration snapshot, stage status and `.conversion-state.json`. Stage directories are created when their stages execute; a partial run through `02.5` does not create `14_link_toc`. Final Markdown and assets remain in workspace/14_link_toc. The shared `../conversion/` package owns configuration, Codex transport, cache and publication for both converters. The PDF-only `common/` package owns stage completion, artifact validation, page geometry, page selection and response schemas. Completion records store `status: completed`, required file paths and manifest snapshot paths, without content, configuration or procedure hashes. Existing checksum records are read as statuses and paths and rewritten in the simpler format on execution. Use `--cache-dir` to select a disposable cache; the default is `.cache/codex`. Completed schema-validated responses are cached separately from legacy Gemini data.

[Stage 00](stages/00_text_layer/README.md) prepares `<source stem>-ocr.pdf` beside the source, always covering the whole document. It preserves native text and graphics and uses local Tesseract through PyMuPDF on textless pages. If the sibling prepared PDF already exists, it skips OCR and PDF writing, registering the file in the workspace-local manifest and completion record. Existence controls reuse; remove the sibling file explicitly to request fresh preparation. Stage 00 saves new output without render comparison or text/geometry validation. Source page numbers and `prepared_index` remain stable across attempt selections. The original PDF must remain outside the workspace.

[Stage 01](stages/01_preprocess/README.md) reads only that prepared PDF and deterministically emits PNGs and positioned text JSON. Text comes from the reopened PDF, including its OCR spans. `--page-ranges` applies here. Page IDs retain physical source numbers; manifests record relative paths, `prepared`/`none` text provenance, displayed-page coordinates and actual raster transforms. Missing predecessors stop conversion before cleanup. Workers validate schemas, coverage and geometry when consuming inputs and producing results.

Use `--page-ranges "19"` for a single physical PDF page. Keep the same source filename/page selection and workspace when continuing a stage interval. Select the next starting stage yourself with `--from-stage` (default: `00`). Every start clears that stage and all later stages in execution order, even beyond `--to-stage`; only the requested interval executes. After stopping at `02.5`, start at `02.9` to keep the completed review. Changes to code, prompts, model settings or existing file contents do not automatically invalidate earlier completion records; use Git to review changes and explicitly restart affected stages. Prepared manual tasks record their stage and source/page selection. Runtime/schema failures stop the pipeline without model or provider substitution.

## Independent test workspaces and restarts

[Stage 02](stages/02_page_conversion/README.md) groups complete source objects
and converts their text to Markdown using full Stage 01 PNG/JSON input.
[Stage 02.5](stages/02.5_page_conversion_review/README.md) renders ordered labels
and model-selected frames for every object.
[Stage 02.9](stages/02.9_emit_page_markdown/README.md) assembles unchanged page
Markdown and exact original-PNG crops. Execution order is
`00 -> 01 -> 02 -> 02.5 -> 02.9 -> 03`, then Stages 04-14.
Stage 03 reads validated Stage 02 objects directly; the 02.5 review is not an
artifact input to the stream. Use an explicit end stage for a review-only run:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "tools/bootstrap/pdf-to-markdown/config.yaml" --from-stage 02 --to-stage 02.5
```

Use only user-selected source fragments. Restart `02` invalidates its review,
page Markdown bundle and later stream stages; restart `01` also invalidates page conversion.
Restart `02.5` clears the review, page Markdown bundle and all later stream stages.
`02.5` can run alone with validated Stage 01/02 artifacts. Missing external
interval inputs fail before cleanup. The renderer lives entirely within 02.5. Stage 03 keeps `md_text`, object types,
IDs, continuation flags and pixel boxes. It copies Markdown into the legacy
`raw_text` field and derives point/normalized boxes for existing downstream
consumers; those later workers retain their current formatting and merging behavior.
Renamed configuration keys and workspace directories require regeneration;
old 02d/02m artifacts are not automatically moved. Choose an explicit restart when
procedure changes should apply to existing results; saved content is not relabeled.

Stage 02.9 writes exactly one `<WORKSPACE>/02.9_emit_page_markdown/document.md`
and a sibling `assets/` directory, including when there are no crops. All selected
pages, including disjoint ranges, and their object arrays remain in source order.
Textual `md_text` is appended unchanged with blank lines between objects. Tables,
graphics and covers become exact pixel crops named by upstream `segment_id`, with
relative image links in their original positions. There is no padding, rescaling,
text correction, caption generation, TOC repair or multi-page stitching. YAML
frontmatter and a document heading use the known source filename stem. This stage
uses no model and has no inference configuration entry.

With validated Stage 01/02 predecessors, run it alone:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.9 --to-stage 02.9
```

The stage validates inputs and writes a temporary bundle, verifies its Markdown
and crop links, then replaces the output without accumulating stale assets.
Execution requires completed Stage 01 and 02 with their recorded files present.
Restart `02.9` clears its bundle and all later stages; Stage 03 keeps its direct Stage 02
input dependency. Review PNGs are neither crop sources nor required inputs. Publication
through `--publish` remains restricted to Stage 14 and does not export this bundle.
Technical completion does not certify text, reading order or crop-boundary quality.

For agent-run conversions, use the source PDF directory's `workspace/` only as a container for named attempt directories. Create a child from the first attempt, even when `workspace/` is empty, such as `<PDF_DIRECTORY>/workspace/page-64-attempt-01/` or `<PDF_DIRECTORY>/workspace/all-pages-00-01/`. Always pass the child explicitly with `--workspace`: the CLI currently defaults to `<PDF_DIRECTORY>/workspace/` when this option is omitted with `--pdf`. Each independent attempt gets a new child; continuation or explicit restart of the same attempt reuses its existing child. The original PDF stays outside the attempt workspace. Final Markdown stays in `<WORKSPACE>/14_link_toc`; `--output-dir` has been removed. Restarting one workspace leaves other workspaces intact. Cached model responses remain reusable after intermediate artifacts are cleared.

Add `--publish` to copy completed Markdown and assets into the sibling book directory
without `-tmp`. The workspace must be inside `<book>-tmp/`. The target must be absent
or empty; occupied directories and links are rejected before execution and checked
again before copying. Publication requires completion through Stage 14. An already
completed conversion can rerun Stage 14 with `--from-stage 14 --to-stage 14 --publish`;
the previous Stage 14 output is cleared and regenerated from validated predecessors
before copying. Intermediate state and metrics stay in the workspace.

For example, first prepare stages 00–04 for physical pages 5–10 in `<WORKSPACE_A>`. Then restart from stage 05:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py `
  --workspace "<WORKSPACE_A>" `
  --config "tools/bootstrap/pdf-to-markdown/config.yaml" `
  --page-ranges "5-10" --from-stage 05
```

Before running stage 05, the orchestrator checks retained stages 00–04, persists invalidation of stages 05–14, restores shared working manifests from retained snapshots, and clears invalidated artifacts, statistics and manual tasks. Invalidation is saved before deletion, so interrupted cleanup cannot leave a deleted result marked completed. Each stage is marked completed only after successful output validation. Cleanup covers the entire downstream conversion, even when `--to-stage` requests only one stage. It never selectively clears individual pages. A different page range requires a separate workspace or explicit regeneration from stage 00; retained stages from another range cannot supply stage 05.

Missing downstream files do not prevent explicit restart. Missing shared working manifests are restored from retained snapshots. The operator chooses the restart stage; the pipeline does not select the earliest unfinished stage or skip completed stages in the requested interval. A missing retained predecessor requires restarting at its owning stage. Running stage 00 requires `--pdf`; an existing sibling OCR PDF is retained even on an explicit restart. Starting after 00 retains the saved page selection when `--page-ranges` is omitted. Restart at 01 retains completed Stage 00 and needs no source path. Restart at 00 retains per-page OCR request recovery records, whose request identities are separate from stage completion. Modified retained files are not checked against historical contents. Workspaces without Stage 00 completion require `--pdf "<PDF>" --from-stage 00` to register or prepare the shared full-document PDF; their compact workspace PDFs are not reused as full documents.

Manual `--prepare-stage` also resets that stage and all later stages. `--apply-stage` preserves the current stage's prepared edits while clearing old stage outputs and later tasks. Earlier source assets remain intact.

## Processing Model

- **Deterministic Python Scripts** handle mechanical tasks (page extraction, 300 DPI rendering, text geometry, asset slicing with 10% margins, stream stitching, chapter partitioning, Markdown emission, and TOC link resolution).
- **Codex text and original-detail image input** handle segmentation, continuation decisions, table and graphic transcription, prose formatting, title normalization and properties generation through the shared stage-bound client. Stages requiring inference have no offline fallback. Stage 10 currently normalizes titles/slugs and copies rendered node text; it does not perform a separate body-proofreading request.

---

## 1. Directory Structure

```text
tools/bootstrap/pdf-to-markdown/
├── README.md                                # Program usage and stage workflow
├── pipeline.py                              # Master CLI orchestrator & task manager
├── config.yaml                              # Global configuration (DPI, paths, heuristics)
├── common/                                  # PDF-only infrastructure and stage contracts
└── stages/
    ├── 00_text_layer/
    │   ├── prepare_text_layer.py            # Validates/publishes a separate native/OCR PDF
    │   └── README.md
    ├── 01_preprocess/
    │   ├── preprocess.py                    # Renders PDF -> 300 DPI PNG, text blocks and page geometry JSON
    │   └── README.md
    │
    ├── 02_page_conversion/
    │   ├── convert_page.py                  # Complete page objects with Markdown and pixel boxes
    │   ├── prompt.md
    │   └── README.md
    │
    ├── 02.5_page_conversion_review/
    │   ├── render_review.py                 # Page frames, ordered labels and continuation flags
    │   └── README.md
    │
    ├── 02.9_emit_page_markdown/
    │   ├── emit_page_markdown.py             # One document and exact table/graphic/cover crops
    │   └── README.md
    │
    ├── 03_build_raw_stream/
    │   ├── build_stream.py                  # Merges segmented pages into global raw_stream.json
    │   ├── extract_initial_assets.py        # Crops Stage 01 PNGs; raw text stays in node JSON
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
    │   ├── emit_markdown.py                 # Emits clean Markdown files per partition ({index:02d}_{slug}.md); ignores toc_heading
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
> **NEVER invoke stage worker scripts directly** (e.g. `python stages/01_preprocess/preprocess.py` or `python stages/02_page_conversion/convert_page.py`).
>
> **ALL execution MUST go through `pipeline.py`**.
> Directly executing sub-scripts bypasses:
> 1. Status progression tracking in `stage_status.json`
> 2. LLM call metrics logging in `.metrics` (JSON content)
> 3. Completed predecessor status, required files and downstream artifact invalidation
> 4. Workspace path normalization and guarded publication (`--workspace`, `--publish`, `--config`)
> 5. Hermetic configuration snapshotting to `<WORKSPACE>/config.yaml`
>
> ### Execution Rules:
> - **Full pipeline**: `python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>"`
> - **Stage interval**: `python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 03 --to-stage 05`
> - **Single stage**: Set `--from-stage` and `--to-stage` to the exact same stage number:
>   `python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02 --to-stage 02`
> - **Prepare and preprocess specific pages in a new workspace**:
>   `python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --page-ranges "1-5, 7, 8, 10-15" --from-stage 00 --to-stage 01`
> - **Selected deterministic stage (00, 01, 02.5, 02.9, 03, 05, 11, 14)**: `--run-deterministic` requires `--from-stage`, runs that stage alone and rejects an inference stage. It clears the selected stage and all later results.
>   `python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.5 --run-deterministic`
> - **Optional manual review stages (06, 07, 08, 09)**:
>   - Prepare task items: `python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --prepare-stage 07`
>   - Apply edited task items: `python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --apply-stage 07`

---

## 3. Stage Execution and Optional Manual Correction

The full command above runs the pipeline through `pipeline.py`. For staged execution or inspection, use the six phases below. The `--prepare-stage` and `--apply-stage` commands expose task files for optional manual correction by an operator or agent.

### Phase A: Ingestion & Mechanical Stream Building (Stages 00 – 05)
Run ingestion through the orchestrator:
```powershell
# Run Stages 00 to 05:
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PATH_TO_PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 00 --to-stage 05

# Or prepare and preprocess the requested physical pages:
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PATH_TO_PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --page-ranges "1-5, 7, 8, 10-15" --from-stage 00 --to-stage 01
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
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 13 --to-stage 14
```

---

## 4. Monitoring & Status Check
Check pipeline progress and pending tasks at any time:
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --status
```


## Fragment evidence and closure limits

On 2026-10-06, physical TestBook page 64 completed stages 00-01. Its source render
matched physical page 5 of the A500/A2000 Technical Reference Manual. The source
page had no text. Stage 00 added invisible OCR, preserved all 160 page identities
and geometry, retained native/unselected content and verified the selected render
was identical. Stage 01 extracted 20 text blocks from the reopened prepared PDF
and wrote `page_0064.png`/`.json`, with zero OCR calls in preprocessing. One live
OCR request produced a reusable response; the successful rerun used the cache.
This establishes the scanned path for that fragment, not native-copy, mixed,
blank/graphic-only or real rotated/cropped sample acceptance. The user assesses
transcription and content quality. No downstream stage or full-book conversion ran.

The user closed PDF-TEXT-1.1 on 2026-10-06 with these recorded limits. No additional
source-category conversion was requested; no additional coverage or successful
repository milestone gate is implied by that closure.

### Historical migration pilot

Physical page 19 of the 160-page test book passed all 14 stages using the shared Codex client. The page exercised native extraction, segmentation, table transcription, prose formatting, properties and naming; stages with no matching work still executed. Eight live requests were needed across the pilot. This verifies pipeline integration for one page under the pilot's implementation. Transcription quality, scanned OCR, graphics, real TOC linking, multi-page continuations and manual recovery remain separate validation work; no full-book conversion or indexing ran.

# PDF-to-Markdown

This bootstrap program prepares the initial reference knowledge base. Follow the
[developer-led workflow](../reference-conversion-contract.md#development-workflow):
change the converter, run only a fragment selected and requested by the user,
and let the user assess content. Technical checks do not certify conversion quality.

## Run an attempt

Install shared dependencies with `python -m pip install -r tools/bootstrap/conversion/requirements.txt`
and authenticate with `codex login`. Inference uses the pinned Codex transport,
ChatGPT authentication, isolated requests and original-detail images.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py `
  --pdf "<PDF_DIRECTORY>/source.pdf" `
  --workspace "<PDF_DIRECTORY>/workspace/attempt-01" `
  --config tools/bootstrap/pdf-to-markdown/config.yaml `
  --page-ranges "1-5,7" --from-stage 00 --to-stage 01
```

Agent-run conversions use a named child of the source directory's `workspace/`
container. The CLI still defaults to the container when `--workspace` is omitted.
Supply `--pdf` for every conversion or restart; its path is never persisted.
The attempt's existing `config.yaml` takes precedence; `--config` initializes a
missing file. Existing model settings remain. Rendering uses fixed 300 DPI PNGs,
inference concurrency 1 and a 180-second request timeout. Configuration is parsed
without local validation; ordinary YAML, JSON, image/PDF and filesystem errors stop execution.

## Stage data and execution

The execution sequence is `00 -> 01 -> 02 -> 02.80 -> 02.81 -> 02.90`.
Each stage reads explicitly named predecessors and writes its own results.

| Stage | Input and result |
| --- | --- |
| [00](stages/00_text_layer/README.md) | Source PDF -> shared full-source sibling OCR PDF and reusable recovery records |
| [01](stages/01_preprocess/README.md) | Prepared PDF -> positioned text JSON and physical-page PNGs |
| [02](stages/02_page_conversion/README.md) | Stage 01 -> ordered logical objects, Markdown and pixel boxes |
| [02.80](stages/02.80_filter_page_content/README.md) | Stage 01/02 -> objects with headers and footers removed |
| [02.81](stages/02.81_transform_page_tables/README.md) | Stage 01/02.80 -> table markup |
| [02.90](stages/02.90_emit_page_markdown/README.md) | Stage 01/02.81 -> `document.md` and required raster assets |

Filtering reads Stage 02 directly, without reclassification overrides.
Table conversion reads only Stage 02.80; omitted pages never fall back to Stage 02.
Export reads Stage 02.81 directly and emits converted table markup at its source
position, without `<details>`, RAG text or source-image companions. Stage 02.81
uses temporary crops for transcription only. Separate captions, notes and legends
remain unchanged; grouping and descriptions belong to later book processing.
Empty rows are preserved as produced by table conversion. Unconverted tables
retain a raster fallback cropped from Stage 01.
Graphics and covers crop exact Stage 01 pixels. The emitter replaces its bundle
after successful writes and removes obsolete assets.

### Physical-page selection

`--page-ranges` selects physical 1-based pages for this invocation only.
Stage 00 always prepares or reuses the complete source PDF. Stages 01, 02, 02.80
and 02.81 apply the selection while enumerating their input, including direct
restarts with all-page predecessors. Omission means all available input pages;
selection is not stored in config. Stage 02.90 exports the whole completed 02.81
dataset, so select the intended fragment when generating that predecessor.
Page files are ordered numerically, without coverage or PNG/JSON pairing checks.

## Restart, status and publication

Starting a stage invalidates it and every later active stage before deleting
outputs. The end stage limits execution, not cleanup. Successful external
predecessors are checked before cleanup through `STAGE_REGISTRY.inputs` and
`stage_status.json`. There is no automatic completed-stage skipping.
Changing code does not regenerate retained artifacts. Preserve edited exports
outside cleanup before restarting. The sibling prepared OCR PDF and request cache
survive cleanup. Removed-stage artifacts in historical
attempts remain untouched and are not consumed by the active pipeline.

`stage_status.json` records running/success/failed and execution metrics atomically.
Success establishes completed execution, not source or semantic fidelity.
Normal execution creates no conversion state, manifest, snapshot or chapter registry.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --status
```

Use `--cache-dir` to override the shared `.cache/codex` request cache. PDF retains
transport authentication/completion handling and JSON parsing, without local
configuration/catalog checks or response-schema enforcement. HTML retains its validations.
Saved page objects omit zero rotation, false continuation, null heading levels
and empty Markdown. Source lineage, replacement stage and `image_only` are not
persisted. Consumers apply missing-field defaults without modifying prompt data.

Final output is `<WORKSPACE>/02.90_emit_page_markdown/`. `--publish` requires
completion through 02.90 and copies its Markdown/assets to the absent or empty
sibling book directory without `-tmp`. Destination protection is checked before
execution and publication. Independent attempts retain their own outputs.

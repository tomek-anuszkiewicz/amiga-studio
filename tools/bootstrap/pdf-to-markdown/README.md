# PDF-to-Markdown

Prepare PDF page images, positioned text, logical objects and transcribed tables
for [book-to-markdown](../book-to-markdown/README.md). Preparation ends at Stage 05;
the separate chapter generator is not implemented yet.

## Run an attempt

Install dependencies with `python -m pip install -r tools/bootstrap/pdf-to-markdown/requirements.txt`
and authenticate with `codex login`. Run from the repository root:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py `
  --pdf "<PDF_DIRECTORY>/source.pdf" `
  --workspace "<PDF_DIRECTORY>/workspace/attempt-01" `
  --config tools/bootstrap/pdf-to-markdown/config.yaml `
  --page-ranges "2-3" --from-stage 01 --to-stage 05
```

Use a named workspace for each independent attempt. Supply the original `--pdf`
on every run. The attempt's existing `config.yaml` takes precedence; `--config`
initializes a missing file. It selects model/effort for Stages 03 and 05.
Inference uses ChatGPT authentication, original-detail images, concurrency 1
and a 180-second request timeout. Rendering uses 300 DPI.

## Stages and outputs

| Stage | Result |
| --- | --- |
| [01: Text layer](stages/01_text_layer/README.md) | Full-source `<source stem>-ocr.pdf` beside the original PDF; local Tesseract when needed |
| [02: Preprocess](stages/02_preprocess/README.md) | Selected-page PNGs and positioned-text JSON in `02_preprocess/` |
| [03: Page conversion](stages/03_page_conversion/README.md) | Ordered page objects, Markdown text and pixel boxes in `03_page_conversion/` |
| [04: Filter](stages/04_filter_page_content/README.md) | Page objects with headers/footers removed in `04_filter_page_content/` |
| [05: Tables](stages/05_transform_page_tables/README.md) | Only pages changed by table transcription in `05_transform_page_tables/` |

The book-processing handoff consists of Stage 02 images/text, Stage 04 filtered
objects and Stage 05 replacement pages. Unchanged pages remain in Stage 04;
an empty Stage 05 directory is valid. Its status entry records the selected range.

### Physical-page selection

`--page-ranges` selects physical 1-based pages for this invocation, for example
`"2-3,7"`. Stage 01 always prepares or reuses the complete PDF; Stages 02–05
process only selected input pages. Omission selects all available pages.

## Restart and status

Use `--from-stage` and `--to-stage` to run an interval; equal values run one stage.
Starting a stage clears its results and all later stage results, even when
`--to-stage` stops earlier. Retained predecessors must have successful statuses.
Completed stages are not automatically skipped.

The sibling OCR PDF and request cache survive restart. An existing OCR PDF is
reused by file existence; remove it explicitly when new OCR is required.
Attempts using the previous stage numbering require a new named workspace.

`stage_status.json` records execution and request metrics. Inspect it with:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --status
```

The shared response cache defaults to `Obsidian/Amiga/Reference/.cache/codex`;
`--cache-dir` overrides it. Stage IDs participate in cache identity.

Follow the [developer-led workflow](../reference-conversion-contract.md#development-workflow):
run the selected fragment and inspect its content. A successful stage confirms
execution, not transcription quality. PDF responses are parsed without local
schema enforcement; source-fidelity details belong to the
[conversion contract](../reference-conversion-contract.md).

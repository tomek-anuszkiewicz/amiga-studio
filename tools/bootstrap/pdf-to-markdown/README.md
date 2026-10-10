# PDF-to-Markdown

Prepare PDF page images, positioned text, logical objects and transcribed tables
for [book-to-markdown](../book-to-markdown/README.md). Preparation ends at Stage 06;
the separate chapter generator is not implemented yet.

## Run an attempt

Install dependencies with `python -m pip install -r tools/bootstrap/pdf-to-markdown/requirements.txt`
and authenticate with `codex login`. Run from the repository root:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py `
  --pdf "<PDF_DIRECTORY>/source.pdf" `
  --page-ranges "2-3" --from-stage 01 --to-stage 06
```

Without `--workspace`, the program creates and uses `<PDF_DIRECTORY>/pdf-to-markdown/`.
This directory, named `pdf-to-markdown`, is the default for subsequent development.
Pass `--workspace` only when a separate attempt directory is needed. For status
without `--pdf` or `--workspace`, the default is `pdf-to-markdown/` in the current directory.

Supply the original `--pdf` on every conversion or restart. If the workspace has
no `config.yaml`, the program copies `tools/bootstrap/pdf-to-markdown/config.yaml`
there automatically. An existing workspace config is preserved without rewriting.
Edit that local file to configure Stage 01 OCR, shared rendering DPI, and
model/effort for Stages 03 and 05. Missing preparation fields use the repository
defaults without rewriting an existing workspace config.
Inference uses ChatGPT authentication, original-detail images and a 180-second
request timeout. `llm.concurrency` limits simultaneous requests within each stage
(repository default: 2; set 1 for sequential execution). Stage 03 runs per page;
Stage 05 runs per table, including tables on the same page. Stages remain
sequential. Each worker owns a separate Codex transport; one coordinator saves
page results, preserving source object order. Metrics writes are serialized.
On failure, workers stop taking new jobs; requests already in progress may finish.
Capacity-limit notifications abort the affected transport without retrying.
The limit applies to one pipeline invocation, not other Codex sessions or
concurrent pipeline processes. It is a conservative starting value, not a
provider-guaranteed safe account limit. Edit the attempt's local `config.yaml`
to change an existing workspace's limit.
Rendering uses `rendering.dpi` (default 300 DPI) in Stages 01 and 02.

## Stages and outputs

| Stage | Result |
| --- | --- |
| [01: Text layer](stages/01_text_layer/README.md) | Full-source `<source stem>-ocr.pdf` beside the original PDF; local Tesseract when needed |
| [02: Preprocess](stages/02_preprocess/README.md) | Selected-page PNGs and positioned-text JSON in `02_preprocess/` |
| [03: Page conversion](stages/03_page_conversion/README.md) | Ordered page objects, Markdown text and pixel boxes in `03_page_conversion/` |
| [04: Filter](stages/04_filter_page_content/README.md) | Page objects with headers, footers and thumb indexes removed in `04_filter_page_content/` |
| [05: Tables](stages/05_transform_page_tables/README.md) | Only pages changed by table transcription in `05_transform_page_tables/` |
| [06: Crops](stages/06_extract_crops/README.md) | All `graphic` bbox crops in `06_extract_crops/assets/<segment_id>.png` |
| [07: Callouts](stages/07_repair_callouts/README.md) | Final page objects with adjacent advisory labels/bodies assembled as Markdown callouts in `07_repair_callouts/` |

Stage 05 replacements overlay Stage 04 pages; unchanged pages remain in Stage 04.
An empty Stage 05 directory is valid. Its status entry records the selected range.
Stage 06 overlays Stage 05 pages on Stage 04 and extracts graphic crops from
Stage 02 PNGs. Stage 07 applies that same page overlay and repairs callouts.
The book-processing handoff consists of Stage 02 images/text, Stage 06 graphic
assets and all retained Stage 07 final page objects.

### Physical-page selection

`--page-ranges` selects physical 1-based pages for this invocation, for example
`"2-3,7"`. Stage 01 always prepares or reuses the complete PDF; Stages 02–07
process only selected input pages. Omission selects all available pages.

## Restart and status

Use `--from-stage` and `--to-stage` to run an interval; equal values run one stage.
Starting a stage clears its results and all later stage results, even when
`--to-stage` stops earlier. With `--page-ranges`, Stages 02–07 clear only files
for the selected pages, including Stage 06 graphic crops; other pages remain.
Without a range, these stage directories are cleared completely. Stage 01 always
clears its whole workspace directory, while preserving the sibling OCR PDF.
Retained predecessors must have successful statuses.
Completed stages are not automatically skipped.

The sibling OCR PDF and request cache survive restart. An existing OCR PDF is
reused by file existence; remove it explicitly when new OCR is required.
Attempts using the previous stage numbering require a new named workspace.

`stage_status.json` records the last invocation's execution, page range and request
metrics, not accumulated page coverage. Restart invalidates the selected stage
and all later completion statuses before cleanup, including partial restarts.
Book import reads all retained Stage 07 pages rather
than restricting the book to the last invocation's range. Inspect status with:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF_DIRECTORY>/source.pdf" --status
```

PDF worker responses use the fixed shared cache at
`tools/bootstrap/book-to-markdown/books/.cache/codex`. Cache identity uses stable
operation IDs (`pdf_page_to_objects`, `pdf_table_to_markup`), independent of stage
names and numbering. Prompts, inputs, model settings and isolated execution settings
participate in the key. The pipeline has no `--config`, `--cache-dir` or `--verbose` options.

Model requests contain only the stage prompt, source JSON where needed, and the
page or table image. The shared transport clears base/developer instructions and
does not send output schemas, separate type lists or examples outside the prompt.
Stage 03 chooses `type` and nullable `role` independently in one image-and-text
request. Roles are `caption`, `legend`, `note` and `footnote`; they are preserved
through filtering, table transcription and final callout preparation into book
import. A legend represented as a table is transcribed as a table in Stage 05.

Agent development guidance is in the [bootstrap instructions](../AGENTS.md#converter-development).

## Tests

PDF execution, OCR profile and page-selection tests live in `tests/` beside this
program. Run them from the repository root:

```powershell
python -m unittest discover -s tools/bootstrap/pdf-to-markdown/tests -q
```

Shared conversion transport tests remain in the repository's top-level `tests/`.
A successful stage confirms execution, not transcription quality.
PDF responses are parsed without local schema enforcement. Shared execution and
cache behavior are documented in the [conversion client README](../conversion/README.md).

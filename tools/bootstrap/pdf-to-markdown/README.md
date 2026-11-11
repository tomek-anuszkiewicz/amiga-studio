# PDF-to-Markdown

Prepare PDF page images, positioned text, logical objects and transcribed tables
for [book-to-markdown](../book-to-markdown/README.md). Preparation ends at Stage 08;
book chapter generation is handled by the separate Book-to-Markdown program.

## Run an attempt

Install dependencies with `python -m pip install -r tools/bootstrap/pdf-to-markdown/requirements.txt`
and authenticate with `codex login`. Run from the repository root:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py `
  --pdf "<PDF_DIRECTORY>/source.pdf" `
  --page-ranges "2-3" --from-stage 01 --to-stage 08
```

Without `--workspace`, the program creates and uses `<PDF_DIRECTORY>/pdf-to-markdown/`.
This directory, named `pdf-to-markdown`, is the default for subsequent development.
Pass `--workspace` only when a separate attempt directory is needed. For status
without `--pdf` or `--workspace`, the default is `pdf-to-markdown/` in the current directory.

Supply the original `--pdf` on every conversion or restart. If the workspace has
no `config.yaml`, the program copies `tools/bootstrap/pdf-to-markdown/config.yaml`
there automatically. An existing workspace config is preserved without rewriting.
Edit that local file to configure Stage 01 OCR, shared rendering DPI, and
model/effort for Stages 03, 05 and 08. Missing preparation fields use the repository
defaults without rewriting an existing workspace config.
Existing workspace configs need an `llm.stages.08_proofreading` model/effort pair
before running Stage 08; the pipeline does not rewrite them.
Inference uses ChatGPT authentication, original-detail images and a 180-second
request timeout. `llm.concurrency` limits simultaneous requests within each stage
(repository default: 1). Stages 03 and 08 run per page;
Stage 05 runs per table, including tables on the same page. Stages remain
sequential. Each worker owns a separate Codex transport; one coordinator saves
page results, preserving source object order. Metrics writes are serialized.
On failure or Ctrl+C, workers stop taking new jobs and receive a shared
cancellation signal that interrupts request delays, retry waits and active
transports. Responses that finish before cancellation may still be saved.
HTTP 429/503, `rateLimitExceeded` and `serverOverloaded` pause and retry the same
request with exponential backoff; see the
[shared client retry policy](../conversion/README.md#execution). Usage and
session-budget limits still stop the stage.
The limit applies to one pipeline invocation, not other Codex sessions or
concurrent pipeline processes. It is a conservative starting value, not a
provider-guaranteed safe account limit. Edit the attempt's local `config.yaml`
to change an existing workspace's limit.
For a rate-limited attempt, set `llm.concurrency: 1` and
`llm.post_request_delay_seconds` in its local `config.yaml`. This delays each
new model request until the configured time has elapsed after the previous
model response, regardless of generation time. Cached responses are reused
immediately and do not reset the delay.
Pass `--trace-codex` to show spacing waits, requests sent to Codex and their
completion or failure. Cached results and streaming notification fragments
produce no trace events. The trace omits prompt, image and response content.
Stage 08 also prints each selected page's request, response call and cache-hit
counts, and completion without requiring this flag.
For a saved live log, pipe through
`Tee-Object` in PowerShell (include stderr with `2>&1`).
Rendering uses `rendering.dpi` (default 300 DPI) in Stages 01 and 02.

On Windows, the pipeline automatically prevents idle system sleep while running
the selected stages, including request delays and retry backoff. It uses
[`SetThreadExecutionState`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate)
with `ES_CONTINUOUS | ES_SYSTEM_REQUIRED` and restores the previous thread state
on success, failure or Ctrl+C. The display may still turn off, and manual sleep
remains available. No power plan is changed. Status-only invocations and other
operating systems do not acquire this request.

## Stages and outputs

| Stage | Result |
| --- | --- |
| [01: Text layer](stages/01_text_layer/README.md) | Full-source `<source stem>-ocr.pdf` beside the original PDF; local Tesseract when needed |
| [02: Preprocess](stages/02_preprocess/README.md) | Selected-page PNGs and positioned-text JSON in `02_preprocess/` |
| [03: Page conversion](stages/03_page_conversion/README.md) | Ordered page objects, Markdown text and pixel boxes in `03_page_conversion/` |
| [04: Filter](stages/04_filter_page_content/README.md) | Only pages changed by removing headers, footers and thumb indexes in `04_filter_page_content/` |
| [05: Tables](stages/05_transform_page_tables/README.md) | Only pages changed by table transcription in `05_transform_page_tables/` |
| [06: Crops](stages/06_extract_crops/README.md) | All `graphic` bbox crops in `06_extract_crops/assets/<segment_id>.png` |
| [07: Callouts](stages/07_repair_callouts/README.md) | Only pages changed by assembling adjacent advisory labels/bodies as Markdown callouts in `07_repair_callouts/` |
| [08: Proofreading](stages/08_proofreading/README.md) | Only pages changed by source-supported text corrections in `08_proofreading/` |

Stage 03 supplies the base page objects. Stages 04, 05, 07 and 08 save full-page
replacements only when their data changes; an empty directory is valid for each
of these stages. Stage 04 saves an empty `segments` list when filtering empties a
page, so later readers omit it instead of restoring excluded objects.
Each reader chooses the latest available JSON up to its input stage.
Stage 06 reads Stages 05, 04 and 03 and extracts graphic crops from Stage 02 PNGs.
Stage 07 uses the same page collection to repair callouts. Stage 08 reads
Stages 07, 05, 04 and 03 and reviews text-only ambiguities against Stage 02 page
images when needed. The book-processing handoff consists of Stage 02 images/text,
Stage 06 graphic assets and pages resolved in order from Stages 08, 07, 05, 04 and 03.

### Physical-page selection

`--page-ranges` selects physical 1-based pages for this invocation, for example
`"2-3,7"`. Stage 01 always prepares or reuses the complete PDF; Stages 02–08
process only selected input pages. Omission selects all available pages.

## Restart and status

Use `--from-stage` and `--to-stage` to run an interval; equal values run one stage.
Starting a stage clears its results and all later stage results, even when
`--to-stage` stops earlier. With `--page-ranges`, Stages 02–08 clear page files
for the selected pages, including Stage 06 graphic crops; other pages remain.
Without a range, these stage directories are cleared completely. Stage 01 always
clears its whole workspace directory, while preserving the sibling OCR PDF.
Retained predecessors must have successful statuses.
Completed stages are not automatically skipped.

Preparation caches and the Codex request cache survive restart. Stages 01, 02 and 06
restore matching preparation results instead of repeating OCR, rendering, text
extraction or graphic cropping. An existing output file alone does not establish a cache hit.
Attempts using the previous stage numbering require a new named workspace.

### Preparation cache

`tools/bootstrap/.cache/pdf/01_text_layer/<SHA-256>/` stores a completed prepared
PDF and its provenance JSON. The key hashes the original PDF SHA-256, effective
OCR profile (including language model hash and DPI), backend/runtime identity and
preparation contract version. Stage 01 always covers the complete source PDF.

`tools/bootstrap/.cache/pdf/02_preprocess/<SHA-256>/` stores one PNG and positioned
text JSON per page. Its key hashes the prepared PDF SHA-256, physical page number,
DPI, geometry schema, backend/runtime identity and preprocessing contract version.
Page ranges and workspace paths are not part of the key, so attempts with
overlapping selections share cached pages. PNGs remain separate binary files.

`tools/bootstrap/.cache/pdf/06_extract_crops/<SHA-256>/` stores one graphic crop
PNG per entry. Its key hashes the Stage 02 page PNG SHA-256, crop `bbox`, runtime
identity including Pillow, and crop contract version. Workspace paths, page
ranges and segment IDs are excluded; the same source crop can be restored under
the current segment filename without repeating extraction.

Each entry has a completion manifest with the input identity and output hashes.
Missing, incomplete or damaged entries are recomputed; only complete results are
published. Hits copy files into the ordinary output paths and print `[cache]`.
The first execution with this cache may recompute old outputs, which are not
imported into it. Deleting a prepared output alone restores it from cache; remove
the matching cache entry as well to force fresh preparation with unchanged inputs.
Neither cache reuse nor successful preparation establishes transcription quality.

`stage_status.json` records the last invocation's execution, page range and request
metrics, not accumulated page coverage. Restart invalidates the selected stage
and all later completion statuses before cleanup, including partial restarts.
Book import reads all retained pages across Stages 03 through 08 rather
than restricting the book to the last invocation's range. Inspect status with:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF_DIRECTORY>/source.pdf" --status
```

PDF worker responses use the fixed shared cache at
`tools/bootstrap/.cache/codex`. Cache identity uses stable
operation IDs (`pdf_page_to_objects`, `pdf_table_to_markup`,
`pdf_page_proofreading`), independent of stage names and numbering. Prompts,
inputs, model settings and isolated execution settings
participate in the key. The pipeline has no `--config`, `--cache-dir` or `--verbose` options.

Model requests contain only the stage prompt, source JSON where needed, and the
page or table image where required. Stage 08 first sends compact text objects and
short neighboring excerpts; only pages with unresolved items receive a second
request with the page image. The shared transport clears base/developer
instructions and
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

Shared conversion transport tests live in `tools/bootstrap/conversion/tests/`.
A successful stage confirms execution, not transcription quality.
Stages 03 and 05 parse JSON without local response-schema enforcement. Stage 08
validates patch fields and target segment IDs before applying text replacements.
Shared execution and cache behavior are documented in the
[conversion client README](../conversion/README.md).

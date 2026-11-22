# PDF-to-Markdown

PDF-to-Markdown prepares PDF page images, positioned text, logical objects and
transcribed tables for [book-to-markdown](../book-to-markdown/README.md).
Preparation ends at Stage 08. The separate Book-to-Markdown program generates
book chapters.

## Run an attempt

Install dependencies with `python -m pip install -r tools/bootstrap/pdf-to-markdown/requirements.txt`
and authenticate with `codex login`. Run the following command from the
repository root.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py `
  --pdf "<PDF_DIRECTORY>/source.pdf" `
  --page-ranges "2-3" --from-stage 01 --to-stage 08
```

Without `--workspace`, the program creates and uses `<PDF_DIRECTORY>/pdf-to-markdown/`.
Continue development in that default `pdf-to-markdown` directory. Pass
`--workspace` only when you need a separate directory for an attempt. A status
request with neither `--pdf` nor `--workspace` looks for `pdf-to-markdown/`
in the current directory.

Supply the original `--pdf` on every conversion or restart. If the workspace has
no `config.yaml`, the program copies `tools/bootstrap/pdf-to-markdown/config.yaml`
there automatically. It leaves an existing workspace configuration file as
written and uses repository defaults for missing preparation fields. Edit the
local file to configure Stage 01 OCR, shared rendering DPI, and the model and
effort for Stages 03, 05 and 08. An existing workspace configuration needs an
`llm.stages.08_proofreading` model and effort pair before Stage 08 can run.

Model requests use ChatGPT authentication, original-detail images and a
180-second timeout. `llm.concurrency` limits simultaneous requests within each
stage and defaults to 1 in the repository. Stages 03 and 08 work one job per
page. Stage 05 works one job per table, and its limit includes tables on the
same page. The stages themselves run in order. Each worker uses its own
Codex transport to send requests and receive responses. A single coordinator
saves results by page and preserves the source object order. The program
writes metrics one at a time.

On failure or Ctrl+C, the workers stop taking new jobs. A shared cancellation
signal interrupts request delays, retry waits and active transports. The
coordinator may still save responses that finish before cancellation.
HTTP 429/503, `rateLimitExceeded` and `serverOverloaded` pause and retry the same
request with exponential backoff, which increases the wait between retries.
Usage and session-budget limits still stop the stage. See the
[shared client retry policy](../conversion/README.md#execution).

The concurrency limit applies to one pipeline invocation. Other Codex sessions
and concurrent pipeline processes can issue their own requests. The default is
a conservative starting value and carries no provider guarantee for account
usage. Edit the attempt's local `config.yaml` to change an existing workspace's
limit.
For a rate-limited attempt, set `llm.concurrency: 1` and
`llm.post_request_delay_seconds` in its local `config.yaml`. This delays each
new model request until the configured time has elapsed after the previous
model response, regardless of generation time. Cached responses are reused
immediately and do not reset the delay.
Pass `--trace-codex` to show spacing waits, requests sent to Codex and their
completion or failure. Cached results and streaming notification fragments
produce no trace events. The trace omits prompt, image and response content.
Even without this flag, Stage 08 reports each selected page's request, the
response call and cache-hit counts, and when processing finishes.
For a saved live log, pipe through
`Tee-Object` in PowerShell (include stderr with `2>&1`).
Rendering uses `rendering.dpi` (default 300 DPI) in Stages 01 and 02.

On Windows, the pipeline automatically prevents idle system sleep while running
the selected stages, including request delays and retry backoff. It uses
[`SetThreadExecutionState`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate)
with `ES_CONTINUOUS | ES_SYSTEM_REQUIRED` and restores the previous thread state
on success, failure or Ctrl+C. The display may still turn off, and manual sleep
remains available. The pipeline leaves the power plan unchanged and requests
this behavior only on Windows when running stages. A status-only invocation
makes no such request.

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

Stage 03 supplies the initial page objects. To read a page, a later stage
looks up the latest available JSON through its input stage. Stages 04, 05, 07
and 08 save a full-page replacement only if the page data changes. An empty
output directory is therefore valid for each of these stages. If Stage 04
removes every object on a page, it saves an empty `segments` list. Later readers
omit the page when they see that version, which keeps filtered objects out of
their results.
Stage 06 reads Stages 05, 04 and 03 and extracts graphic crops from Stage 02 PNGs.
Stage 07 uses the same page collection to repair callouts. Stage 08 reads
Stages 07, 05, 04 and 03 and reviews text-only ambiguities against Stage 02 page
images when needed. The book-processing handoff consists of Stage 02 images/text,
Stage 06 graphic assets and pages resolved in order from Stages 08, 07, 05, 04 and 03.

### Physical-page selection

Use `--page-ranges` to select physical pages for this invocation, counting from 1.
For example, `"2-3,7"` selects the indicated pages. Stage 01 always prepares or
reuses the complete PDF. Stages 02–08 process only the selected input pages.
Omit the range to select all available pages.

## Restart and status

Use `--from-stage` and `--to-stage` to run a range of stages. Set them to the same
value to run one stage.
Starting a stage clears its results and all later stage results, even when
`--to-stage` stops earlier. With `--page-ranges`, Stages 02–08 clear files for the
selected pages, including Stage 06 graphic crops, and retain the other pages.
Without a range, these stage directories are cleared completely. Stage 01 always
clears its whole workspace directory, while preserving the sibling OCR PDF.
Any earlier stages whose results the run retains must have successful
statuses. The pipeline still runs selected stages that finished earlier.

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

Each entry has a completion manifest that identifies the input and lists the
output hashes. If an entry is missing, incomplete or damaged, the pipeline
recomputes the result and publishes it only when complete. On a hit it copies
the files into the ordinary output paths and prints `[cache]`. The cache does
not import old outputs, so the first execution with it may recompute them.
If you delete only a prepared output, the next run restores it from cache.
Remove the matching cache entry as well to force fresh preparation with unchanged
inputs. Transcription quality remains unverified by cache reuse or successful
preparation.

`stage_status.json` records execution, page range and request metrics for the
most recent invocation only. It does not accumulate page coverage. Before
clearing outputs on restart, the pipeline invalidates completion statuses for
the selected stage and all later stages, including on a partial restart.
Book import reads all retained pages across Stages 03 through 08, regardless
of the last invocation's range. Use the following command to inspect status.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF_DIRECTORY>/source.pdf" --status
```

PDF worker responses use the fixed shared cache at
`tools/bootstrap/.cache/codex`. The cache key uses operation IDs that stay the
same if stage names or numbering change (`pdf_page_to_objects`,
`pdf_table_to_markup`, `pdf_page_proofreading`). The key also includes the
prompts, inputs, model settings and isolated execution settings. The pipeline
has no `--config`, `--cache-dir` or `--verbose` options.

Model requests contain only the stage prompt, source JSON where needed, and the
page or table image where required. Stage 08 first sends compact text objects and
short neighboring excerpts. It follows up with the page image only for pages
with unresolved items. The shared transport clears base and developer
instructions and sends no output schemas, separate type lists or examples
outside the stage prompt.
Stage 03 chooses `type` and nullable `role` independently in one image-and-text
request. Roles are `caption`, `legend`, `note` and `footnote`; they are preserved
through filtering, table transcription and final callout preparation into book
import. A legend represented as a table is transcribed as a table in Stage 05.

Agent development guidance is in the [bootstrap instructions](../AGENTS.md#converter-development).

## Tests

PDF execution, OCR profile and page-selection tests live in `tests/` beside this
program. Run them from the repository root.

```powershell
python tools/verification/run_unittest.py discover -s tools/bootstrap/pdf-to-markdown/tests -q
```

Shared conversion transport tests live in `tools/bootstrap/conversion/tests/`.
A successful stage confirms execution and leaves transcription quality unverified.
Stages 03 and 05 parse JSON without local response-schema enforcement. Stage 08
validates patch fields and target segment IDs before applying text replacements.
Shared execution and cache behavior are documented in the
[conversion client README](../conversion/README.md).

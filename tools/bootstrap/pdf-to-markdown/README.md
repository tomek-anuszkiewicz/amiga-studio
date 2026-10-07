# PDF-to-Markdown

This bootstrap program converts source PDFs into the initial reference knowledge
base. Work follows the [developer-led conversion workflow](../reference-conversion-contract.md#development-workflow):
change the converter, run only a fragment selected and requested by the user, and
let the user assess the output. Offline tests check execution mechanics, not content quality.

## Run an attempt

Install shared dependencies with `python -m pip install -r tools/bootstrap/conversion/requirements.txt`
and authenticate with `codex login`. The pinned Codex transport uses ChatGPT authentication,
isolated requests and original-detail images. Each inference stage has an explicit model/effort pair.

Use a named child of the source directory's `workspace/` container:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py `
  --pdf "<PDF_DIRECTORY>/source.pdf" `
  --workspace "<PDF_DIRECTORY>/workspace/attempt-01" `
  --config tools/bootstrap/pdf-to-markdown/config.yaml `
  --page-ranges "1-5,7" --from-stage 00 --to-stage 01
```

The CLI retains its default `<PDF_DIRECTORY>/workspace/` when `--workspace` is
omitted; agent-run conversions always supply the named attempt explicitly.
The original PDF stays outside the attempt. Stage 00 prepares all source pages in
a sibling `<source stem>-ocr.pdf`; an existing sibling skips OCR and PDF writing.
Restart cleanup never deletes that sibling. Stage 01 applies the physical page selection.

The existing attempt's `config.yaml` takes precedence over `--config`; the argument
initializes only a missing configuration. Requested inputs persist there:

```yaml
input:
  source_pdf: ../../source.pdf  # Relative to this config.yaml
  pages: [1, 2, 3, 4, 5, 7]   # null means every physical page
```

Explicit `--pdf` and `--page-ranges` update the inputs. Omitted arguments retain
configured values, including when restarting at 00. Input configuration describes
requests; it contains no output inventory or completion history. PDF configuration
is parsed without local validation. Filesystem, YAML, JSON and image/PDF-library
errors stop execution normally. No older-stage inputs or alternative models are substituted.

## Stage data and execution

Execution order is `00 -> 01 -> 02 -> 02.5 -> 02.9 -> 03`, then 04-14.
Each worker reads the predecessor's own directory and writes its own results:

| Stage | Input and result |
| --- | --- |
| [00](stages/00_text_layer/README.md) | Source PDF -> full-source sibling OCR PDF; compatible OCR recovery stays in the attempt |
| [01](stages/01_preprocess/README.md) | Prepared PDF -> physical-page PNGs and positioned text JSON |
| [02](stages/02_page_conversion/README.md) | Stage 01 -> ordered logical objects, Markdown and pixel boxes |
| [02.5](stages/02.5_page_conversion_review/README.md) | Stage 01/02 -> review frames and ordered labels |
| [02.9](stages/02.9_emit_page_markdown/README.md) | Stage 01/02 -> one unchanged-text `document.md` and original-PNG crops |
| [03](stages/03_build_raw_stream/README.md) | Stage 02 objects and Stage 01 geometry -> raw stream and initial assets |
| [04](stages/04_stream_reduction/README.md) | Raw stream -> reduced stream |
| [05](stages/05_chapter_partition/README.md) | Reduced stream -> self-contained chapters |
| [06](stages/06_detect_continuations/README.md) | Stage 05 -> continuation metadata |
| [07](stages/07_transform_tables/README.md) | Stage 06 -> formatted tables |
| [08](stages/08_transform_graphics/README.md) | Stage 07 -> transformed graphics |
| [09](stages/09_transform_prose/README.md) | Stage 08 -> formatted prose/code/TOC |
| [10](stages/10_proofread_stream/README.md) | Stage 09 -> corrected chapter titles, slugs, filenames and nodes |
| [11](stages/11_emit_markdown/README.md) | Stage 10 -> Markdown bodies and referenced assets |
| [12](stages/12_generate_properties/README.md) | Stage 11 -> Obsidian frontmatter |
| [13](stages/13_refine_first_chapter_name/README.md) | Stage 12 -> opening-section names and links |
| [14](stages/14_link_toc/README.md) | Stage 13 -> final TOC links |

Page files are enumerated in physical-page numeric order, without coverage or
PNG/JSON pairing checks. Actual chapter JSON carries `index`, `slug`, `title`,
`target_md_file` and `nodes`. Stages 06-09 preserve metadata; Stage 10 writes corrected
metadata with nodes in its own directory. Stage 11 uses those files directly.
The identity is index plus slug: both preface and TOC can use index zero.
Prior-stage files remain unchanged. Filename collision handling and Stage 12
frontmatter processing remain in place.

Stage 02 request schemas guide model output. The PDF path does not locally enforce
schemas, geometry, identifiers, response types or text content. Review images and
Markdown/crop bundles receive no completeness, dimension, asset-inventory or content
comparison pass. Successful execution does not certify page coverage, classification,
geometry, complete assets or source fidelity. The user assesses generated content.

Select the full pipeline by omitting the interval, a range with start/end, or one
stage with matching boundaries. Deterministic and inference stages use the same routing:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02 --to-stage 02.5
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.9 --to-stage 02.9
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 06 --to-stage 09
```

Stages 06-09 run automatically, including their inference calls. All real conversions
use `pipeline.py` so they receive status, metrics, cleanup and publication handling.
Stage 02.9 is a separate intermediate export; Stage 03 reads Stage 02 directly.

## Restart, status and publication

Starting a stage invalidates it and every later stage in execution order before
deleting their outputs. The end stage limits execution, not cleanup. Missing successful
external predecessors are rejected before cleanup using `STAGE_REGISTRY.inputs` and
`stage_status.json`. No completed stage is automatically skipped. Advancing the start
stage retains earlier results; changing code or input files does not recertify them.

`stage_status.json` is the only completion/metrics record. Atomic writes retain
running/success/failed and execution metrics. Success follows worker execution and
required asset operations; actual failures record failed. Normal execution creates
no conversion-state file, manifest, snapshot or chapter registry.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --status
```

Use `--cache-dir` to override the shared `.cache/codex` request cache. PDF requests
retain completion/authentication/transport handling and JSON parsing, with no local
configuration/catalog checks, schema enforcement or validator callbacks. HTML keeps its existing validations.

Final output remains in `<WORKSPACE>/14_link_toc/`. `--publish` requires completion
through Stage 14 and copies Markdown/assets to the empty or absent sibling book
directory without `-tmp`. Destination protection remains enforced before execution
and when publishing. Independent attempts retain their own outputs.

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

Physical page 19 of the 160-page test book passed all 14 stages using the shared Codex client. The page exercised native extraction, segmentation, table transcription, prose formatting, properties and naming; stages with no matching work still executed. Eight live requests were needed across the pilot. This verifies pipeline integration for one page under the pilot's implementation. Transcription quality, scanned OCR, graphics, real TOC linking, multi-page continuations and recovery remain separate validation work; no full-book conversion or indexing ran.

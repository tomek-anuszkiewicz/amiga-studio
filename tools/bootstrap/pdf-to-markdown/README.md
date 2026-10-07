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

Execution order is `00 -> 01 -> 02 -> 02.4 -> 02.5 -> 02.8 -> 02.81 -> 02.82 -> 02.9 -> 03`, then 04-14.
Each worker reads the predecessor's own directory and writes its own results:

| Stage | Input and result |
| --- | --- |
| [00](stages/00_text_layer/README.md) | Source PDF -> full-source sibling OCR PDF; compatible OCR recovery stays in the attempt |
| [01](stages/01_preprocess/README.md) | Prepared PDF -> physical-page PNGs and positioned text JSON |
| [02](stages/02_page_conversion/README.md) | Stage 01 -> ordered logical objects, Markdown and pixel boxes |
| [02.4](stages/02.4_reclassify_callouts/README.md) | Stage 01/02 -> vision-assisted advisory range replacements on changed pages only |
| [02.5](stages/02.5_page_conversion_review/README.md) | Stage 01/02 -> review frames and ordered labels |
| [02.8](stages/02.8_filter_page_content/README.md) | Stage 01/02 -> retained objects after source-content and fixed NXP-logo exclusions |
| [02.81](stages/02.81_transform_page_tables/README.md) | Stage 01/02.8 -> table markup, saved crops and inferred HTML group text |
| [02.82](stages/02.82_table_conversion_review/README.md) | Stage 01/02.81 -> side-by-side table review PNGs, no inference |
| [02.9](stages/02.9_emit_page_markdown/README.md) | Stage 01/02.81 -> `document.md`, converted tables and required raster assets |
| [03](stages/03_build_raw_stream/README.md) | Stage 02.81 objects and Stage 01 geometry -> raw stream and initial assets |
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

Stage 02 request schemas guide model output. Stage 02.4 interprets explicit edit
ranges and retains ambiguous proposals; this does not certify text fidelity.
The PDF path does not locally enforce
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
Stage 02.81 transcribes exact table crops and separately infers complete HTML group
text. Stage 02.82 reviews saved results with a local browser and no model calls.
Stage 02.9 is a separate intermediate export; it and Stage 03 read Stage 02.81
objects directly, using Stage 01 geometry and original PNGs. HTML tables publish
collapsed literal-text and original-image companions after their source groups.
Markdown tables need no published crop. Tables retain their separate converted
fragments through stream reduction, continuation detection and Stage 07.
The explicit whole-input alternative `--table-predecessor 02` requires restart at
02.81 when filtering was deliberately skipped; default remains completed 02.8.
Existing attempt settings are retained; missing 02.4/02.81 model settings are added
only when their stage is selected. Review backend setup is in the Stage 02.82 README.
Stage 02.5 uses resolved complete page input for type-colored review annotations,
consolidates unobstructed consecutive prose, and hides header/footer/thumb-index
annotations without removing source content. Stage 02.8 owns all source-content removal; Stage 04 retains
graphics union and prose seam processing.

Stage 02.8 applies the user's deliberate source-fidelity exception: remove
headers/footers, all objects before the first selected-input `toc_heading`, and
entire pages containing a list-of-tables, list-of-figures or index object/heading.
After those rules, standalone upper-left `graphic` objects matching the bundled
NXP raster are excluded using original Stage 01 PNGs. Fixed geometry, foreground
aspect and RGB gates must all pass, without configuration. See
[Stage 02.8](stages/02.8_filter_page_content/README.md) for thresholds and evidence
limits. Stage 01/02 completion is required.
The boundary and page triggers come from resolved page objects and exact types.
A TOC heading on an excluded page still defines the boundary. Without a TOC heading,
skip pre-TOC removal. Retain the actual TOC subject to whole-page exclusions.
Surviving fields, IDs, text and geometry remain unchanged; empty pages are omitted
and an empty filtered result is valid. Generated YAML metadata remains unchanged.

Stage 02.4 sends complete original Stage 01 PNGs and numbered-frame companions
with frozen source texts to recover independent advisory ranges. It writes sparse
complete-page overrides without modifying Stage 02. Review 02.5, filtering 02.8
and explicit table predecessor `02` enumerate Stage 02 pages and select completed
same-name overrides. Absent 02.4 status uses Stage 02; running/failed status blocks
direct consumers. Filtering output remains authoritative for default 02.81 input:
omitted pages are never restored. See the [02.4 contract](stages/02.4_reclassify_callouts/README.md)
for decision reports and replacement provenance. To deliberately skip 02.4,
finish at 02 and restart at 02.5 with no current 02.4 status.

## Restart, status and publication

Starting a stage invalidates it and every later stage in execution order before
deleting their outputs. The end stage limits execution, not cleanup. Missing successful
external predecessors are rejected before cleanup using `STAGE_REGISTRY.inputs` and
`stage_status.json`. No completed stage is automatically skipped. Advancing the start
stage retains earlier results; changing code or input files does not recertify them.
Restart 02.8 clears its filtered objects and every later result even when ending at
02.8. Existing 02.9/03 exports require explicit regeneration through 02.81 to
represent table conversion; restart 02.82 preserves 02.81 and clears later stages; implementation does not mutate saved attempt artifacts.
Preserve edited Stage 02.9 bundles outside cleanup before restarting. Regeneration
rebuilds image links/assets and removes obsolete assets; the bundled NXP resource
survives attempt cleanup.

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

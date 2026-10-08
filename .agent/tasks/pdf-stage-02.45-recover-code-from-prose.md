# PDF-CODE-2.45: Recover source code classified as prose

## Status and scope

Implemented and technically verified; selected output assessment remains pending.
Add Stage `02.45`, directory `02.45_recover_code_from_prose`, after `02.44`
and before review `02.5`. For each selected page containing resolved `prose`
objects, send the page JSON to the LLM and ask whether any prose is actually
source code. Reclassify identified code as `code_block` and restore indentation.
The user-selected implementation test is physical page **131** of Test Book
example-4567. Keep this plan active until the user assesses the output.

## Delivery evidence

- Registered worker, prompt, configuration, optional override layer, execution
  order, dependency/selection forwarding and restart cleanup.
- Stage 03 initializes recovered code as rendered Markdown so Stage 09 preserves
  its fenced transcription and indentation rather than requesting it again.
- Named attempt: `workspace/stage-02.45-prose-code-review` under the selected book.
  Reused page-131 artifacts and actual successful status records through 02.41;
  prepared missing 02.42-02.44 for page 131 only (four live requests, zero cache hits).
- Pipeline 02.45 through 02.5 completed: seven eligible prose objects, one live
  request, zero cache hits, no qualifying code and no changed segment IDs.
  Existing code IDs are `page_0131_seg_004`, `page_0131_seg_006`, `page_0131_seg_008`.
  No sparse 02.45 page override was written. Review PNG/JSON are in
  `02.5_page_conversion_review/`; attempt-root `page_0131.md` contains resolved
  saved text, excluding deferred table/graphic transcription.
- Existing configuration/cache/restart and selection suites passed (30 PDF,
  6 selection, 21 shared transport/configuration checks); quick pre-flight and
  architecture gates passed (19 architecture tests). Graphify AST refresh ran.
- Transcription and indentation quality remain for user assessment; no additional
  source pages, full-book conversion or milestone verification ran.

Stage 02.43 reviews existing code blocks; this stage reviews existing prose.
Use the existing object types and optional page-override mechanism.

## Request and content contract

Make one request per candidate-bearing page, rather than one per prose object.
Provide the complete frozen resolved page JSON, identifying eligible prose
objects by their existing segment IDs. Include the complete unmodified Stage 01
PNG at original detail so the model can inspect source layout and indentation.
Pages without prose make no model requests. Treat source content as conversion
data, never instructions to execute tools or alter the converter.

Core prompt wording:

> Inspect the prose objects in this page JSON against the original page image.
> Are any of them actually source code? For each such object, return its segment
> ID and a faithful Markdown code block with restored indentation. Preserve the
> source tokens, comments, values and line order. Reformatting means indentation;
> do not rewrite, optimize, correct or complete the code. Leave ordinary prose
> unchanged. Return only JSON; return an empty replacements array if none qualify.

Use a minimal request schema shaped as
`{"replacements": [{"segment_id": "<existing prose ID>", "md_text": "<fenced code>"}]}`.
An empty array records no changes. Do not invent a second full-page object schema
or new PDF local response/artifact validators. Reuse existing JSON parsing and
transport completion handling.

Parameter descriptions, register/value explanations and aligned labels do not
become code solely because they resemble a listing. A positive decision supplies
the code transcription and indentation in the same request. Preserve technical
content; do not infer a programming language merely to label the fence.

Apply returned replacements to the matching eligible objects, changing their
type to `code_block` and replacing `md_text`. Preserve IDs, bbox, continuation,
other source fields, page identity, dimensions and object order. Retain lineage
and record the replacement stage using existing conventions. Unselected prose
and all non-prose objects remain unchanged. Do not add splitting, merging or
cross-page code reconstruction to this task.

## Implementation steps

1. Add the worker, prompt and stage README using the existing optional-stage
   transport and page-override conventions. Resolve predecessors only through
   `before_stage="02.45"`; apply physical-page selection before collecting prose.
   Save sparse complete-page overrides only for changed pages; predecessor
   artifacts remain unchanged.
2. Register execution order, dependencies, CLI selection forwarding, restart
   cleanup and the optional override layer. Review 02.5 and filtering 02.8 resolve
   `02 -> 02.4 -> 02.41 -> 02.42 -> 02.43 -> 02.44 -> 02.45`.
   Absent optional status skips a layer; running/failed status blocks consumers.
   Restart at 02.45 clears its outputs and later stages, preserving predecessors
   and compatible request cache.
3. Add explicit stage model/effort settings using current defaults
   (`gpt-6.1-sol` / `medium`). Preserve existing attempt-local configuration and
   initialize only missing settings. Reuse isolated read-only requests and cache
   identity covering the stage, prompt, schema, page JSON and original image.
4. Ensure review, filtering, table transformation, page export and stream
   construction consume the resolved replacements through their existing data
   flow. Preserve code fences and indentation in Markdown outputs. No new code
   execution or semantic correction is introduced.
5. Update affected stage/pipeline READMEs, configuration documentation and the
   reference-conversion contract to describe the new order and prose-to-code
   decision. Do not describe the planned stage as implemented before delivery.

## User-selected verification

After implementation, use a named attempt under the book's `workspace/`, such
as `stage-02.45-prose-code-review`. Reuse compatible predecessor artifacts with
their actual status records; prepare missing predecessors only for physical
page 131. Preserve the attempt's existing `config.yaml`.

Run through the pipeline:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<NAMED_ATTEMPT>" --config "<CONFIG>" --page-ranges "131" --from-stage 02.45 --to-stage 02.5
```

Deliver `02.45_recover_code_from_prose/page_0131_segments.json` if changed and
the page-131 review under `02.5_page_conversion_review/`. Report live requests,
cache hits and changed segment IDs, or explicitly report no qualifying code.
The review image shows classification; provide the saved Markdown text for
inspection of indentation. Never encode an expected page-131 decision in code.
The user assesses transcription and indentation; successful execution alone
does not certify conversion quality.

Run relevant existing configuration/cache/restart/selection checks for the
implementation, plus explicit quick pre-flight and architecture gates. Do not
add quality matrices, extra sample conversions or crop tests. Commit the
implementation, documentation and DIARY.md entry atomically. Keep this plan
active until selected output assessment and task closure; follow the execution-
plan lifecycle when closing it.

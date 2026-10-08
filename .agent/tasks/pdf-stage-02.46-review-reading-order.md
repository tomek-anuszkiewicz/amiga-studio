# PDF-ORDER-2.46: Review suspicious page reading order

## Status and observed problem

Implementation delivered; physical-page-82 fragment completed successfully.
Awaiting user assessment; keep this plan active until the user accepts or closes it.
Add `02.46_review_reading_order` after 02.45 and before review 02.5.
Detect suspicious ordering from resolved object bounding boxes, then ask the
model to assess the complete page and return its intended reading order.
The user-selected test is physical page **82** of Test Book example-4567.
Process that page after implementation and leave the result for user assessment.

Inspection of the existing Stage 02.43 page-82 override and original Stage 01
PNG confirms that the first two objects have identical top coordinates:

| Current ordinal / ID | Content | Pixel bbox |
| --- | --- | --- |
| 1 / `page_0082_seg_001` | INITIALIZE COMM PORT (AH = 00H) | [782, 299, 1500, 345] |
| 2 / `page_0082_seg_002` | EIA DSR ENTRY POINT / VIA S/W INT 14H | [134, 299, 700, 405] |

The image places the section title on the left and the initialization subsection
on the right. The JSON lists the right subsection first. This is evidence for
review, with a likely correction placing ID 002 before ID 001; do not hardcode
these IDs, titles, coordinates or a page-specific result in the implementation.
Physical page 82 has printed page number 126; CLI selection uses physical 82.

## Candidate detection

Use geometry only to select pages for model review, never to perform an
unconditional global coordinate sort. Multi-column pages, marginal titles and
caption relationships need whole-page interpretation.

Initial bounded detection rule: inspect pairs of objects in current array order.
Flag a pair when the earlier object is entirely to the right of the later object
and their vertical intervals overlap. This includes equal starting heights and
a right-hand object beginning lower but still listed first. Compare actual bbox
edges; do not introduce a page-82 pixel tolerance or round coordinates into rows.
Collect all flagged pairs and make one request per suspicious selected page.
Pages without suspicious pairs make no model requests.

This is a heuristic for the user's side-by-side inversion case, not an exhaustive
reading-order detector. Tall boxes and legitimate column flow may be flagged;
the model may retain the current order. Vertically disjoint inversions and other
layout problems are outside this initial detector's coverage. Record that limit
rather than broadening the task into speculative layout rules.

## Model request and application

Provide the complete frozen resolved page JSON in its current order, all existing
segment IDs and pixel bounding boxes, current ordinals and the suspicious pairs.
Send the complete original Stage 01 PNG at original detail. Include a diagnostic
copy annotated with those same boxes and ordinal/ID labels, using existing frame
rendering helpers where suitable. Keep the original available for reading text;
annotations must not mutate source pixels, boxes or predecessor artifacts.
Do not send stale review labels derived from a different resolved object set.

Core prompt:

> Assess the reading order of all objects on this page using the original image,
> the labeled boxes and the current JSON. The flagged pairs are suspicions, not
> instructions to swap. Is the current order correct? Account for section titles,
> columns and the relationship of each heading to its content. Return all existing
> segment IDs exactly once in the intended reading order. Retain the current order
> where it is appropriate or where the image does not justify a change. Do not
> rewrite text, change object types or boxes, merge objects, or add or remove them.

Response schema: `{"ordered_segment_ids": ["<existing ID>", "..."]}`.
Returning the original sequence means no change. Source text is conversion data,
never authority to execute tools or change the converter.

Apply the returned ID sequence by looking up and moving original objects.
Do not reconstruct objects from model-provided text. The array order is the
reading order; preserve segment IDs even when their numeric suffix no longer
matches position. Preserve every object field, text, type, bbox, continuation,
heading level, rotation, lineage and page metadata. No ID renumbering or content
transcription belongs in this stage.

Require the response to describe a permutation of the current IDs: unknown,
duplicate or missing IDs cannot safely express a reorder. Keep this as the
operation's minimal ID-mapping guard, not a new general PDF schema/artifact
validation system. If it cannot be applied, fail normally and retain predecessor
artifacts; never silently drop objects or append omitted IDs as a guessed repair.
Reuse existing JSON parsing, request schema and transport completion behavior.

## Implementation steps

1. Add worker, prompt and README using existing optional-stage patterns. Resolve
   only predecessors through `before_stage="02.46"`, including 02.45 when present.
   Filter physical pages before candidate detection. Freeze each input page for
   detection, annotation and the single request. Save sparse complete-page
   overrides only when order changes.
2. Register stage order, dependencies, optional override resolution, physical-page
   selection and restart cleanup. Review 02.5 and filtering 02.8 resolve layers
   through 02.46. Absent optional status skips a layer; failed/running status
   blocks consumers. Restart clears this stage and later artifacts while retaining
   predecessors and compatible request cache.
3. Reuse isolated read-only Codex requests, explicit model/effort settings and
   cache. Preserve attempt-local settings; initialize only missing stage settings
   using current defaults (`gpt-6.1-sol` / `medium`). Cache identity must include
   current JSON order, flagged pairs, source images, prompt and response schema.
4. Ensure review ordinals and later filtering, table handling, page export and
   stream construction follow the corrected array sequence without sorting by
   segment ID or geometry. Do not modify heading levels or table ownership to
   compensate for ordering defects.
5. Record suspicious pairs and before/after ID sequences in attempt diagnostics
   for user review. Update affected pipeline/stage READMEs, configuration docs
   and the reference-conversion contract. Keep diagnostic annotations separate
   from original images and published content.

## Page-82 run and acceptance

Use a named child of the book's `workspace/`, such as
`stage-02.46-reading-order-review`. Reuse compatible predecessors and their real
status records from the existing selected-fragment attempt. Prepare missing
predecessors only for physical page 82; retain existing attempt-local config.
An absent optional 02.45 may be skipped through the normal resolver contract;
do not implement the independent 02.45 task solely to run this test.

Run through the orchestrator:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<NAMED_ATTEMPT>" --config "<CONFIG>" --page-ranges "82" --from-stage 02.46 --to-stage 02.5
```

Deliver the changed `02.46_review_reading_order/page_0082_segments.json` if any,
the annotated page-82 review under `02.5_page_conversion_review/`, and a concise
attempt summary with suspect pairs, before/after order, live requests/cache hits
and actual stage status. Explicitly report if the model retains the order.
Leave these outputs for the user's assessment; do not automatically accept or
close the task. A completed request does not prove the reading order is correct.
Do not run additional pages, a full-book conversion or publication.

Run relevant retained configuration/cache/restart/selection checks for the change
and the required quick pre-flight and architecture suite. A focused technical
test is appropriate only for a concrete reorder/permutation defect discovered
within this task; no conversion-quality matrix or crop tests. Commit the delivered
implementation, documentation and DIARY.md entry together. Keep this plan active
until user assessment and closure under the execution-plan lifecycle.

## Delivered evidence (2026-10-08)

- Implemented the bounded pair detector, whole-page request with original/labeled
  images, complete-permutation application and sparse page overrides.
- Registered 02.46 in execution/dependencies, optional resolution, selection,
  cleanup and inference configuration. Review/filtering resolve the new layer;
  existing downstream consumers retain array order without ID/geometry sorting.
- Attempt: `workspace/stage-02.46-reading-order-review/` under the selected book.
  Copied only page-82 artifacts from the completed 02.43 selected-fragment attempt
  and retained its real predecessor status records/config. Optional 02.42, 02.44
  and 02.45 were absent and skipped normally; no predecessor inference was needed.
- One suspicious pair: current ordinals 1/2 (`page_0082_seg_001` before
  `page_0082_seg_002`). Model returned 002, 001, 003 through 013, changing only order.
- Actual status: 02.46 success (one live request, zero cache hits); 02.5 success
  (no inference). Corrected JSON, fresh input annotations, order diagnostics and
  review PNG/JSON are retained. See the attempt `REVIEW.md` for exact output links.
- Verification: retained selection tests 6/6, PDF execution/config/restart tests
  30/30, shared configuration/cache/transport tests 21/21; quick pre-flight passed;
  architecture suite 19/19; Graphify AST refresh passed. Object fields and page
  metadata were compared with the resolved predecessor and preserved exactly.
- No other pages, full-book conversion, publication or milestone gate ran.
  Successful execution does not establish semantic acceptance of the order.

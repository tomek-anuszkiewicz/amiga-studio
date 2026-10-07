# Stage 02.4: simplify callout recovery

Task ID: PDF-CALLOUT-SIMPLE-2.4.
Status: planned; implementation and user-selected fragment review pending.
Created: 2026-10-07.

## Result

Ask a short visual question about potential advisory labels using one original
page image and existing text-object IDs and boxes. Return only the replacements
needed to represent confirmed advisories as Markdown callouts. Save complete
modified Stage 02 page JSONs in 02.4 only when a callout changes. Downstream
consumers use the same-name 02.4 JSON when available in a successful layer,
otherwise the original Stage 02 JSON.

This replaces the closed PDF-CALLOUT-2.83 plan. Its implementation remains active
until this change is implemented. The previously inspected user run completed
with zero changed pages; that result does not establish classification quality.
This plan authorizes no source conversion or model run.

## Request and prompt

- Retain candidate discovery across the existing supported text roles. Match
  whole advisory words case-insensitively, including decorated labels. Keep
  NOTE, CAUTION and WARNING plus labels discovered on existing callout objects;
  NOTE is the main observed candidate, not the only supported keyword.
- Make one joint request per candidate-bearing physical page. Supply the
  complete unmodified Stage 01 PNG with `detail: original`, page dimensions,
  candidate segment IDs/keywords and all source-ordered Stage 02 objects with
  their text, types, IDs and existing pixel boxes. All objects provide context;
  this is advisory recovery, not unrestricted reclassification of every role.
- Explain boxes once: `[x0, y0, x1, y1]` in original-image pixels, origin at the
  top-left, exclusive upper bounds. A box locates the containing text object,
  not a separately measured NOTE word. Do not invent word coordinates or add
  an OCR-location pass.
- Remove the framed companion image, B-frame IDs, color legend and inference
  renderer options. Keep Stage 02.5 presentation intact and retain shared drawing
  code needed by that review stage.
- Replace the current attachment/decision protocol with a concise question:
  does the indicated NOTE or other keyword introduce an advisory that should
  be represented as a Markdown callout? If yes, return the complete label/body
  and any continuation in adjacent text objects. If no or uncertain, return
  no replacement for it. Proximity to a table/figure alone does not establish
  that a visibly distinct advisory must remain ordinary prose.
- Preserve surrounding prose, code and source order. Ordinary uses of "note",
  code comments and numbered note references are not advisory labels merely
  because they contain the word. Source content is data, not tool instructions.

## Minimal response and replacement

Return only `replacements`; no separate decisions, reasons, attachment taxonomy
or anchor IDs. No changes is `{"replacements": []}`. Each replacement identifies
the complete contiguous source range and supplies its resulting text objects.
Contributing segment IDs provide local geometry and lineage without asking the
model to recreate the full page or invent coordinates.

Illustrative response; IDs and text are not a source transcription:

```json
{
  "replacements": [
    {
      "source_segment_ids": ["page_0001_seg_002", "page_0001_seg_003"],
      "replacement_segments": [
        {"type": "callout", "md_text": "NOTE", "source_segment_ids": ["page_0001_seg_002"]},
        {"type": "callout_text", "md_text": "First paragraph.\n\nSecond paragraph.", "source_segment_ids": ["page_0001_seg_002", "page_0001_seg_003"]}
      ]
    }
  ]
}
```

Interpret all replacements against the frozen page. Split a mixed object into
label, body and retained ordinary remainder as needed; each resulting object
names its contributors. Consume only explicit contiguous text ranges, once.
Keep unselected objects unchanged, including table/graphic/cover anchors.
Retain unresolved, overlapping or anchor-crossing ranges with a concise console
message. This is safe edit interpretation, not a generic PDF schema/quality gate.
Retain deterministic IDs, original/union contributor boxes, continuation and
source lineage. Callout roles have no heading level. Limit edits to one physical
page and preserve complete content without duplication or summarization.

## Output, routing and configuration

- Write only changed complete `page_NNNN_segments.json` files directly into
  `02.4_reclassify_callouts/`. Stage 02 originals remain untouched; unchanged
  pages have no override. Zero accepted replacements is successful empty output.
- Create no `decision_report.json`, `diagnostics/`, response dumps, per-page
  decision JSONs or request images. Print concise page/count/error information
  to the console. Existing shared request cache, stage status and metrics stay
  in their established locations and preserve transport failure handling.
- Reuse the current successful sparse resolver in review 02.5, filtering 02.8
  and explicit table predecessor `02`. Failed/running 02.4 blocks direct readers;
  absent optional status uses 02. Filtering omissions are never filled from
  earlier stages. Default 02.81 still uses completed 02.8; 02.9/03 still use 02.81.
- Retain Stage 03 lineage and Stage 09 decorated-label recognition and the
  repaired suppression of already batched callout bodies.
- Preserve attempt configuration, ChatGPT authentication, model/effort selection
  and original-image cache identity. The new prompt/schema and one-image request
  distinguish requests from old two-image responses without deleting the cache.
  Missing source imagery fails rather than switching to text-only inference.
- Existing restart invalidation clears old 02.4 diagnostics and all later output
  while preserving 01/02 and cache. Do not rewrite existing attempts during
  implementation; regeneration occurs only at the user's request.

## Implementation and verification

Update `common/pdf_callouts.py`, the 02.4 worker/prompt, obsolete inference-only
renderer options and affected converter/stage/developer documentation. Keep
the stage registration, sparse routing and review presentation. Remove obsolete
definitions instead of retaining a parallel legacy inference path.

Run compilation/CLI checks and relevant retained technical tests. Use one
bounded synthetic check to confirm the one-image request, explicit replacement
of mixed/multiple text blocks, unchanged neighboring objects, empty no-change
output and downstream routing. Do not create quality matrices, default test
expansion or reinstate removed PDF schema/crop gates. Run quick preflight and
the separate architecture suite explicitly, refresh Graphify after source edits,
review the diff and commit with a diary entry.

At the user's request, run only the source fragment and stopping stage they
select in a named attempt. Report changed page filenames and execution errors;
the user evaluates whether the callouts are correctly identified and preserved.
Keep this plan active until implementation and requested review finish, or the
user explicitly closes the remaining scope.

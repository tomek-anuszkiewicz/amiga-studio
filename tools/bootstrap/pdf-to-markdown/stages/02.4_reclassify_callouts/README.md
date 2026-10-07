# Stage 02.4: recover advisory callouts

After completed Stage 01/02, scan other text roles for whole-word NOTE, CAUTION,
WARNING and additional leading labels found on existing callout objects in the
selected input. A hit triggers review, not reclassification. Existing callout
objects provide context. Tables, graphics, covers and other non-target roles are
read-only anchors.

Apply this invocation's `--page-ranges` before reading Stage 02 pages,
collecting keywords or issuing requests. An omitted selection reads all
available predecessor pages. A later-stage restart with `--page-ranges 64-64`
therefore processes only physical page 64 even in an all-page workspace.
Selection is not persisted; legacy `input.pages` has no effect.

Each candidate-bearing page receives one joint Codex vision request containing
the complete unmodified Stage 01 PNG, actual image dimensions, candidate segment
IDs/keywords and all source-ordered objects with text, types, stable IDs and boxes.
Boxes locate containing objects in original-image pixels, not individual words;
their top-left origin and exclusive upper bounds are explained once. The image
uses original detail, ChatGPT authentication, existing cache and metrics. Its bytes,
the new prompt and replacement schema participate in cache identity.

The [prompt](prompt.md) asks whether each keyword introduces a Markdown advisory
callout. Numbered/symbol-marked footnotes, table/figure legends and explanatory
lists remain in their source roles. A visually distinct NOTE label with its own
advisory paragraph is recovered even when it discusses a nearby table or figure;
classification follows the label and presentation rather than the subject.
Return only explicit contiguous replacements using source_segment_ids; no or
uncertain advisories return no replacement. The program interprets proposals
against the frozen input. Unresolved, overlapping or anchor-crossing ranges
remain unchanged with a concise console message. Replacement text comes from
the model; there is no local transcription-quality certification or generic schema gate.
Labels become `callout`, complete bodies become `callout_text`, and any ordinary
remainder keeps its source role. Replacement IDs retain the first contributor
where available, with deterministic suffixes for splits. Each replacement carries
its own model-provided `bbox` in original-image pixels: the standalone label,
complete body and ordinary remainder receive separate tight boxes from the image.
The program saves these coordinates directly; contributor boxes provide request
context and do not determine replacement geometry. The prompt and schema change
invalidates previous cached responses; existing overrides require an explicit
restart at 02.4 to regenerate their boxes.
Continuation and source segment lineage are retained; cross-page restructuring
is outside scope.

Write complete `page_NNNN_segments.json` overrides only for changed pages.
No request images, response dumps, diagnostics directory or decision reports
are created. Page/candidate/change counts and retained-range errors go to the
console. No candidates or no accepted changes means successful empty output.
Shared request cache, stage status and metrics retain their established locations.
Missing imagery and transport/parsing failures fail the stage normally.

Review 02.5 and filtering 02.8 resolve the
authoritative Stage 02 filenames through completed sparse overrides. An absent
02.4 status means deliberately skipped; running/failed status blocks these inputs.
Filtering omissions are never filled from earlier stages. Stage 02.81 builds table
groups and companions from the corrected objects, then 02.9/03 consume its output.
Stage 03 carries replacement lineage in node metadata; Stage 09 assembles native
callouts and recognizes decorated leading advisory labels.

Run through the orchestrator in a named attempt:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.4 --to-stage 02.4
```

Restart clears 02.4 and every later stage, preserving 01/02 and the request cache.
Existing attempt model/effort selections stay intact; missing stage settings are
initialized to `gpt-6.1-sol` / `medium` only when 02.4 is selected. The user selects
real fragments and assesses recovery quality.

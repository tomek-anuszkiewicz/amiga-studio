# Stage 02.4: recover advisory callouts

After completed Stage 01/02, scan other text roles for whole-word NOTE, CAUTION,
WARNING and additional leading labels found on existing callout objects in the
selected input. A hit triggers review, not reclassification. Existing callout
objects provide context. Tables, graphics, covers and other non-target roles are
read-only anchors.

Each candidate-bearing page receives one joint Codex vision request containing
all source objects, frame IDs, unchanged text, pixel boxes and page metadata.
Images are the complete original Stage 01 PNG followed by a newly generated
original-scale framed companion. The shared renderer draws thin external outlines
and a side legend mapping B1, B2, ... to stable segment IDs; colors convey identity.
Inference frames neither group prose nor suppress objects as review 02.5 does.
Both images use original detail, ChatGPT authentication, existing cache and metrics.
Their bytes and the full request context participate in cache identity.

The [prompt](prompt.md) asks for candidate decisions, attachment evidence and
explicit contiguous multi-object replacements. The program interprets proposals
against the frozen input. Unresolved, overlapping, contradictory or anchor-crossing
ranges remain unchanged and are reported. Replacement text comes from the model;
there is no local transcription-quality certification or generic schema gate.
Labels become `callout`, complete bodies become `callout_text`, and any ordinary
remainder keeps its source role. Replacement IDs retain the first contributor
where available, with deterministic suffixes for splits. Geometry is original or
the contributors' enclosing union, including coarse shared geometry for splits.
Continuation and source segment lineage are retained; cross-page restructuring
is outside scope.

Write complete `page_NNNN_segments.json` overrides only for changed pages.
`diagnostics/` contains framed PNGs, raw JSON responses and per-page decision
reports with exact frame mappings, proposals, applied/retained ranges and output
objects. Original request PNGs remain in `01_preprocess/`; reports reference them.
Aggregate counts are printed to the console. No candidates or no accepted changes means
success with an empty override set. Missing imagery and transport/parsing failures
fail the stage normally.

Review 02.5, filtering 02.8 and the explicit table predecessor `02` resolve the
authoritative Stage 02 filenames through completed sparse overrides. An absent
02.4 status means deliberately skipped; running/failed status blocks these inputs.
Filtering omissions are never filled from earlier stages. Stage 02.81 builds table
groups and companions from the corrected objects, then 02.9/03 consume its output.
Stage 03 carries replacement lineage in node metadata; Stage 09 assembles native
callouts and recognizes decorated leading advisory labels.

Run through the orchestrator in a named attempt:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.4 --to-stage 02.4
```

Restart clears 02.4 and every later stage, preserving 01/02 and the request cache.
Existing attempt model/effort selections stay intact; missing stage settings are
initialized to `gpt-6.1-sol` / `medium` only when 02.4 is selected. The user selects
real fragments and assesses recovery quality.

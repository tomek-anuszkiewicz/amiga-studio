# Stage 02.44: Detect image rotation

Record an optional numeric `rotation` on each resolved `graphic`, including
graphics produced by table reclassification. The value is the correction to
apply clockwise from its current orientation in the original Stage 01 PNG:
`0` needs no correction, `90` is clockwise, `180` is a half turn and `270` is
counterclockwise. Fractional values in `[0, 360)` are allowed. Missing metadata
means no recorded decision; an explicit zero records a decision.

Send one isolated read-only request per graphic using its JSON/bbox, frozen
resolved page objects and the complete unmodified Stage 01 PNG at original detail.
Tables, covers and textual objects are excluded. Physical page selection precedes
candidate collection; pages without graphics make no requests. The response
schema contains only numeric `rotation`; existing parsing and transport completion
handling apply, without local PDF schema validation. Cache identity includes the
stage, model/effort, prompt, schema and original image bytes.

Only changed pages receive complete-page sparse overrides in
`02.44_detect_image_rotation/page_NNNN_segments.json`; adding zero is a change.
Preserve IDs, types, boxes, text, continuation, dimensions and order. Existing
source IDs survive, and `replacement_stage` records this contribution.

The shared resolver applies `02 -> 02.4 -> 02.41 -> 02.42 -> 02.43 -> 02.44`
for review 02.5 and filtering 02.8. This worker resolves only predecessors.
Absent optional status skips a layer; failed/running status blocks consumers.
Restart clears 02.44 and later artifacts while retaining predecessors and cache.
Existing attempt settings are preserved; missing stage settings initialize to
`gpt-6.1-sol` / `medium`.

Filtering 02.8 and table transformation 02.81 retain graphic metadata; review
02.82 reads those pages without changing them. Stage 03 carries `rotation` on
the node and in its metadata. Review 02.5 labels show the saved angle.
Original boxes and all review/crop pixels remain unchanged. Applying the angle
to exported assets is deferred; a completed request does not certify orientation.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --page-ranges "134" --from-stage 02.44 --to-stage 02.5
```

Inspect the sparse override and `02.5_page_conversion_review/page_0134_review.png`.
The user evaluates the orientation decision.

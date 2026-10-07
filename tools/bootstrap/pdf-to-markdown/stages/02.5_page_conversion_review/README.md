# Stage 02.5: Independent Page Conversion Review

Consume resolved Stage 02/02.4/02.41/02.42/02.43 JSON and the exact Stage 01 PNG. Completed sparse
02.4, 02.41, 02.42 and 02.43 overrides replace same-name pages in that order; absent optional
status skips that layer, while failed/running status blocks execution. Save `02.5_page_conversion_review/page_NNNN_review.png` at original
page height and twice its width, with a blank equal-width right panel.

Read only physical pages selected by this invocation's `--page-ranges`;
omission reads all available predecessor pages, regardless of legacy `input.pages`.
The pipeline forwards the parameter even when restarting directly at 02.5. Resolve sparse
02.4/02.41/02.42/02.43 overrides within that selection before rendering.

Each displayed type has a fixed distinct color for its containing 2-pixel frame,
straight leader and label, independent of page and object order. Labels show
source ordinals, type, optional heading level and `continuation: true` when any
contributing object has that flag. False flags are omitted. A merged run uses
an ordinal range, such as `3-5. prose`; other labels retain their source ordinal.
Labels contain neither text nor coordinates. JSON source order is preserved.

Only consecutive `prose` objects in source order can form one review annotation.
For the current run and the next prose object, compute the smallest enclosing
rectangle. Join them only if no other source object's rectangle overlaps that
union. Inspect the complete page, including hidden annotations and later objects,
and exclude every contributing source object in the proposed run from the check.
An obstacle closes the current run; start the next run at the rejected prose
object and continue. A non-prose object in source order also ends the run.
There are no alignment, column, width, height or spacing thresholds. Overlap
between contributing prose objects does not block joining; only other objects
do. Rectangle edges touching without a positive-area intersection are permitted.
Other roles always remain separate; source text is never assembled or changed.

`header`, `footer` and `thumb_index` receive no frames, leaders or labels.
Their original pixels remain visible, including where another leader would
cross their boxes. They still participate in reading-order and overlap decisions.
The source page remains at original scale with an equal-width right label panel.
All bounds derive from resolved page boxes; no OCR geometry is synthesized.

Each `page_NNNN_review.json` sidecar maps annotations to one-based
`source_ordinals` and corresponding `source_ids` from `segment_id` (falling back
to `id`, or null if neither exists), recording union bounds, type, heading level
and combined continuation.
It also records `hidden_source_ordinals`. These are review-only records; Stage 02
JSONs and downstream conversion content remain untouched. The low-level
`draw_review_image` draws exactly the objects supplied by its caller, without
grouping or suppression, and is shared with 02.4 inference, which supplies every original request object.
The shared renderer supports explicit frame-ID legends and colors independent of roles.

No inference, segmentation correction, sorting or line rerouting occurs. The
worker performs no completeness, dimension, frame or label validation; drawing
and image-library errors propagate normally.

Use the [orchestrator](../../README.md) with `--from-stage 02.5 --to-stage 02.5`
after Stage 01/02. Restart 02 invalidates this review and downstream stream artifacts.
Restart 02.5 clears this review and every later stage, including 02.8, 02.81, 02.82, 02.9 and 03, even for a review-only interval. Start the next run at 02.8 to retain this review while building the required filtered/table outputs. Stages 02.9 and 03 consume 02.81 directly; cleanup follows execution order. Review PNGs are deterministic; user assessment remains separate.

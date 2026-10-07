# Stage 02.5: Independent Page Conversion Review

Consume Stage 02 JSON and the exact Stage 01 PNG. Save `02.5_page_conversion_review/page_NNNN_review.png` at original
page height and twice its width, with a blank equal-width right panel.

Labels show every object's ordinal, type, optional heading level and
`continuation: true` only when the flag is true, in exact JSON
array order. Every object receives a containing 2-pixel frame and straight review
leader, using the local renderer with an identity pixel transform and
continuation labels enabled. This includes prose, captions, table
legends, footnotes, headers and footers. All geometry comes from the Stage 02 model output; no OCR geometry is synthesized. Crossed leaders can expose
order inversions for any object. Labels contain neither text nor coordinates.
No inference, segmentation correction, sorting or line rerouting occurs. The
worker performs no completeness, dimension, frame or label validation; drawing
and image-library errors propagate normally.

Use the [orchestrator](../../README.md) with `--from-stage 02.5 --to-stage 02.5`
after Stage 01/02. Restart 02 invalidates this review and downstream stream artifacts.
Restart 02.5 clears this review and every later stage, including 02.9 and 03, even for a review-only interval. Start the next run at 02.9 to retain the review. Stage 03 consumes 02 directly; cleanup follows execution order. Review PNGs are deterministic; user assessment remains separate.

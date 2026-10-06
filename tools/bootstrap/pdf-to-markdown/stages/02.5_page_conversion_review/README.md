# Stage 02.5: Independent Page Conversion Review

Consume validated Stage 02 JSON and the exact Stage 01 PNG. Save `02.5_page_conversion_review/page_NNNN_review.png` at original
page height and twice its width, with a blank equal-width right panel.

Labels show every object's ordinal, type, optional heading level and
`continuation: true` only when the flag is true, in exact JSON
array order. Every object receives a containing 2-pixel frame and straight review
leader, using the local renderer with an identity pixel transform and
continuation labels enabled. This includes prose, captions, table
legends, footnotes, headers and footers. All geometry comes from the validated
Stage 02 model output; no OCR geometry is synthesized. Crossed leaders can expose
order inversions for any object. Labels contain neither text nor coordinates.
No inference, segmentation correction, sorting or line rerouting occurs.

Use the [orchestrator](../../README.md) with `--from-stage 02.5 --to-stage 02.5`
after validated Stage 01/02. Restart 02 invalidates this review and downstream stream artifacts.
Restart 02.5 retains Stage 03 and its dependents; Stage 03 consumes 02 directly. Review PNGs are deterministic; user assessment remains separate.

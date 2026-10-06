# Stage 02m: Independent Page Conversion Review

Consume validated Stage 02d JSON and the exact Stage 01 PNG; 02/02k artifacts
are not inputs. Save `02m_page_conversion_review/page_NNNN_review.png` at original
page height and twice its width, with a blank equal-width right panel.

Labels show every object's ordinal, type, optional heading level and explicit
`continuation: true/false` in exact JSON
array order. Every object receives a containing 2-pixel frame and straight review
leader, using the shared [Stage 02k](../02k_segmentation_review/README.md)
renderer with an identity pixel transform and continuation labels enabled.
Stage 02k keeps its existing labels. This includes prose, captions, table
legends, footnotes, headers and footers. All geometry comes from the validated
Stage 02d model output; no OCR geometry is synthesized. Crossed leaders can expose
order inversions for any object. Labels contain neither text nor coordinates.
No inference, segmentation correction, sorting or line rerouting occurs.

Use the [orchestrator](../../README.md) with `--from-stage 02m --to-stage 02m`
after validated Stage 01/02d. Restart 02d invalidates this review without clearing
the old branch. Review PNGs are deterministic; user assessment remains separate.

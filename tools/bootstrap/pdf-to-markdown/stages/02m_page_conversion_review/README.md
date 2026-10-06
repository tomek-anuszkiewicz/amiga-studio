# Stage 02m: Independent Page Conversion Review

Consume validated Stage 02d JSON and the exact Stage 01 PNG; 02/02k artifacts
are not inputs. Save `02m_page_conversion_review/page_NNNN_review.png` at original
page height and twice its width, with a blank equal-width right panel.

Labels show every object's ordinal, type and optional heading level in exact JSON
array order. Only table/graphic objects receive containing 2-pixel frames and straight
review leaders, using unchanged [Stage 02k](../02k_segmentation_review/README.md)
drawing primitives and edge handling. Text objects have no frames or leaders;
no OCR geometry is synthesized. Labels contain neither text nor coordinates.
No inference, segmentation correction, sorting or line rerouting occurs.

Use the [orchestrator](../../README.md) with `--from-stage 02m --to-stage 02m`
after validated Stage 01/02d. Restart 02d invalidates this review without clearing
the old branch. Review PNGs are deterministic; user assessment remains separate.

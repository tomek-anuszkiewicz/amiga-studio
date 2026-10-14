# Stage 07: Repair Callouts

Read retained Stage 04 pages with full Stage 05 replacements where available.
Stage 06 must have completed before this final preparation stage. No model request
or new role inference is performed.

Within each page, join an immediately adjacent `callout` label and `callout_text`
body into a Markdown callout, for example:

```markdown
> [!NOTE] NOTE
>
> Advisory text.
```

Keep the label's segment ID and original title, including Markdown emphasis.
Preserve body Markdown, blank lines and reading order. Store both original source
boxes in `pdf_regions` as ordered `{page, bbox}` records; the segment's `bbox`
encloses the complete callout. Remove the consumed body segment without renumbering
survivors. Already formatted callouts remain unchanged.

Leave orphan labels/bodies, invalid labels, empty bodies and pairs with different
roles unchanged and report
warnings with segment IDs. Do not guess associations across page boundaries.

Write every selected retained page to `07_repair_callouts/page_*_segments.json`,
including unchanged pages. This directory is the final page-object input for book
import; graphic assets still come from Stage 06. Preserve unrelated fields and
transcribed tables. An empty output is valid when filtering retained no pages.

Run through the [pipeline](../../README.md) with `--from-stage 07 --to-stage 07`
after successful predecessor stages. `--page-ranges` selects pages for this run;
pipeline restart clears only selected outputs when a range is supplied.

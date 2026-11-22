# Stage 07: Repair Callouts

After Stage 06 completes, Stage 07 reads retained pages from the latest
available JSON in Stages 05, 04 and 03, in that order. It omits a page when its
latest file has an empty `segments` list. It makes no model requests and infers
no new roles.

When a `callout` label is immediately adjacent to its `callout_text` body on the
same page, the stage joins the two into a Markdown callout like this one.

```markdown
> [!NOTE] NOTE
>
> Advisory text.
```

The stage keeps the label's segment ID and original title, including Markdown
emphasis. It preserves the body's Markdown and blank lines, and keeps the
reading order. The callout's `pdf_regions` stores both original source boxes
as ordered `{page, bbox}` records. The segment's `bbox` encloses the complete
callout. The stage removes the body segment used in the callout and keeps the
IDs of the remaining segments. Already formatted callouts stay as written.

The stage leaves unpaired labels or bodies, invalid labels, empty bodies and
pairs with different roles unchanged and reports warnings with segment IDs. It
does not guess associations across page boundaries.

The stage writes a full page to `07_repair_callouts/page_*_segments.json` only
when callout repair changes the page. It preserves unrelated fields and
transcribed tables. During proofreading before book import,
[Stage 08](../08_proofreading/README.md) reads these replacements where available
and falls back to Stages 05, 04 and 03 for unchanged pages. Graphic assets
still come from Stage 06. An empty output directory is valid even when pages
remain in earlier stages.

Run through the [pipeline](../../README.md) with `--from-stage 07 --to-stage 07`
after successful predecessor stages. `--page-ranges` selects pages for this run.
When a range is supplied, restarting the pipeline clears only selected outputs.

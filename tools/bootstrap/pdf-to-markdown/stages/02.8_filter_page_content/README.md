# Stage 02.8: Filter Page Content

Read `02_page_conversion/page_*_segments.json` in numeric physical-page order
and preserve each page's segment-array order. Write retained page objects with
the same filenames to `02.8_filter_page_content/`. This deterministic stage
makes no model requests and needs no inference configuration entry.

Apply the user's requested source-fidelity exception:

- Remove all `header` and `footer` objects.
- Remove all objects before the first `toc_heading`, retaining that heading and
  later objects subject to the other rules. Without a heading in the selected
  input, skip this rule so chapter fragments retain their content.
- Omit an entire page if any original object is `list_of_tables_heading`,
  `list_of_tables`, `list_of_figures_heading`, `list_of_figures`, `index_heading`
  or `index`. This also removes unrelated text and crop candidates on that page.

Find the TOC boundary and page-removal triggers in the original objects. A heading
on an excluded page still defines the boundary. Match exact types; do not infer
roles from text, geometry or continuation flags, or inspect unselected PDF pages.
The actual TOC remains, except where a whole-page exclusion applies.

Preserve surviving IDs, fields, text, geometry and ordering without renumbering.
Omit pages with no surviving objects; an empty output is permitted. Print input
and output page/object counts, removed page count and whether a boundary was found.
JSON parsing and filesystem failures stop execution; no runtime schema or artifact
validation is added. Source front matter here means pre-TOC content, not generated
YAML metadata. Stages 02.9 and 12 keep their metadata behavior.

After the existing exclusions, compare only surviving standalone `graphic`
objects with the required [NXP template](resources/nxp.png), resolved relative to
the stage module. The unmodified supplied page-49 logo is 257 x 94 pixels.
Expected relative width/height use a fixed 2550 x 3300 reference page; allow 20%
relative difference. Require the whole box inside `[0, 0, 0.13, 0.055]` in page
fractions. Tables, covers and larger illustrations are retained.

Open each original Stage 01 PNG once only when geometry candidates exist. Crop
exact integer boxes into memory and convert to RGB. Trim near-white outer margins
for both images: any channel below 245 is foreground; keep all components.
Blank candidates remain. Require relative foreground aspect difference at most
0.03, resize the candidate to the trimmed template with Lanczos for comparison,
then require normalized mean absolute RGB difference at most 0.02. Geometry
alone cannot remove content. Retained objects keep their original geometry.
Print candidate/match counts and removed page/segment IDs with comparison scores.
No configuration, temporary crops, output manifest or new dependency is added.
The fixed tolerances derive from the supplied page-49/50 pair; book-wide matching
remains unverified. Merged or nonmatching objects remain.

Run through the [orchestrator](../../README.md):

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.8 --to-stage 02.8
```

Execution requires successful Stage 01/02 statuses. Missing template or candidate
PNG files fail through normal IO without fallback. Restarting 02.8 clears its output
and every later stage, even for this single-stage interval; earlier restarts also
clear it. Stage 01, Stage 02 and Stage 02.5 artifacts remain unchanged. Review
continues to show original objects. Stages 02.9 and 03 consume only filtered JSON
and use original Stage 01 PNGs for retained crops. Existing downstream successes
need explicit regeneration starting at 02.8 to represent this filtering.
Preserve edited Stage 02.9 bundles outside cleanup before restarting; regeneration
rebuilds links/assets and removes stale assets. The bundled resource survives
attempt cleanup.

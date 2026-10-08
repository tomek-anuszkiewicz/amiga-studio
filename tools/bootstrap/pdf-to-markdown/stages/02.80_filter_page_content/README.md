# Stage 02.80: Filter Page Content

Enumerate `02_page_conversion/page_*_segments.json` in numeric physical-page order,
reading Stage 02 directly.

Remove all `header` and `footer` objects by exact type, then apply the fixed NXP
logo comparison below. These are the automatic source-content exclusions at this
stage. Do not infer additional removals from text, geometry or continuation flags.

The [Stage 02 prompt](../02_page_conversion/prompt.md) leaves covers, contents,
indexes and lists of tables or illustrations for manual classification and removal
in the later editor. Preserve that content here. Older saved objects with those
dedicated types are also retained: no TOC boundary or whole-page role triggers
apply. Do not inspect unselected PDF pages.

Preserve surviving IDs, fields, text, geometry and ordering without renumbering.
Omit pages with no surviving objects; an empty output is permitted. Print input
and output page/object counts and the number of pages emptied by filtering.
JSON parsing and filesystem failures stop execution; no runtime schema or artifact
validation is added. Generated YAML metadata is unaffected.

After header/footer removal, compare only surviving standalone `graphic`
objects with the required [NXP template](resources/nxp.png), resolved relative to
the stage module. The unmodified supplied page-49 logo is 257 x 94 pixels.
Expected relative width/height use a fixed 2550 x 3300 portrait reference page,
or 3300 x 2550 for landscape canvases; the logo remains upright. Allow 20%
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
The fixed tolerances derive from the supplied page-49/50 pair; page 63 established
the landscape reference correction. Book-wide matching
remains unverified. Merged or nonmatching objects remain.

Run through the [orchestrator](../../README.md):

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.80 --to-stage 02.80
```

Execution requires successful Stage 01/02 statuses. Missing template or candidate
PNG files fail through normal IO without fallback. Restarting 02.80 clears its output
and every later stage, even for this single-stage interval; earlier restarts also
clear it. Stage 01 and Stage 02 artifacts remain unchanged. Stage 02.81 consumes
only the filtered JSONs; Stage 02.90 exports the completed Stage 02.81 dataset.
Missing filtered pages never fall back to Stage 02.

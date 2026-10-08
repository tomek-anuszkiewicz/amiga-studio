# Stage 02.80: Filter Page Content

Enumerate `02_page_conversion/page_*_segments.json` in numeric physical-page order,
reading Stage 02 directly.

Remove all `header` and `footer` objects by exact type. These are the only
automatic source-content exclusions at this stage. Do not infer additional removals from text, geometry or continuation flags.

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

The Stage 02 prompt classifies recurring margin logos and publisher branding
as header/footer content. This stage does not inspect images or match logo
templates. A logo classified as graphic remains for later manual editing.

Run through the [orchestrator](../../README.md):

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.80 --to-stage 02.80
```

Execution requires successful Stage 01/02 statuses. Restarting 02.80 clears its output
and every later stage, even for this single-stage interval; earlier restarts also
clear it. Stage 01 and Stage 02 artifacts remain unchanged. Stage 02.81 consumes
only the filtered JSONs. The prepared dataset is input for separate book processing.
Missing filtered pages never fall back to Stage 02.

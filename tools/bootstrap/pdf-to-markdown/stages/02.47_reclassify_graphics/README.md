# Stage 02.47: Extract descriptive text from graphics

For every resolved `graphic`, ask whether it contains visibly independent
field explanations or explanatory notes that can be separated from the illustration. Send the complete
unmodified Stage 01 PNG at original detail, frozen page JSON and target bbox.
No request annotation or cropped image is needed. Apply page selection before
requests; pages without graphics make no requests.

The [prompt](prompt.md) retains the source when uncertain. A clear split returns
`graphic` and `prose` children with their own model-provided tight pixel boxes
and continuation flags in source reading order. Prose is transcribed faithfully
into Markdown; graphic content remains deferred. Internal labels, bit numbers,
cell text and text connected by arrows, lines, shared borders or meaningful
geometry remain in the image. Assess the complete figure: retain coordinated
subfigure titles, labels and local descriptions when extracting only some would
leave an inconsistent mixture of external and embedded descriptions. Placement
below a drawing or whitespace alone does not establish independence. A separate
field explanation or note may concern the drawing without being graphically
connected to it. Whole-target table reclassification
is no longer performed.

Shared replacement handling preserves retained objects exactly, gives the first
child the source ID and subsequent children collision-free `_part_N` IDs. Split
children retain other source fields; prose drops graphic-only rotation while
remaining graphics preserve it. Only changed pages receive full-page overrides.

Resolution appends 02.47 after 02.46 before review/filtering and downstream
consumers. The worker reads only predecessors. Absent optional status skips the
layer; failed/running status blocks consumers. Restart clears 02.47 and later
stages, preserving predecessors and cache. Existing attempt model/effort settings
remain; missing settings initialize to gpt-6.1-sol/medium. Prompt/schema changes
change request identity, so former whole-target answers are not reused.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --page-ranges "21,35" --from-stage 02.47 --to-stage 02.50
```

The user selects fragments and assesses quality; execution success does not
certify the split or transcription. The current trial is limited to pages 21 and 35.

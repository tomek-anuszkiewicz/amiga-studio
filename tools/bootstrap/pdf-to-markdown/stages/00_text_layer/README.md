# Stage 00: PDF Text Layer

Stage 00 publishes `00_text_layer/<source stem> - OCR.pdf` and
`text_layer_manifest.json`. Stage 01 consumes this PDF exclusively. The source
PDF stays outside the workspace and is never overwritten.

Existing native text spans are retained, even below 20 characters. Detectable
defects such as replacement characters or invalid text geometry stop preparation;
the worker never overwrites a partial native layer. Native classification is
inferred from existing spans, rather than a visual completeness assessment.
For selected pages without text, the configured Codex client classifies the page
and transcribes visible text, including diagram labels. Text-bearing pages with
empty OCR, invalid boxes, inconsistent classifications or unsupported characters
fail. Blank and graphic-only classifications require empty blocks and retain
their physical page identities.

The worker adds invisible text (`render_mode=3`) using embedded Unicode fonts.
Each block must be supported by one of the bundled Nimbus Sans, Droid Sans
Fallback or Noto Serif fonts. It explicitly fits each OCR line into an equal-height
slice of the supplied box using serialized CID widths and descriptor metrics.
Tabs expand to four spaces; PDF extraction whitespace may differ. Text is never
truncated or replaced with descriptions. Blocks are positioned in displayed-page
coordinates and mapped back through rotation and the PDF y-up frame before
insertion. Reopened extraction must recover the text and intended positions.
Font/matrix serialization uses a fixed 0.02-point geometry tolerance (less than
0.1 pixel at 300 DPI); visual comparison has no pixel tolerance.

If no selected page needs text added, the separate PDF is a byte-for-byte copy.
Otherwise a candidate is saved and reopened before publication. Verification
checks the complete page tree and geometry, retained original content streams
in their original order (allowing new `q/Q` wrappers), unchanged retained native
content and identical renders for every selected page at configured DPI.
The source hash is rechecked. A manifest is published only after these checks;
failure leaves no valid stage completion or consumable manifest.

For a fragment, the full PDF page tree is retained. Only selected physical pages
are prepared and certified; unselected pages remain unchanged and explicitly
outside preparation coverage. Expanding the selection requires restart at 00
with the source PDF or another workspace.

Validated per-page responses live in `recovery/`. Restart preserves these records
but reuse requires identical source/page selection, page/image bytes, geometry,
schema, prompt, preparation procedure, model/effort and runtime identity. Changed
records are rejected. Incompatible intact records are replaced only after a new
validated response. The shared Codex cache remains separately reusable. Recovery
never certifies a partially prepared PDF.

Each read page also writes `page_reads/page_XXXX.json` immediately. Native reads
contain `native_blocks` with extracted text and displayed-page boxes. OCR reads
contain the model's `ocr_response`, including its normalized `box_2d` coordinates,
and a `text_validation` status. Schema-shaped OCR responses rejected for unreadable
text (including U+FFFD), invalid boxes or inconsistent classification remain in
this file with the validation error. Accepted cache/recovery responses produce
the same read record. Transport failures and responses rejected before the OCR
validator runs do not produce an OCR read record.

These partial reads support inspection; a passed text check does not certify PDF
insertion, source preservation or completion of Stage 00. Stage 01 still requires
the fully validated prepared PDF and manifest. Restart at 00 clears `page_reads/`
and rebuilds it for the current run, preserving only compatible recovery records.

Run through the orchestrator:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config tools/bootstrap/pdf-to-markdown/config.yaml --page-ranges "64" --from-stage 00 --to-stage 01
```

`0`, `00` and `00_text_layer` select this stage. `--to-stage 00` prepares the PDF
without preprocessing. Stages 01-14 retain their existing numbers.

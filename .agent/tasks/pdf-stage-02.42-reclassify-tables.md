# PDF-TABLE-2.42: Turn these tables into something else, because they are not tables

Status: planned; collaborative design and implementation remain open.

## Goal and scope

Add Stage `02.42_reclassify_tables` after table splitting at 02.41 and before
review at 02.5. Review objects currently classified as `table` and suggest a
different representation when the visible source is not a genuine table.
Keep genuine tables unchanged. This plan records the requested work; it does
not implement the stage. After implementation, the user requests the selected-page
test and a separate workspace retained for assessment, as specified below.

For each candidate, supply the complete unmodified original Stage 01 page image
at original detail and the target's JSON, including its bounding box. Use
original-image pixel coordinates `[x0, y0, x1, y1]`, top-left origin and exclusive
upper bounds. Include page identity and image dimensions so the JSON region can
be located on the full page. Freeze predecessor objects during the decision pass.
Do not use review images or substitute OCR geometry.

## Collaborative prompt work

The page numbers below are physical PDF pages in the existing test book.
They are user-selected examples for the planned test, not hardcoded rules
or confirmed classification results.

| Transition | Physical pages | Prompt direction | Design status |
| --- | --- | --- | --- |
| `table => image` | 39, 62 | If the selected region is not really a table and cannot faithfully be represented as one, suggest an illustration or image representation. | Map the image outcome to the existing `graphic` object type. |
| `table => compound` | 40 | If the selected region is not really a table but contains several independent elements, split it into those elements. | Determine each child's type, tight bbox and source reading order; decide the response representation together. |
| `table => code_block` | 144 | If the selected region is not a table but a code block set in a monospaced font, change its type to `code_block`. | Prompt supplied; decide when faithful code transcription occurs. |
| `table => prose` | 94 | If the selected region is prose rather than a table, change its type to prose. | Decide when faithful prose transcription occurs. |

When integrating the code-block prompt, consult the existing Stage 02 distinction
between aligned register/parameter descriptions, character-built layouts and
genuine register-map tables. Preserve visible characters, whitespace and
alignment if code transcription is assigned to 02.42.

`compound` describes a decomposition operation here. Whether it needs a persisted
container or only ordered replacement objects remains an explicit design decision.
Independent children may have different existing object types. Do not turn one
compound region into several tables merely because its predecessor was `table`.

## Decisions to settle before implementation

- Define the response contract for retain, reclassify and split suggestions,
  including how a suggestion becomes an applied replacement. Preserve the source
  object when the model cannot establish a different representation.
- Decide whether 02.42 transcribes `prose` and `code_block` or leaves that work to
  a named downstream stage. Existing text consumers need `md_text`; do not emit
  empty textual replacements without an agreed downstream transcription path.
  Retained tables and image/graphic outcomes follow existing deferred transcription.
- Agree how child IDs, source lineage, bbox and reading order are represented,
  reusing existing stage conventions. Preserve independent surrounding objects
  and all source content belonging to the target.
- Decide whether the four directions use one shared decision prompt or separate
  prompts. The test pages must not determine which outcome is allowed.

## Implementation sequence

1. Read the current Stage 02, 02.41 and downstream object contracts. Finalize the
   decisions above with the user and integrate the supplied prompt directions.
2. Implement candidate selection and full-page vision requests through existing
   transport, authentication, cache and attempt-local configuration mechanisms.
   Apply the invocation's `--page-ranges` before context collection or inference.
3. Integrate the stage into ordering, CLI selection, status and restart cleanup.
   Proposed sparse resolution order is `02 -> 02.4 -> 02.41 -> 02.42`; read only
   predecessors and publish complete-page overrides only for changed pages.
   Reuse optional-layer semantics: absent status skips, completed status layers
   overrides, and running/failed status blocks direct consumers.
4. Extend review 02.5 and filtering 02.8 to resolve the new layer. Check that 02.81
   sees only the remaining tables through filtered input and that image, code,
   prose and split children reach later export in the agreed representation.
   Restart at 02.42 clears that stage and all later artifacts while preserving
   predecessors and compatible cache entries.
5. Document the implemented stage, selection and restart contracts in the stage
   README, pipeline README, developer guide and reference conversion contract.
   Preserve existing attempt configuration. Follow the converter testing policy;
   do not add generic local geometry/schema gates or conversion-quality scoring.
6. After implementing 02.42, run the user-requested test on all five physical
   pages `39,40,62,94,144` together in a separate named workspace under the test
   book's `workspace/` directory, proposed name `stage-02.42-reclassification`.
   Prepare or reuse compatible predecessors and run through 02.42 and review
   02.5. Do not overwrite the existing `pages-all` workspace or expand the page
   selection. If the proposed test workspace already exists, inspect its contents
   and configuration before deciding whether it is the same compatible attempt.
7. Leave the separate test workspace intact for the user to assess. Retain source
   page images, predecessor objects, 02.42 outputs, resolved 02.5 review JSON/PNGs,
   attempt configuration and execution status for all five pages. An unchanged
   page may have no sparse 02.42 override; its resolved review must still be
   available. Report the workspace path, review artifact paths and any failed or
   incomplete pages. Do not delete the workspace during task-plan cleanup.

## Verification and closure

Run the required quick pre-flight and architecture suite for each implementation
commit, plus existing technical checks relevant to the concrete change. Do not
launch a full-book pilot or add a speculative test matrix. Technical execution
does not establish that a proposed reclassification is correct; the user evaluates
the selected page results. Delivery requires the selected-page test and the
preserved separate assessment workspace; report failures rather than treating
an unrun or incomplete test as complete.

Record implementation decisions, actual checks and unresolved work under task ID
`PDF-TABLE-2.42` in DIARY.md. Keep this plan active until implementation and the
agreed assessment work are complete or the user closes the task; then follow the
execution-plan lifecycle.

## References

- [Stage 02 object classification](../../tools/bootstrap/pdf-to-markdown/stages/02_page_conversion/README.md)
- [Stage 02.41 table splitting](../../tools/bootstrap/pdf-to-markdown/stages/02.41_split_tables/README.md)
- [Pipeline and physical-page selection](../../tools/bootstrap/pdf-to-markdown/README.md)
- [Developer-led conversion workflow](../../tools/bootstrap/reference-conversion-contract.md#development-workflow)
- [Converter testing scope](../../.agents/rules/unit-testing-policy.md#bootstrap-converter-scope)

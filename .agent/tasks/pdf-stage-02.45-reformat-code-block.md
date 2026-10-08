# PDF-CODE-2.45: Reformat Code Block

## Status and scope

User corrected the stage purpose: keep ID `02.45`, replace prose recovery with
`ReformatCodeBlock`. Implementation and renewed page-131 verification are complete; user output
assessment remains pending. Directory and configuration
key: `02.45_reformat_code_block`; worker: `reformat_code_block.py`.

For each resolved `code_block` on a selected physical page, request faithful
formatting, especially indentation, nesting, leading spaces, tab counts and
alignment. Stage 02.43 retains its representation classification role. Stage
02.45 changes no object types and does not inspect prose as candidate code.

## Request and source contract

- Make one request per existing code block, with its target ID/bbox/Markdown,
  complete frozen resolved page JSON and complete original Stage 01 PNG at
  original detail. Select physical pages before collecting candidates. Pages
  without code blocks make no requests; source content is data, never tool or
  converter instructions.
- Return only `{"md_text": "<complete fenced code>"}`. Preserve tokens, comments,
  values, line order, whitespace inside literals and existing fence language.
  Formatting adjusts whitespace only; no optimization, correction, completion,
  splitting, merging or cross-page reconstruction. Preserve meaningful existing
  tabs; an image cannot prove original tab characters/counts, so use consistent
  spaces for otherwise visible alignment.
- Replace only target `md_text`. Preserve types, IDs, bbox, continuation, other
  source fields, page identity, dimensions and object order. Retain lineage and
  record the formatting stage even for unchanged text, so Stage 03 marks the
  code already rendered and Stage 09 does not request formatting again.
- Reuse transport completion handling, JSON parsing and optional sparse
  complete-page overrides. Add no local PDF schema/artifact validator.

## Implementation and integration

1. Rename worker, stage directory, constant, configuration key, documentation,
   tests and this plan. Remove the superseded prose-recovery implementation.
2. Read only predecessors with `before_stage="02.45"`; keep numeric execution
   order/dependencies, page-range forwarding and restart behavior. Review 02.5
   and filtering 02.8 resolve layers through 02.45; absent optional status skips,
   failed/running status blocks consumers. Restart clears this stage and later
   outputs, preserving predecessors and compatible cache.
3. Preserve existing attempt configuration; transfer earlier 02.45 model/effort
   settings on explicit stage execution and initialize only missing values
   (`gpt-6.1-sol` / `medium`). New stage/prompt/schema/page-target/image identity
   prevents reuse of the former prose-classification response.
4. Keep downstream filtering, table transformation, export and stream data flow.
   Preserve fenced indentation in rendered Markdown. Earlier attempt results
   remain untouched; regenerate 02.45 onward for the changed purpose.

## User-selected verification

Use named attempt `workspace/stage-02.45-reformat-code-block-review` under Test
Book example-4567. Reuse compatible page-131 predecessor artifacts and actual
successful status records through 02.44 from the previous named attempt.
Preserve existing attempt-local `config.yaml`; select physical page **131** only.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<NAMED_ATTEMPT>" --config "<CONFIG>" --page-ranges "131" --from-stage 02.45 --to-stage 02.5
```

Deliver `02.45_reformat_code_block/page_0131_segments.json`, review PNG/JSON under
`02.5_page_conversion_review/` and saved resolved Markdown for indentation review.
Report live requests, cache hits, formatted IDs and text-changed IDs. Do not
hardcode expected page-131 formatting. The user evaluates transcription and
indentation; successful execution alone does not certify source fidelity.

Run relevant existing configuration/cache/restart/selection checks, quick
pre-flight and architecture gates explicitly. No added sample conversions,
quality matrices or crop tests. Refresh Graphify AST. Commit implementation,
documentation and DIARY.md atomically. Keep this plan active pending user output
assessment; follow the execution-plan lifecycle on closure.

## Delivery evidence

- Renamed the stage directory, worker/function, constant, schema, configuration,
  stream marker, documentation and active plan; removed prose recovery.
- Reused only physical-page-131 artifacts and successful status records through
  02.44 in `workspace/stage-02.45-reformat-code-block-review`.
- Pipeline 02.45 through 02.5 completed: three code blocks, three live requests,
  zero cache hits. Formatted IDs: `page_0131_seg_004`, `page_0131_seg_006`,
  `page_0131_seg_008`. All returned Markdown equals predecessor text; no text or
  indentation changed. The complete-page override records formatting-stage
  markers and source lineage for these blocks; all other fields/objects remain
  unchanged. Saved review PNG/JSON and attempt-root `page_0131.md` for inspection
  (resolved saved text only; deferred table/graphic transcription is excluded).
- Verified exact saved model/effort transfer and source-field preservation.
  Existing suites passed: 30 PDF configuration/restart/stream, 6 selection and
  21 shared configuration/transport/cache checks. Quick pre-flight and all 19
  architecture tests passed. Graphify AST update completed.
- No additional pages, prompt tuning, full-book conversion or milestone checks
  ran. Transcription and indentation quality remain for user assessment.

# PDF-CODE-2.45: Reformat Code Block

## Status and scope

User corrected the stage purpose: keep ID `02.45`, replace prose recovery with
`ReformatCodeBlock`. The user requested JSON-only formatting with normalized structural indentation.
This revision and renewed page-131 verification are complete; user output
assessment remains pending. Directory and configuration
key: `02.45_reformat_code_block`; worker: `reformat_code_block.py`.

For each resolved `code_block` on a selected physical page, request faithful
formatting, especially indentation, nesting, leading spaces, tab counts and
alignment. Stage 02.43 retains its representation classification role. Stage
02.45 changes no object types and does not inspect prose as candidate code.

## Request and source contract

- Make one request per existing code block, with its target ID/bbox/Markdown,
  complete frozen resolved page JSON only, without an image attachment. Select physical pages before collecting candidates. Pages
  without code blocks make no requests; source content is data, never tool or
  converter instructions.
- Return only `{"md_text": "<complete fenced code>"}`. Preserve tokens, comments,
  values, line order, whitespace inside literals and existing fence language.
  Formatting adjusts whitespace only; no optimization, correction, completion,
  splitting, merging or cross-page reconstruction. Normalize indentation from code structure with four spaces per nesting level,
  consistent brace alignment and indentation tabs converted to spaces. Existing
  indentation is input to improve, not a formatting reference.
- Replace only target `md_text`. Preserve types, IDs, bbox, continuation, other
  source fields, page identity, dimensions and object order. Retain lineage and
  record the formatting stage even for unchanged text, so Stage 03 marks the
  code already rendered and Stage 09 does not request formatting again.
- Reuse transport completion handling, JSON parsing and optional sparse
  complete-page overrides. Add no local PDF schema/artifact validator.

## Implementation and integration

1. Remove Stage 02.45 image attachment and vision selection. Update prompt,
   stage/pipeline documentation and this plan for JSON-only structural formatting.
2. Read only predecessors with `before_stage="02.45"`; keep numeric execution
   order/dependencies, page-range forwarding and restart behavior. Review 02.5
   and filtering 02.8 resolve layers through 02.45; absent optional status skips,
   failed/running status blocks consumers. Restart clears this stage and later
   outputs, preserving predecessors and compatible cache.
3. Preserve existing attempt configuration; transfer earlier 02.45 model/effort
   settings on explicit stage execution and initialize only missing values
   (`gpt-6.1-sol` / `medium`). Changed prompt and JSON-only request identity
   prevents reuse of the former prose-classification response.
4. Keep downstream filtering, table transformation, export and stream data flow.
   Preserve fenced indentation in rendered Markdown. Earlier attempt results
   remain untouched; regenerate 02.45 onward for the changed purpose.

## User-selected verification

Use named attempt `workspace/stage-02.45-json-code-format-review` under Test
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

- Removed image attachment and vision selection from 02.45. Requests contain
  frozen page/target JSON only. Prompt normalizes four-space structural nesting,
  brace alignment and indentation tabs rather than reproducing source layout.
- Named attempt `workspace/stage-02.45-json-code-format-review` reuses only page
  131 and actual successful predecessor records through 02.44.
- Pipeline 02.45-02.5 completed with three live requests and zero cache hits.
  Formatted IDs: `page_0131_seg_004`, `page_0131_seg_006`, `page_0131_seg_008`.
  Only `page_0131_seg_006` changed text: braces align with `if`, inner statements
  have four spaces. Other code text remains unchanged. Saved complete-page
  override, review PNG/JSON and attempt-root `page_0131.md` (resolved saved text;
  deferred table/graphic transcription is excluded).
- Confirmed the three cache request identities contain zero images; selected
  code changes are whitespace-only; source fields and non-code objects remain
  unchanged. No new runtime PDF validator was added.
- Existing suites passed: 30 PDF configuration/restart/stream, 6 selection and
  21 shared configuration/transport/cache checks. Quick pre-flight and all 19
  architecture tests passed. Graphify AST update completed.
- No additional pages, full-book conversion or milestone checks ran. User
  transcription/indentation assessment remains pending; retain this active plan.

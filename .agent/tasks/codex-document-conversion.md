# Roadmap 1.1: Codex Conversion and Stage Configuration

Status: active. The shared Codex client, HTML migration and PDF caller/configuration migration are implemented. One physical PDF page passed all 14 stages; remaining acceptance concerns workflow and input/output contracts. Conversion-quality evaluation is outside this task. Roadmap 1.1 is not complete.

Restart follow-up: whole-workspace downstream cleanup now includes manual tasks and preserves upstream assets. Missing working manifests restore from retained snapshots; resume selects the earliest missing stage artifact. Default output belongs to the selected workspace. Explicit stage intervals retain only matching source/page predecessors; corruption repair is outside scope. Focused offline regressions cover stages 5–10 page selection, stage 05 cleanup, missing outputs/manifests, manual reset/apply and independent workspaces. Conversion-quality changes remain deferred.

Restart validation: 10 additional PDF regressions passed (17 PDF and 15 shared/HTML tests total), plus 4 bootstrap tests. A separate page-19 workspace completed all stages from cached responses, then passed stage-05 reset with missing page/chapter manifests and chapter output, followed by resume through stage 14. Earlier artifacts and the original workspace remained unchanged; no live inference ran. Evidence is under `.agent/tmp/pdf-restart-pilot/verification.json`. Quick pre-flight passed; architecture remains 18/19 because 62 Design links target missing Reference materials.

## Current execution evidence

- Installed and pinned Python SDK and bundled runtime 0.160.1 in a disposable environment. Verified ChatGPT authentication, runtime model catalog, explicit `gpt-6.1-sol` / `medium`, fresh ephemeral read-only threads, no repository instruction sources, and disabled inherited tools. An empty MCP override merges with user configuration, so the client explicitly disables inherited server names and verifies the effective configuration.
- HTML now uses the shared stage-bound client, strict YAML parser and isolated Codex cache. Removed duplicated Gemini files and silent configuration/DOM substitutions. A single small HTML page preserved prose, code and a table; unchanged replay hit the cache. Offline regressions cover selection, error propagation, JSON/cache validation, image detail, fresh threads and timeout closure.
- PDF pilot used only physical page 16 from the supplied 160-page book, rendered at 300 DPI and visually inspected. One image-plus-schema request with `detail: original` produced a Markdown transcription preserving all three sections, signal overbars, footer and the incomplete final sentence. It took 14.63 seconds; reported usage was 17,197 input tokens, 737 output tokens (61 reasoning tokens). This is transport/source-fidelity evidence for one prose page, not a completed PDF stage or full-book validation.
- Inputs, source hashes, requests, responses, metrics and page image are under `.agent/tmp/codex-conversion-pilot/`. The source PDF and tracker were not changed. Initial transport pilots included one text request and one schema request. HTML integration used one aborted request, two completed requests after transport corrections and one cache replay; PDF used one completed image/schema request. No bulk conversion or indexing ran.
- PDF integration now uses the shared stage-bound client and schemas, strict YAML, predecessor completion fingerprints, manifest snapshots, manual identity markers and persistent worker metrics. Physical page 19 passed all 14 stages with 8 live requests in total (segmentation 1, table 1, prose 3, title 1, properties 1, opening title 1). Completed-run resume validated all 14 records without inference. Artifacts are under `.agent/tmp/pdf-one-page-19/`. Quality repairs were explicitly deferred by the user; the temporary prose-image repair and title-call optimization were reverted.
- The latest recorded offline validation includes 36 conversion tests: 15 shared/HTML and 21 PDF tests covering configuration, lineage, restart/manual recovery, native/scanned raw streams and PNG crops. Pending: identify any uncovered flow or input/output branches and add only the minimal fixtures needed to verify them. Milestone review, checkpoint synchronization, diary compaction and roadmap pruning remain pending.


Replace Gemini inference in both bootstrap converters with Codex, using the user's ChatGPT sign-in. Give every inference stage one explicit model and reasoning-effort entry. Keep the existing conversion pipeline usable until its clients, configuration readers, and callers migrate together.

## Scope and Decisions

- Own [roadmap item 1.1](../../ROADMAP.md), the [PDF converter](../../tools/bootstrap/pdf-to-markdown/README.md), and the [HTML converter](../../tools/bootstrap/html-to-markdown/README.md).
- Validate pipeline flow and input/output contracts: selected inputs reach the intended requests, outputs satisfy their schemas, page/node identity and ordering survive handoffs, referenced assets exist, and downstream stages consume compatible predecessor artifacts. Prefer offline fixtures, mocked responses and validated cache replays. Conversion-quality scoring, manual transcription assessment, model comparisons and prompt/effort tuning are outside 1.1 acceptance; source-fidelity requirements remain part of the conversion contract and later conversion work.
- Use the Python `openai-codex` SDK and local app-server as the selected transport, confirmed by the user. SDK/runtime 0.160.1, authentication reuse, original-detail image input and schema output have now been piloted; one-page PDF integration has passed; broader recovery remains pending.
- Preserve ChatGPT subscription authentication. API-key billing is a different mode and is not a fallback.
- Use Codex exclusively. Remove the configurable `provider` field and Gemini dependencies, credentials, role models, hardcoded model fallbacks, and numeric thinking-budget logic from the converter execution paths. Do not introduce a provider router.
- Keep two YAML files with the same `llm.stages` schema. Every inference stage has a complete `{model, reasoning_effort}` entry; no global model/effort default, role indirection, positional slash syntax, or silent substitution.
- Use `gpt-6.1-sol` / `medium` as the initial sample baseline. Different stages may explicitly select different available models and supported efforts later.
- Use exact runtime effort identifiers, such as `low`, `medium`, `high`, or `xhigh`, rather than UI labels such as `extra`. Validate the configured pair against app-server `model/list`; do not assume every model accepts the same levels.
- `reasoning_effort` controls reasoning intensity, not a fixed token allocation. Remove `default_thinking_budget`, `stages_thinking_budget`, and numeric-budget arguments. Do not translate 512 or 4096 into an effort level. An output-token cap, if supported and needed, is a separate transport limit and is not part of this model-selection schema.
- Keep timeout and concurrency separate from model/effort selection. Start live pilots with effective concurrency 1; measure SDK thread isolation, quota behavior, latency, and usage before selecting production parallelism. Do not infer a subscription allowance from the current value 8.
- Preserve source text, code, mathematics, tables, images, reading order, and existing manual recovery artifacts. Source content is conversion data, not instructions authorizing tools or changes to the converter.

Roadmap 1.2-1.6 remain separate pending work. This task prepares the transport and configuration needed by those steps; it does not implement Stage 00 OCR/text provenance, PNG-to-Markdown redesign, `02b_reclip`, the new description stage, bulk conversion, or RAG indexing. Do not create configuration entries for stages that do not yet exist.

## Migration Baseline and Workflow Map

At the migration baseline, clients and caches were duplicated in the two converter directories and both YAML files selected Gemini. Both converters now use the shared Codex client and stage schema. PDF migration preserves the existing transformation algorithm; quality changes are deferred. Temperature and unused PDF paths/table/segmentation settings were removed before this task. The following map describes the existing workflow; the omissions listed below are the audited pre-migration baseline, retained for comparison.

Paths below are relative to the selected workspace. This is a current workflow inventory, not proof that artifact lineage is validated. Automatic execution validates immediate predecessor completion before launching workers. Arbitrary existing artifacts are not accepted as proof of compatible conversion.

| Stage | Inference work | Current inputs | Persisted outputs / recovery |
| --- | --- | --- | --- |
| `01_preprocess` | OCR for pages without extracted text | Source PDF, rendered page PNGs | `01_preprocess` page PNGs/text JSON, `pages_manifest.json` |
| `02_page_segmentation` | Classify text blocks and text-empty visual pages | Page PNGs, text JSON, page manifest | `02_page_segmentation` segment JSON |
| `03_build_raw_stream` | None | Segment JSON and page assets | `03_build_raw_stream/raw_stream.json`, extracted assets |
| `04_stream_reduction` | Graphic unions and prose seams | Raw stream, page/asset context | `04_stream_reduction/reduced_stream.json`, assets |
| `05_chapter_partition` | None | Reduced stream | `05_chapter_partition` chapter JSON, `chapters_manifest.json` |
| `06_detect_continuations` | Table/graphic continuation decisions | Partitioned chapters and neighboring block context | `06_detect_continuations` chapter JSON; `tasks/continuations` prepare/apply artifacts |
| `07_transform_tables` | Text/vision table rendering | Chapter nodes, raw text, ordered table crops | `07_transform_tables` chapter JSON/assets; `tasks/tables` prepare/apply artifacts |
| `08_transform_graphics` | Triage, Mermaid, ASCII, image descriptions | Chapter nodes, raw labels, graphic crops | `08_transform_graphics` chapter JSON/assets/sidecars; `tasks/graphics` prepare/apply artifacts |
| `09_transform_prose` | Prose/code/TOC formatting, optionally vision | Chapter nodes and optional crops | `09_transform_prose` chapter JSON; `tasks/prose` prepare/apply artifacts |
| `10_proofread_stream` | Title/slug/text proofreading | Chapter JSON and chapter manifest | `10_proofread_stream` normalized JSON/assets, updated manifest |
| `11_emit_markdown` | None | Proofread/formatted chapter JSON | `11_emit_markdown` Markdown/assets |
| `12_generate_properties` | Extract publication metadata | Markdown and document/chapter context | `12_generate_properties` Markdown/assets |
| `13_refine_first_chapter_name` | Extract canonical first-chapter title | Generated Markdown and first-chapter context | `13_refine_first_chapter_name` renamed Markdown/assets |
| `14_link_toc` | None | Markdown and header catalog | `14_link_toc` linked Markdown/assets |
| HTML `html_to_markdown` | Transcribe a single HTML document or aggregated crawl | HTML, document title, conversion instructions | Markdown/assets in the output directory |

All branches within an inference stage use its configured pair: text, vision, JSON, triage, OCR, empty-page handling, and repair requests. Audited baseline omissions addressed by this migration:

- `02_page_segmentation/segment_page.py`: the text-empty-page vision call has no stage argument.
- `04_stream_reduction/reduce_stream.py`: prose-seam inference has no stage argument; graphic-union inference already names stage 04.
- `06_detect_continuations/detect_continuations.py`: continuation inference has no stage argument. Both stages 04 and 06 are absent from the current thinking-budget map.
- `13_refine_first_chapter_name/refine_name.py`: replace the alias `13_refine_chapter` with the canonical directory/stage identifier.
- HTML `pipeline.py`: one inference stage, `html_to_markdown`. Rendering, downloading, placeholder replacement, and link audits currently have no model calls and receive no model entries.
- HTML catches inference failures and silently falls back to DOM extraction; it also substitutes the PDF configuration or embedded defaults. Remove these substitutions from the mandatory Codex path. Preserve useful deterministic helpers as explicit preprocessing/diagnostics, not successful replacements for failed inference.
- PDF `--run-deterministic` currently includes stages 04 and 10 despite their inference calls. Reconcile the mode and CLI documentation with actual capabilities.

## Target Configuration

Keep rendering, markers, and other consumed non-model settings in their existing sections. The examples below show model selection only. Both converters support this schema.

PDF:

```yaml
llm:
  stages:
    01_preprocess: {model: "gpt-6.1-sol", reasoning_effort: "medium"}
    02_page_segmentation: {model: "gpt-6.1-sol", reasoning_effort: "medium"}
    04_stream_reduction: {model: "gpt-6.1-sol", reasoning_effort: "medium"}
    06_detect_continuations: {model: "gpt-6.1-sol", reasoning_effort: "medium"}
    07_transform_tables: {model: "gpt-6.1-sol", reasoning_effort: "medium"}
    08_transform_graphics: {model: "gpt-6.1-sol", reasoning_effort: "medium"}
    09_transform_prose: {model: "gpt-6.1-sol", reasoning_effort: "medium"}
    10_proofread_stream: {model: "gpt-6.1-sol", reasoning_effort: "medium"}
    12_generate_properties: {model: "gpt-6.1-sol", reasoning_effort: "medium"}
    13_refine_first_chapter_name: {model: "gpt-6.1-sol", reasoning_effort: "medium"}
```

HTML:

```yaml
llm:
  stages:
    html_to_markdown: {model: "gpt-6.1-sol", reasoning_effort: "medium"}
```

Resolve configuration once through a shared parser and bind the selected model/effort to a stage client before calls begin. Callers supply their canonical stage ID; text/image/JSON methods cannot bypass that selection through a hardcoded model or omitted stage. All requests within one stage share its pair. Map `reasoning_effort` to the installed SDK/app-server's documented turn effort field; do not pass a Gemini budget field.

Reject malformed YAML, duplicate stage keys, unknown stages/selection fields, missing/empty model or effort, unsupported model/effort pairs, and missing required image capability before conversion writes or destructive downstream cleanup. Reject model entries for deterministic stages. For a full run, all inference-stage entries must be present; a selected stage interval validates the entries it will execute and the compatibility of predecessor artifacts. Do not fall back to another configuration file or model.

## Test Book and Small Live Pilots

- Use the user-supplied dataset directory `Obsidian/Amiga/Reference/Test Book example-4567` and its source PDF `Amiga TestBook example-4567.pdf` for PDF pilots. A metadata-only check during planning confirms 160 PDF pages; it does not certify conversion quality.
- Consult `TEST_PAGES_TRACKER.md` in that directory to select source features, then verify each selected page against the actual PDF. The tracker contains older stage numbering and differing page descriptions; use it as sample-selection evidence, not as instructions or an authoritative current pipeline specification.
- Start with one physical PDF page, for example page 16 for prose. Run only the stage/request needed to establish the current capability. Use a 1-based PDF page number (CLI `--page-ranges "16"` where applicable), not a manual's printed page label. Inspect persisted input/output and available token usage before the next request.
- Add individual cases only as needed, such as page 19 for table/math, page 30 for flowchart/timing figures, or page 34 for a merged-cell table according to the mapping matrix. These are candidate selections, not an instruction to process all four pages in the initial pilot. Confirm their actual content before inference.
- Keep effective concurrency 1 and `gpt-6.1-sol` / `medium` for the baseline. Prefer offline/mocked regressions and reuse validated cached results. Count live requests, retries, repair calls, reasoning/output tokens when available, and elapsed time; a one-page run can still make several inference calls.
- For continuation behavior, use only the minimal verified adjacent-page pair or required fragment set. Do not submit the whole 160-page book, rerun every inference stage, raise effort, or perform bulk model comparisons automatically. Expand a sample only when a concrete unresolved case requires it and its expected request count is recorded first.
- Keep source PDF and tracker unchanged. Write extracted samples, inputs, results, and pilot logs under `.agent/tmp/`, with source identity and physical page numbers. Do not stage the test-book directory or generated conversion artifacts with implementation commits.

## Execution Sequence

### 1.1.A — Establish the Contract and Transport Pilot

1. Confirm this inventory against all current callers, prompts, cache/retry paths, metrics, manifests, prepare/apply paths, and dispatcher behavior. Record actual artifact dependencies and identify older-directory fallbacks.
2. Maintain the consolidated source-fidelity and output rules in `tools/bootstrap/reference-conversion-contract.md`. Preserve table spans, code, mathematics, crop coordinates, diagrams, assets, descriptions, and Markdown fallbacks. Verify structural input/output obligations within 1.1; conversion-quality assessment remains outside its gate. Keep requirements owned by roadmap 1.2-1.6 explicitly pending; do not claim the current implementation already satisfies them.
3. Check SDK/runtime versions and ChatGPT authentication without printing credentials. Query available models, efforts, and input modalities through `model/list`; API catalog values alone do not prove availability on the signed-in account.
4. Install/pin the chosen SDK in the converter environment during implementation. Prove subscription authentication, text, image, structured JSON, effective effort, completed-result extraction, timeouts, and usage reporting through the one-page pilot policy above before replacing workers. Inspect each result before issuing the next live request.
5. Use a fresh independent thread per conversion request. Supply explicit conversion instructions and inputs, with read-only execution boundaries and no inherited unrelated repo instructions/tools. Verify these boundaries in the pilot rather than assuming SDK defaults. Test requested original-image fidelity where the transport supports it; an unsupported required image mode is an explicit compatibility blocker.

Gate: a reproducible pilot proves the selected Codex transport can perform all three request forms with ChatGPT authentication and exposes the actual configured model/effort. If image detail or schema semantics differ, resolve the contract before production replacement. Do not switch to a billed API or Gemini to hide the gap.

### 1.1.B — Add Shared Configuration, Client, and Cache

1. Introduce a cohesive shared module under `tools/bootstrap/conversion/` for configuration validation, a stage-bound Codex client, request completion/error handling, and cache operations. Keep it transport-specific; no provider registry or speculative router.
2. Preserve text, vision, JSON, call counts, cache-hit counts, duration, and usage interfaces needed by existing workers. Normalize SDK completion and error states at this boundary, with JSON/schema validation before caching.
3. Require explicit model/effort selection. Do not retry by silently changing either. Stop authentication/quota failures clearly and preserve resumable diagnostic state; allow bounded backoff only for genuinely transient failures.
4. Use a Codex cache namespace separate from untouched Gemini cache data. Include fixed engine identity, runtime/contract version, canonical stage, model, effort, effective instructions/prompt, ordered image contents and relevant image settings, and output schema in request identity.
5. Cache only complete, validated responses. Preserve model/effort metadata in entries and metrics; malformed, partial, interrupted, or truncated output cannot be published as successful conversion.

Gate: offline contract tests prove stage selection reaches text/image/JSON requests and covers error states and cache separation. The shared implementation works from repo-root CLI and worker subprocesses without private paths.

### 1.1.C — Migrate Both Configurations and Every Caller Together

1. Rewrite both YAML files to the target stage schema; remove `provider`, `model_vision`, `model_prose`, `model_table`, `model_fast`, and all thinking-budget keys/arguments.
2. Bind every current inference caller to its canonical stage ID, including the missing stage branches identified above. Maintain one stage pair even where graphics uses multiple prompts or tables uses multiple images.
3. Replace both duplicated Gemini clients/caches with shared Codex integration. Remove Gemini imports, key loading, hardcoded model fallback loops, and obsolete retry/metric terminology within the converter scope.
4. Remove silent HTML configuration and DOM fallbacks. Update deterministic-mode selection from the audited stage capabilities. Preserve deterministic extraction, image processing, prepare/apply tools, and source/output ordering where they serve the documented workflow.
5. Update both converter READMEs, affected stage documentation, dispatcher dependency instructions, and [developer conversion guidance](../../docs/developers.md#processing-raw-documents-into-markdown) to the implemented behavior. Remove stale claims about unused paths/heuristics while updating configuration documentation. Historical diary entries and already converted reference documents remain history.

Gate: no mandatory conversion path calls Gemini, bypasses stage selection, or silently substitutes a model, effort, configuration, or deterministic result. Both configs load through the same schema; every active inference stage has exactly one effective entry.

### 1.1.D — Make Resume and Manual Handoffs Configuration-Aware

1. Include effective model/effort, instructions, source identity, output schema, and relevant contract/runtime versions in conversion fingerprints alongside existing artifact identity. Store each stage's request/config identity with its completion record and predecessor lineage.
2. Reject incompatible `--resume`, `--from-stage`, and prepared/manual results before cleanup or processing. Report the earliest affected stage; unchanged predecessors remain reusable, but affected output and its dependents require explicit regeneration.
3. Validate the immediate intended predecessor rather than accepting arbitrary earlier chapter/Markdown directories. Planned new Stage 00 and `02b_reclip` handoffs remain owned by 1.2 and 1.4.
4. Preserve stable page/node IDs, source ordering, workspace snapshots, stage status, task manifests, accepted assets, and manual recovery data. Check metadata for prepared edits as well as automatic responses.

Gate: unchanged runs reuse valid artifacts; changing one stage's model/effort invalidates that stage and dependent results without falsely completing conversion or reusing stale manual output.

### 1.1.E — Validate Flow and Input/Output Contracts and Close the Roadmap Item

1. Add focused offline regressions for configuration parsing, duplicate/missing/unknown stage entries, unsupported model/effort or image capability, all inference call branches, complete-result/schema enforcement, cache separation, auth/quota/timeout errors, and fingerprint/resume/manual-handoff behavior.
2. Verify both CLIs and the dispatcher. Use disposable diagnostics under `.agent/tmp/` and avoid running conversion in user source directories during tests.
3. Build a coverage matrix for implemented PDF/HTML flow and input/output branches using existing tests and pilot evidence. Use minimal offline fixtures, mocked responses or compatible cache replays for native/scanned/mixed and blank-page paths, text/table/graphic branches, continuation handoffs and manual recovery only where a concrete contract remains uncovered. Verify that selected text/images/schema reach the request, persisted outputs are structurally valid and the next stage consumes the intended artifacts. Add a live request only when an unresolved transport behavior cannot be verified offline; retain `gpt-6.1-sol` / `medium`, concurrency 1 and record the expected request count first. Treat text-layer/reclip behaviors not yet implemented under later roadmap items as pending; full-book/full-crawl processing belongs to the later bulk-conversion gate.
4. Persist fixture/source identity, assembled requests, outputs, schema checks, artifact/asset references, page/node ordering, predecessor fingerprints and recovery results. Record call/cache counts, latency and available usage when requests run. Do not add visual-quality reviews, transcription scoring, model comparisons or prompt/effort tuning to this gate. Bound claims to tested flow and input/output behavior.
5. Per implementation commit, run targeted conversion regressions, `python tools/harness/pre_flight.py --quick`, and `cargo test -p test_runner --test test_architecture_rules -- --quiet`; include a Section 10 diary entry and the stable task ID in the commit rationale.
6. Before declaring 1.1 complete, follow [milestone policy](../../.agents/rules/roadmap-maintenance.md) and the [roadmap completion procedure](../../.agents/skills/roadmap-maintenance/SKILL.md): milestone gate, required unit/integration checks, semantic parity review of modified conversion behavior, affected documentation/checkpoints, diary compaction, and roadmap pruning. Planning alone does not satisfy this gate.

Acceptance: both converters use Codex with ChatGPT authentication and validated per-stage model/effort settings; implemented stage flow, request inputs, output schemas, artifact/asset handoffs and cache/resume/manual recovery satisfy the structural conversion contracts. This gate does not certify transcription or conversion quality. Remaining 1.2-1.6 work stays visible in the roadmap. Prune completed execution detail only after its implementation evidence exists.

## Official References

- [Codex Python SDK](https://learn.chatgpt.com/docs/codex-sdk#python-library): Python integration with a local app-server and pinned runtime dependency.
- [Codex app-server](https://learn.chatgpt.com/docs/app-server): `model/list` reports model capabilities and supported efforts; turn configuration accepts model, effort, and output schema.
- [Codex authentication](https://learn.chatgpt.com/docs/auth): ChatGPT sign-in uses subscription access; API-key use is separately billed.
- [GPT-6.1 Sol](https://developers.openai.com/api/docs/models/gpt-6.1-sol): model specifications and Responses API efforts. Validate the installed Codex runtime/account separately.

## Planning Validation

The original planning-only validation scope was: validate local links, YAML examples and audited inference-stage coverage, English/portable paths, and preservation of pending roadmap scope. Run the quick pre-flight and separate architecture suite, record observed results in the diary, and commit the plan and roadmap together. Do not install an SDK, run inference, migrate active YAML/code, convert manuals, or index documents as part of preparing the plan.

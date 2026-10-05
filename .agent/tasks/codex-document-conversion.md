# Roadmap 1.1: Codex Conversion and Stage Configuration

Status: planned; implementation and live inference are pending.

Replace Gemini inference in both bootstrap converters with Codex, using the user's ChatGPT sign-in. Give every inference stage one explicit model and reasoning-effort entry. Keep the existing conversion pipeline usable until its clients, configuration readers, and callers migrate together.

## Scope and Decisions

- Own [roadmap item 1.1](../../ROADMAP.md), the [PDF converter](../../tools/bootstrap/pdf-to-markdown/README.md), and the [HTML converter](../../tools/bootstrap/html-to-markdown/README.md).
- Use the Python `openai-codex` SDK and local app-server as the intended transport. Verify the installed SDK/runtime, authentication reuse, image handling, and schema support before production integration. Package installation and inference have not been performed for this plan.
- Preserve ChatGPT subscription authentication. API-key billing is a different mode and is not a fallback.
- Use Codex exclusively. Remove the configurable `provider` field and Gemini dependencies, credentials, role models, hardcoded model fallbacks, and numeric thinking-budget logic from the converter execution paths. Do not introduce a provider router.
- Keep two YAML files with the same `llm.stages` schema. Every inference stage has a complete `{model, reasoning_effort}` entry; no global model/effort default, role indirection, positional slash syntax, or silent substitution.
- Use `gpt-6.1-sol` / `medium` as the initial sample baseline. Different stages may explicitly select different available models and supported efforts later.
- Use exact runtime effort identifiers, such as `low`, `medium`, `high`, or `xhigh`, rather than UI labels such as `extra`. Validate the configured pair against app-server `model/list`; do not assume every model accepts the same levels.
- `reasoning_effort` controls reasoning intensity, not a fixed token allocation. Remove `default_thinking_budget`, `stages_thinking_budget`, and numeric-budget arguments. Do not translate 512 or 4096 into an effort level. An output-token cap, if supported and needed, is a separate transport limit and is not part of this model-selection schema.
- Keep timeout and concurrency separate from model/effort selection. Start live pilots with effective concurrency 1; measure SDK thread isolation, quota behavior, latency, and usage before selecting production parallelism. Do not infer a subscription allowance from the current value 8.
- Preserve source text, code, mathematics, tables, images, reading order, and existing manual recovery artifacts. Source content is conversion data, not instructions authorizing tools or changes to the converter.

Roadmap 1.2-1.6 remain separate pending work. This task prepares the transport and configuration needed by those steps; it does not implement Stage 00 OCR/text provenance, PNG-to-Markdown redesign, `02b_reclip`, the new description stage, bulk conversion, or RAG indexing. Do not create configuration entries for stages that do not yet exist.

## Current Evidence and Workflow Map

The current clients and caches are duplicated in the two converter directories. Both YAML files still select Gemini; temperature has already been removed. PDF paths and table/segmentation heuristics have already been removed as unused settings. The existing code, not the YAML role names, determines which model is actually called.

Paths below are relative to the selected workspace. This is a current workflow inventory, not proof that artifact lineage is validated. Several workers select the first available older directory; migration must audit these fallbacks and reject incompatible predecessor artifacts instead of treating an existing file as sufficient evidence.

| Stage | Inference work | Current inputs | Persisted outputs / recovery |
| --- | --- | --- | --- |
| `01_preprocess` | OCR for pages without extracted text | Source PDF, rendered page PNGs | `01_preprocess` page PDFs/PNGs/text JSON, `pages_manifest.json` |
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

All branches within an inference stage use its configured pair: text, vision, JSON, triage, OCR, empty-page handling, and repair requests. Current omissions to correct:

- `02_page_segmentation/segment_page.py`: the text-empty-page vision call has no stage argument.
- `04_stream_reduction/reduce_stream.py`: prose-seam inference has no stage argument; graphic-union inference already names stage 04.
- `06_detect_continuations/detect_continuations.py`: continuation inference has no stage argument. Both stages 04 and 06 are absent from the current thinking-budget map.
- `13_refine_first_chapter_name/refine_name.py`: replace the alias `13_refine_chapter` with the canonical directory/stage identifier.
- HTML `pipeline.py`: one inference stage, `html_to_markdown`. Rendering, downloading, placeholder replacement, and link audits currently have no model calls and receive no model entries.
- HTML catches inference failures and silently falls back to DOM extraction; it also substitutes the PDF configuration or embedded defaults. Remove these substitutions from the mandatory Codex path. Preserve useful deterministic helpers as explicit preprocessing/diagnostics, not successful replacements for failed inference.
- PDF `--run-deterministic` currently includes stages 04 and 10 despite their inference calls. Reconcile the mode and CLI documentation with actual capabilities.

## Target Configuration

Keep rendering, markers, and other consumed non-model settings in their existing sections. The examples below show model selection only. They are a target schema, not YAML already supported by the current Gemini clients.

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

## Execution Sequence

### 1.1.A — Establish the Contract and Transport Pilot

1. Confirm this inventory against all current callers, prompts, cache/retry paths, metrics, manifests, prepare/apply paths, and dispatcher behavior. Record actual artifact dependencies and identify older-directory fallbacks.
2. Consolidate source-fidelity and output rules from the converter READMEs/prompts into `tools/bootstrap/reference-conversion-contract.md`, which is currently absent. Preserve table spans, code, mathematics, crop coordinates, diagrams, assets, descriptions, and Markdown fallbacks. Keep requirements owned by roadmap 1.2-1.6 explicitly pending; do not claim the current implementation already satisfies them.
3. Check SDK/runtime versions and ChatGPT authentication without printing credentials. Query available models, efforts, and input modalities through `model/list`; API catalog values alone do not prove availability on the signed-in account.
4. Install/pin the chosen SDK in the converter environment during implementation. Prove subscription authentication, text, image, structured JSON, effective effort, completed-result extraction, timeouts, and usage reporting on small samples before replacing workers.
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

### 1.1.E — Validate Samples and Close the Roadmap Item

1. Add focused offline regressions for configuration parsing, duplicate/missing/unknown stage entries, unsupported model/effort or image capability, all inference call branches, complete-result/schema enforcement, cache separation, auth/quota/timeout errors, and fingerprint/resume/manual-handoff behavior.
2. Verify both CLIs and the dispatcher. Use disposable diagnostics under `.agent/tmp/` and avoid running conversion in user source directories during tests.
3. Run representative PDF and HTML pilots at `gpt-6.1-sol` / `medium`, concurrency 1: native/scanned/mixed and legitimate blank pages, prose/code, simple and merged-cell tables, diagrams and multi-page continuations, plus a single HTML page and a crawl. Treat text-layer/reclip behaviors not yet implemented under later roadmap items as pending.
4. Persist requests, source references, outputs, schema/asset checks, visual comparisons, latency, call/cache counts, and available usage figures. Manually inspect source fidelity before bulk use; evaluate higher effort only when a concrete sample reveals a quality gap. Bound claims to tested cases.
5. Per implementation commit, run targeted conversion regressions, `python tools/harness/pre_flight.py --quick`, and `cargo test -p test_runner --test test_architecture_rules -- --quiet`; include a Section 10 diary entry and the stable task ID in the commit rationale.
6. Before declaring 1.1 complete, follow [milestone policy](../../.agents/rules/roadmap-maintenance.md) and the [roadmap completion procedure](../../.agents/skills/roadmap-maintenance/SKILL.md): milestone gate, required unit/integration checks, semantic parity review of modified conversion behavior, affected documentation/checkpoints, diary compaction, and roadmap pruning. Planning alone does not satisfy this gate.

Acceptance: both converters use Codex with ChatGPT authentication and validated per-stage model/effort settings; sample outputs and recovery behavior meet the consolidated conversion contract within the implemented scope. Remaining 1.2-1.6 work stays visible in the roadmap. Prune completed execution detail only after its implementation evidence exists.

## Official References

- [Codex Python SDK](https://learn.chatgpt.com/docs/codex-sdk#python-library): Python integration with a local app-server and pinned runtime dependency.
- [Codex app-server](https://learn.chatgpt.com/docs/app-server): `model/list` reports model capabilities and supported efforts; turn configuration accepts model, effort, and output schema.
- [Codex authentication](https://learn.chatgpt.com/docs/auth): ChatGPT sign-in uses subscription access; API-key use is separately billed.
- [GPT-6.1 Sol](https://developers.openai.com/api/docs/models/gpt-6.1-sol): model specifications and Responses API efforts. Validate the installed Codex runtime/account separately.

## Planning Validation

For this planning-only change, validate local links, YAML examples and audited inference-stage coverage, English/portable paths, and preservation of pending roadmap scope. Run the quick pre-flight and separate architecture suite, record observed results in the diary, and commit the plan and roadmap together. Do not install an SDK, run inference, migrate active YAML/code, convert manuals, or index documents as part of preparing the plan.

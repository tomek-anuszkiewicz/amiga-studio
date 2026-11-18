# Committed task inputs, environment inventory and raw execution capture

- Task ID: `WORKFLOW-EXECUTION-CAPTURE-20261117`.
- Status: Separate proposal, revised 2026-11-18 to use the standard OpenTelemetry Collector, try its YAML filter, and consider a small intermediary for dynamic registration. Implementation, staging, commits and integration remain pending.
- Relationship: This is a separate follow-up to [V1 closure entry](../../DIARY.md#2026-11-18-0950-cet--close-the-manual-ta-workflow-tasks). The inspected V1 candidate already has seven role definitions, 45 skill definitions, a versioned result contract and a next-step matrix. Use the accepted V1 revision and its owners when implementation begins.
- Sources: The user's requirements, the 2026-11-18 clarification and [Funkcja Cloud Code](https://chatgpt.com/share/6afcc2d4-f7ac-83ed-90a9-cc36952a8232), read through Playwright on 2026-11-17. Retain the execution-capture requirements and the addition about code-change history. The latest clarification sets the desired Git history in section 2. Code Mode, temporary commits after each stage and checkpoint-based recovery are outside this task.
- Delivery boundary: An approved implementation baseline in Git, raw local execution archives and an evidence-based completeness report. No database, statistical analysis pipeline, dashboard, automatic environment restoration or replay engine.
- Checkout: Not selected for implementation. Follow the [task checkout procedure](../../.agents/rules/git-merge-commits.md#choose-the-task-checkout) when this separate task starts; the earlier V1 migration's checkout choice does not silently select this task's location.

## 1. Outcome and scope

This task preserves the inputs to implementation and as much of its observable execution as the runtime exposes. The immediate outcome is a recoverable starting point and a checked collection of raw files. Later analysis may compare workflow versions, instructions or model choices on the same task; this revision does not implement those comparisons, except for the explicitly requested environment-version comparison. Planning remains interactive and is outside the required experiment boundary.

Start with Codex's persisted session JSONL. Compare it with the event stream from `codex exec --json` where useful. Preserve the CLI stream when it supplies required evidence or the selected execution already uses it. Run a bounded OpenTelemetry pilot and use its results to decide which signals, if any, to keep in routine capture. At roughly 10–20 executions per day, use a local file archive. Databases, dashboards, a custom live UI and an App Server integration are outside this task. Use the existing client to observe and steer work.

## 2. Initial commit after approval, before implementation

Once the implementation plan and test design are agreed, preserve the following inputs in one initial task commit:

- The actual approved requirements, implementation plan, test scenarios, acceptance criteria and relevant user decisions. A test design is required at this boundary; test code that has not yet been written is not a prerequisite.
- The referenced task attachments needed to understand the work, including screenshots, failure examples and supplied input files. A chat link, local pathname or checksum alone does not preserve their contents. Keep approved versions with the task, subject to existing user-data boundaries.
- The workflow, agent, skill, instruction and configuration versions that will govern execution. Repository-owned definitions are identified by Git. If a replacement workflow is itself being developed, record its candidate identity separately from the source revision governing the work. If an actually used definition comes from outside Git, preserve its available content/version in the private archive and mark anything unavailable; the intended normal setup keeps task workflow definitions in the repository.
- The environment inventory from section 3, including the settings intended for each role. Runtime records later establish which model and reasoning effort were actually used.

Keep active plans under `.agent/tasks/`. During an authorized initial-commit operation, explicitly add only the selected plan, test-design, attachment and environment files despite that directory's default ignore rule. Do not unignore all plans or add `.agent/tmp/` and raw telemetry to Git. Git preserves the approved starting version when the working plan changes later; preserve consequential plan revisions with the execution records as well.

Record the pre-commit code base and dirty/untracked state. The resulting `start_commit` must identify the actual implementation input: isolate unrelated changes through the approved checkout procedure, and do not sweep them into this commit. If necessary inputs cannot be represented by that commit, preserve them explicitly and report the baseline limitation; HEAD alone is insufficient. A missing approved baseline blocks the claim that a reproducible implementation start has been established.

Record the resulting commit SHA in the execution manifest after Git creates it; do not try to embed a commit's own SHA into its contents. An adopted full-task startup includes this initial commit in its stated scope. A narrow planning request, including this revision, still creates no commit. Use the existing preparation and mechanical-commit operations with the applicable checks, four review lanes and diary policy; this plan creates no implicit exemption from protected gates. The initial commit does not certify future implementation or replace final verification.

Preserve the approved-input commit as a separate, reachable Git record after publishing or retiring the task branch. The requested final Git history consolidates the implementation in a single commit where applicable. Prefer creating that single implementation commit during final preparation. If intermediate implementation commits already exist, document the proposed squash range and integration method for approval before any rewrite. Preserve `start_commit` and the exact verified candidate. Confirm separately if the target branch needs a linear history with no merge commit. The existing integration protocol requires original source tips in accepted history, so squashing existing source commits requires a concrete integration decision. Per-stage commits and checkpoint recovery are outside scope. The edit history by workflow stage lives in the execution archive.

## 3. Environment inventory and comparison

Use a small mechanical inventory of the tools and libraries available to the selected task, recording which executable/environment is actually selected. Include probe time, command, result and unavailable/error status; do not turn a failed probe into a guessed version. Keep the committed inventory portable and free of credentials and private absolute paths.

| Inventory area | Required record when installed or relevant to the task |
| --- | --- |
| Host and execution context | OS version, architecture, shell, native/WSL/container context and task-relevant hardware facts for interpreting later timing. |
| Rust and native tools | `rustc`/Cargo versions, active toolchain and targets, applicable linker/compiler/SDK and verification tool versions. Preserve `Cargo.lock` and identify resolved dependency inputs without upgrading them. |
| Python | Available task-relevant Python versions, the interpreter/virtual environment actually selected, package-manager version and installed package/version inventory. Preserve requirements/lockfiles; distinguish declared requirements from installed packages and unresolved pins. |
| Node.js | Available task-relevant Node versions, selected runtime/package manager, local dependency inventory and used global tools, with package manifests and lockfiles. |
| Docker and other runtimes | Docker client/server/Compose versions when present, relevant image tags and immutable digests, and backend availability. Include .NET SDKs or other tools when the task uses them. An absent or stopped Docker installation is an observation, not a reason to install/start it or containerize the task. |
| Agent environment | Actual client/surface and version, role/model/reasoning settings, relevant plugins/MCP tools and workflow configuration identities. Do not substitute a different CLI binary's version for the editor's running client. |

Record task-relevant configuration through a small allowlist of non-secret values; never copy authentication files, tokens, raw environment-variable dumps or `.env` into the commit. Preserve exact input/configuration contents where available and appropriate, rather than relying on hashes of files that will disappear.

For a later execution of the same task, capture a fresh inventory and compare it with the saved one. Report changed, missing and unknown tools/packages/configuration and retain both inventories. Differences warn about changed conditions; they do not automatically block execution or install, downgrade or restore tools. A genuinely missing tool needed to perform the task still has its ordinary operational consequence. Managing multiple compiler versions, rebuilding old environments and a replay runner remain outside this task.

## 4. Capture starts before the recorded work

Before the user's selected start boundary, identify the client and version, confirm that it writes persistent session rollouts, and record the effective settings. For a CLI comparison, link the stdout event stream from `codex exec --json` to the persisted rollout and check what it contributes. Leave session persistence enabled. An IDE trial requires its own evidence because its launch path differs from non-interactive CLI execution.

The installed CLI's help and the [non-interactive mode guide](https://learn.chatgpt.com/docs/non-interactive-mode#make-output-machine-readable) describe `--json` as a JSONL event stream. It can supply turn and item boundaries, command executions, file-change notifications and usage. The [CLI event schema](https://github.com/openai/codex/blob/main/codex-rs/exec/src/exec_events.rs) defines a file-change entry by path and change kind, with an overall patch status. Compare the stream with the local rollout for actual diff bodies. `--output-schema` shapes the final answer separately.

A read-only planning sample of 44 recent Amiga rollout files found 42 successful `patch_apply_end` events with `call_id` and `turn_id`. These contained 72 update entries with nonempty `unified_diff` and 18 additions with file contents. In the same sample, candidate shell-write calls generally had tool results without patch bodies. This establishes that applied patch content is available in the sampled rollout files. It leaves arbitrary shell writes, deletion, rename, binary content, and stage attribution to practical verification.

The [Codex OpenTelemetry documentation](https://github.com/openai/codex/blob/main/codex-rs/otel/README.md) describes separate log, trace and metric export, detected skill use and opt-in final-response logging. It also describes best-effort delivery and size limits. The [configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference) documents tool-result truncation and user-level routing. Live reception remains untested. Run the bounded comparison in section 6 and retain only the signals that fill a demonstrated need in routine use.

### Register a selected execution

The user explicitly selects when execution capture starts and ends and which resumed or replacement threads belong to it. Conversations about planning or other Amiga work can remain outside capture. Repository identity establishes project membership. The explicit registration establishes which execution and turns to preserve.

- On the user's start instruction, the registration skill calls a small Python helper from within the selected Codex thread. Read `CODEX_THREAD_ID` in the command environment and use native Git to check the selected working directory. Compare the canonical result of `git rev-parse --path-format=absolute --git-common-dir` with the configured Amiga Git directory. The main checkout and its linked worktrees share that identity regardless of their names. A separate clone has a different Git directory and needs explicit selection if it belongs in a trial.
- Record `execution_id`, task ID, root thread ID, verified Git identity, and the chosen start boundary. Keep `execution_id` distinct from Codex thread identity and per-invocation `run_id`. Record timestamps in UTC or with an explicit offset, plus a turn ID when available. In a local check, `CODEX_THREAD_ID` matched the persisted `session_meta.payload.id` of the current conversation, and the rollout contained a turn ID. Use the rollout's thread ID for correlation. The dedicated `session_id` is a separate field. The [rollout recorder](https://github.com/openai/codex/blob/main/codex-rs/rollout/src/recorder.rs) and [Codex event documentation](https://github.com/openai/codex/blob/main/codex-rs/otel/README.md#agent-response-logs) show where the correlation keys come from. Confirm the actual OTLP payload in the pilot.
- Persist the registration locally for every capture. For the fixed-filter OTLP pilot, use the selection to prepare the Collector filter and wait for readiness before covered work begins. If a dynamic filter is adopted, register the execution and root thread through its local interface before covered work begins. This leaves the start instruction and readiness check in a control turn if the user wants capture to start with the next turn. When the user ends capture, the same helper records the last included event or turn. Collection can still receive delayed records belonging to the selected range. Report a failed registration or stop and keep the requested range visible.

### Run the project receiver when needed

Use the upstream OpenTelemetry Collector with a project-owned YAML configuration for the local OTLP receiver and file exporter. The Amiga Git repository and its linked worktrees share the service and the archive in the surviving primary checkout. Locate the service by the canonical Git common directory. Bind intake and any local registration interface to loopback. Reuse the upstream Collector distribution and its storage components.

When capture uses OTLP, the registration skill calls the Python helper to reuse the running project service or start it on demand. Confirm readiness, project identity and the effective filter before starting the execution. If a fixed filter needs changing, schedule that change between executions. If launch fails or an unrelated process holds the configured endpoint, return the concrete failure. The client must already be configured to send to that endpoint. Opening a capture does not edit Codex's configuration.

The proposed first version keeps the receiver running between executions. Ending a capture closes its range and lets the receiver finish writing eligible events. Shut down the receiver explicitly after active executions are closed and output is flushed. The pilot should verify start, reuse and orderly shutdown. Revisit automatic exit only if repeated use shows a need.

### Filter with the Collector before adding an intermediary

Start with the Collector's [filter processor](https://github.com/open-telemetry/opentelemetry-collector-contrib/blob/main/processor/filterprocessor/README.md) and file exporter. A fixed YAML rule can drop log records whose `conversation.id` does not match the selected thread IDs, including records with a missing ID. Use the observed attribute location and exercise one allowed and one rejected ID. Check each additional signal independently. A log attribute does not establish the same attribute on spans or metric data points.

Use the fixed YAML for a dedicated trial with known IDs and a bounded intake. A rule for the root ID can include child records whose own metadata identifies that parent. A thread ID alone cannot enforce start and stop inside a reused conversation. The standard [file config provider](https://github.com/open-telemetry/opentelemetry-collector/blob/main/confmap/provider/fileprovider/provider.go) reads YAML without watching the file. A change to processor rules requires a [full configuration reload](https://github.com/open-telemetry/opentelemetry-collector/blob/main/docs/internal-architecture.md#configuration-reload) or a restart. Rebuilding the pipeline on every registration would interrupt active streams. If the trial shows that we need to admit new executions or descendants without interrupting intake, keep the Collector and add a small local Python intermediary for execution-specific rules.

For that variant, the registration skill sends start and stop updates to the intermediary's local registry. Codex can send OTLP/HTTP JSON to the intermediary. It filters individual records in each batch and forwards accepted records to the standard Collector for file output. Test this contract in the selected client and Collector version before adopting it. Keep Collector configuration, reception for file output and archival in the standard distribution. The project owns only admission logic and the small forwarding interface.

The fixed filter or the intermediary enforces execution selection before archival. If dynamic filtering is exercised, keep its registry across restarts and collection. It records root threads, selected continuations, known descendants and boundaries so delayed data is evaluated against the same selection.

- Match OTLP `conversation.id` against registered thread IDs. For an unknown thread, use available parent or root metadata and, when needed, the header or lineage in the local rollout JSONL to verify descent from a registered execution. Cache successful mappings. Record subsequent root continuations only when explicitly included. A planning thread can be registered later. Keep any temporary lookup cache separate from the user's decision to admit it.
- Apply both identity and the execution boundary before writing to the execution archive. Only store attributable records. Keep unregistered and unresolved data out of the archive. Report gaps if child events arrive before their lineage is available. If this causes material loss, assess source isolation or a bounded in-memory buffer before selecting a remedy.
- A shared IDE process can send unrelated events to the receiver. Discard them before archival. The scope of this pilot is the saved data. A dedicated launch is available for the test and is necessary if we later need metrics aggregated solely for Amiga and those metrics lack a usable project or thread attribute.

Map admitted task/work-item/run/round/skill identities to observed thread, turn and call IDs and, when available, trace and span IDs. Preserve known parent relationships and uncertainty. Use verified event boundaries for records that arrive late.

Prefer automatic runtime/tool emission. Supplement only demonstrated gaps with small mechanical records at existing invocation/tool boundaries: stage and round transitions, selected findings, wait/resume reasons, plan revisions, and terminal outcomes. Agent-authored explanations are useful but distinct from observed events. Use no hooks and do not build a lifecycle coordinator or intercept the editor's private App Server connection.

Preserve model and reasoning-effort changes per observable turn/invocation, including subagents, instead of applying the initial setting to the whole task. The desired per-role configuration is recorded separately from runtime evidence. Allow future explicit per-role choices without assuming a cheaper model is better or changing the current migration's model assignments automatically.

## 5. Archive contents and end boundary

Collect at the authorized end of work, including implementation, checks, reviews, selected repairs and any authorized commits/publication. Failed or interrupted attempts retain their records and actual outcome too. Capture status, software-task outcome and batch readiness are separate facts.

| Material | Preserve and correlate |
| --- | --- |
| Original Codex history | Original records for the selected main, child and continuation threads. Copy complete rollout files for threads dedicated to the execution. In a mixed thread, preserve the selected raw JSONL records and their source ranges so other turns stay outside the archive. Preserve available messages, inter-agent exchanges, tool calls and usage. Keep the original CLI stream where selected. |
| OpenTelemetry, if exercised | Preserve the originally received payloads that match the registered execution, together with capture settings, available receiver/export errors and any rotated segments. Record missing expected emissions, unresolved identities and unflushed tails. |
| Workflow artifacts | Exact text/JSON results, approved input references and revisions, scoped assignments, decisions, round/stop records and necessary supplementary events. |
| Execution evidence | Invoked commands and temporary script contents when relevant, full available test/verification/review reports and outputs, exit statuses, failures/retries and artifact references. Preserve separately produced reports when a console/log excerpt was truncated. |
| Code-change history | Available edit/patch events and diffs with time, agent, invocation, round and stage association; baseline/final Git identities and final candidate state. Check actual coverage for patch tools, shell/script writes, new files, deletions, renames and binary attachments used in the trial. |

Reconstruct what changed at each recorded workflow stage from Codex events first. Where the rollout provides an actual patch, correlate it with the stage using its turn and tool-call identity. Compare the reconstructed stage result with known edits and the stage's actual file state in the pilot. If required changes are missing, add only the smallest demonstrated supplement. A Git diff at the affected stage or invocation boundary can establish the net change and may miss an edit reverted inside that boundary. Preserve the observed granularity and unresolved attribution. A final diff alone leaves changes by stage unverified. Use no temporary stage commits, hooks or filesystem surveillance.

Use one private archive directory per execution in the surviving primary checkout at `.agent/task-archives/<task-id>/<execution-id>/`. Write accepted OpenTelemetry payloads under its `otel/` subdirectory. Add `.agent/task-archives/` to `.gitignore` before the first write. Keep approved task inputs, attachments and environment in the initial commit. Raw logs and runtime reports belong in the local archive. Archive files may contain source-native local paths and private session content. Exclude credential stores and unrelated sessions, and record redactions or unavailable data. The shared archive stays in place when a task worktree is removed.

The small archive manifest identifies the format/capture-tool versions, task and execution, `start_commit`, governing workflow revision, final commit/candidate/outcome, session lineage, time boundaries, relative file inventory, sizes/hashes, and missing/partial sources. Preserve actual bytes for files not recoverable from Git; a path or checksum is not a backup. Preserve the baseline commit's reachability when retiring a task branch; do not claim a deleted reference guarantees long-term recovery.

At collection, wait for known task workers and relevant tool outputs, flush a receiver if used and supported, and copy the selected records with a bounded stability check. Mark changing files and trailing partial records provisional and retry after their producer finishes. Use the user's recorded last event or turn. If it includes the current invocation's final response, finalize from a later invocation after that response completes.

Write a new archive/manifest revision on a later collection rather than silently overwriting earlier evidence. Do not delete source logs or rotate away the only retained copy. Reuse an existing local backup arrangement where available; report whether an independent backup exists and leave its destination/setup as an explicit deployment choice. No backup service, cloud upload or retention scheduler is part of this plan.

## 6. Prove data sufficiency on practical tasks

The first capture trial uses a small, observed task. Exercise every row below. Include an applied patch, a shell/script write and a change reverted before a stage ends. Compare persisted JSONL with CLI stdout where the trial uses it. Also start the local receiver and compare the Codex telemetry it actually receives with the native JSONL and task artifacts for the same registered execution. A real Codex turn must produce identifiable records in the archive.

Use the following table as the initial list of information the archive should answer. For each row, record which source supplies it, an actual example or source locator, and a status of `present`, `partial`, `missing`, `unavailable` or `not_exercised`. Mark what OTLP adds and whether that addition answers a useful question. Start by checking structured skill use, individual API attempts and their timing, and any useful MCP timings or errors. Preserve other records attributable to the registered trial so unexpected useful fields can be assessed. Add a requirement only when the comparison identifies a concrete question we need to answer. The goal is `present` for every required row. Record the user's decision for any material gap and preserve the report in the common text/JSON result format.

| Required information | What the check must establish |
| --- | --- |
| Starting inputs | Approved plan, test design and required attachments can be read back from `start_commit`; workflow/environment versions and any initial dirty-input limits are identified. |
| Execution registration and lineage | Explicit start and stop select the expected turns. Confirm the Git identity check accepts the main checkout and a linked worktree. Confirm thread-ID correlation, child attribution and runtime model/effort. Check that an unregistered Amiga conversation, a different repository and turns outside the registered range stay out of the archive. |
| Communication | Main/child intermediate and final responses, handoff payloads and tool outputs are present to the extent exposed; truncation, omitted attachments and missing end-of-turn records are identified. |
| Tools and skills | Commands, arguments/results, errors and used scripts can be located. Distinguish detected/declared skill use, a file read, and evidence that its procedure was followed. |
| Timing and progress | Start/end, stages, rounds, retries, wait periods and termination reasons can be traced. Preserve raw times; do not turn overlapping agent durations into total task time. |
| Usage and verification | Available input/output/cached/reasoning token fields retain their source scope and cumulative/per-call meaning. Tests, reviews and final outcomes have their actual reports and exit/status evidence; do not invent cost or unavailable attribution. |
| Change history | Known edits are visible at the claimed granularity, including the exercised write mechanisms and reversals. Missing intermediate diffs or ambiguous ownership stay visible. |
| File integrity and survival | Selected files are readable, inventoried and intact, all expected segments are accounted for, collection boundaries are explicit, and the archive remains accessible outside any removable worktree. |

Fill demonstrated capture gaps with the smallest in-scope addition and repeat only the affected practical segment. No automated workflow-test suite or permanent acceptance harness is introduced; lightweight inventory, parsing and integrity checks are part of the requested archive operation. An unobserved event, hidden model reasoning or inaccessible runtime context cannot be reconstructed from an agent's recollection. Keep such limits in the report.

Declare capture sufficient only for the checked requirements, client/version, operations and boundary. A material gap limits the completeness verdict. Before authorized cleanup, preserve the starting commit and any required evidence whose only copy would be removed by cleanup. A missing runtime event and a recoverable report still in the worktree have different consequences. Keep software verification independent from capture assessment. If the runtime cannot expose a required item, report the concrete gap and requested decision.

## 7. Atomic operations and workflow handoff

Use the workflow's existing role and operation boundaries. The supporting operations use the established `TA-` prefix and return text plus structured JSON. Reuse the accepted V1 result schema, validator and guidance matrix where applicable.

| Skill | Scope and stopping point |
| --- | --- |
| `TA-capture-environment` | Inspect the selected execution environment and record tool/runtime versions, installed dependencies, lockfile identities and effective workflow configuration under section 3. Return the inventory and unavailable probes; no installation, upgrades, comparison or commit. |
| `TA-compare-environments` | Compare a supplied baseline inventory with a fresh inventory and report meaningful differences and unknowns. Differences are recorded observations, not automatic repair or refusal to run. No environment restoration or new capture invocation. |
| `TA-register-execution` | Record the user's selection and have the Python helper validate Git and thread identity and record the starting boundary. If OTLP is selected, start or reuse the project Collector and check that capture is ready. If using dynamic filtering, wait for the intermediary's registration acknowledgment. The caller invokes the helper to record the user's end boundary. Collection remains a separate operation. |
| `TA-collect-task-records` | Preserve the selected execution's raw rollout and CLI event streams, any received telemetry and associated artifacts in its durable local archive, with a manifest and an explicit capture boundary. Return the archive or a partial-collection result. Collection leaves completeness assessment, deletion and commits to their own operations. |
| `TA-check-task-records` | Inspect a supplied archive against the observed task and section 6's required data, checking integrity, correlation and missing content. Return a concrete completeness/gap report. No additional collection, instrumentation repair, statistical analysis or automatic rerun. |

The user or main session selects the next operation. A clear full-task request may authorize the sequence. Receiver startup is part of registration when OTLP is selected. A gap report does not itself authorize a repair. Use the applicable workflow round limits and report incomplete results accurately.

The intended software-task path is to agree the detailed plan and test design, confirm available capture sources, and register the execution at the boundary chosen by the user. Record the environment and create the approved-input commit. Then implement, verify and review through the existing workflow. Preserve the consolidated implementation commit and any authorized publication. At the user's chosen end, close capture, collect and check the selected records, then perform any separately authorized cleanup. Preserve failed or interrupted attempts at their actual boundary.

Reuse existing preparation, verification and mechanical-commit operations for the starting commit. Before removing a worktree or branch, keep the starting commit reachable and preserve the required evidence that exists only there. Collection is a separate operation before cleanup. The report records capture gaps independently of the Git and software outcomes.

## 8. Policy and ownership changes required by this task

Apply the following changes only during this separately authorized implementation, using the current active instruction owners. If V1 has already moved or renamed those owners, follow the accepted definitions rather than restoring obsolete paths.

- Commit and plan-lifecycle policy must permit the explicitly selected approved plan, test design, attachments and environment to be tracked despite the default task-directory ignore rule, and define the initial commit's authority and timing for adopted executions. Ordinary plan editing does not authorize that commit.
- Define how this task keeps the approved-input commit reachable and preserves its sole-copy evidence before eligible cleanup. Keep collection outside the cleanup operation. A project-wide cleanup gate remains a separate policy decision.
- Add the shared archive directory to `.gitignore` before capture writes data. Keep the durable private archive distinct from disposable diagnostics under `.agent/tmp/`. Preserve task provenance with portable committed references.
- Update affected skills, caller instructions and reviewer criteria together. Preserve independent review, protected checks/references, source safety, exact candidate identity and current verification requirements. A complete archive never replaces a required passing gate.

Relevant current owners are [commit policy](../../.agents/rules/git-commits.md), [plan lifecycle](../../.agents/rules/structural-root-cause.md#execution-plan-lifecycle), [integration and cleanup](../../.agents/rules/git-merge-commits.md), [path privacy](../../.agents/rules/no-external-paths.md) and [instruction ownership](../../.agents/rules/information-hierarchy.md). These links route future implementation; this plan changes none of those active files.

## 9. Implementation sequence and practical verification

All items remain pending. Select the task checkout and actual delivery scope before implementation. Keep environment/capture setup separate from unrelated running sessions and preserve other actors' work.

1. **Verify the available Codex sources.** Use the chosen editor/client version. Check session lineage, turn attribution, the persisted rollout schema and `codex exec --json` for applicable invocations. Compare file-change bodies and other required data against known actions.
2. **Define the small file contracts.** Specify the environment inventory, execution/session mapping, archive manifest and completeness report. Reuse existing text/JSON results and source formats.
3. **Establish registration and the starting-input operation.** Implement the skill and small Python helper, check Git/thread identity and save the user's boundaries. Add native record preservation, baseline inventory and comparison, and the approved-input commit procedure. Bootstrap with direct commands where the skills do not yet exist.
4. **Implement collection and completeness inspection.** Preserve original selected files, associated artifacts and available stage diffs. Keep collection and assessment separate.
5. **Connect the lifecycle at its accepted owners.** Add the core operations and reconcile section 8's policy changes. Preserve the initial plan commit and one consolidated implementation commit in the intended Git history.
6. **Exercise a small real task.** Observe the initial commit, environment capture/comparison, runtime and subagent records, reports and known code edits. Exercise each required row in section 6.
7. **Start and test the standard OpenTelemetry Collector.** Start the Collector through the registration helper with a fixed YAML filter. Run the intended Codex client and verify that matching records reach the execution's `otel/` directory while unrelated records stay out. Compare actual records with JSONL and section 6. If runs or new descendants need admission without a restart, exercise the intermediary on a mixed batch and confirm it forwards only matching records. Check start, reuse, shutdown and archive readability for the selected setup. For dynamic filtering, also exercise start and stop boundaries, descendants, delayed records and ending one execution while another stays active.
8. **Deliver the implemented scope and evidence.** Report the starting and final commit identities, archive, client/configuration, exercised operations and completeness findings. Perform applicable repository checks, reviews and authorized Git operations at their normal triggers.

Use a dedicated Codex conversation for the initial live probe so the archived test input is known. Test exclusion of unrelated data with a known unregistered thread or fixture. If the intended setup uses a shared IDE, verify that client's actual export and the join by conversation ID before relying on it. Use archive slicing for mixed conversations.

For lifecycle trials involving commits, publication or cleanup, use a disposable independent local Git repository with its own target branch. A linked worktree shares the real repository's refs and does not isolate publication. Do not publish a trial to real master or remove a real task's worktree.

Practical inspection is required; no automated workflow-test suite, permanent acceptance matrix, benchmark campaign or replay runner is introduced. Small inventory, parsing and integrity checks belong to the requested capture operation. Existing software and independent-tool verification remains protected. Do not claim a missing or unrun check passed.

## 10. Completion and deferred work

Completion requires a real task demonstrating the durable starting commit, consolidated implementation history, environment inventory and comparison, explicit execution selection, raw execution archive and a checked outcome for every required row in section 6. The OpenTelemetry trial must show standard Collector startup, real delivery, scoped file output and shutdown. Use that evidence to decide whether dynamic filtering and routine telemetry are needed. Report unsupported data, truncation, missing attribution and the last included event. A material gap remains open until addressed or explicitly accepted by the user.

The records support later comparisons of workflow versions, instructions, model and reasoning settings. They do not guarantee identical LLM behavior, availability of an older hosted model or exact environment restoration. Per-role model choices remain explicit; adding capture does not authorize changing existing model assignments.

Deferred work includes databases, derived statistics and cost analysis, dashboards, App Server integration, automated environment restoration, model-selection optimization and replay. Temporary stage commits and checkpoint recovery remain outside this plan.

This revision updates the execution-capture proposal. Implementation and concrete Git operations require their own authorized scope.


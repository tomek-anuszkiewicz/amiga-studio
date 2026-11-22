# Amiga 500 Emulator: Agent Instructions

## Shared Contract

Work within the requested outcome and necessary technical changes. Inspection
requests authorize investigation; implementation requests authorize scoped edits.
Propose optional behavior before adding it. Preserve unrelated files, staged
versions, user data and concurrent work. This is a private, pre-release project:
prefer the simplest correct current implementation and complete internal migrations.

Read affected requirements before design or editing; nearby code supplies examples,
not authority over specifications or hardware evidence. Applicable result standards
remain binding even when their detailed reviewer checklist is not loaded by the
author. In-scope corrections complete the assignment; findings grant no unrelated
write authority.

Ordinary changes stay unstaged; batch preparation authorizes only selected staging, and commits,
publication, cleanup and push require their corresponding authorization.
When reporting or committing incomplete work, state the known blockers and
distinguish a working commit from verified readiness.
Architectural checkers/policy data, weakened gates and protected references retain
their [explicit approval boundaries](.agents/references/protected-verification.md#protected-verification).

Keep active, paused and blocked plans in `.agent/tasks/` and use `.agent/tmp/`
for disposable diagnostics. Preserve stable task IDs across copies and
continuations. On resumption, verify approved input versions and record
the continuation boundary.
Do not trust old artifacts without checking their inputs. Run commands explicitly;
do not install, configure or use lifecycle, code or Git hooks. Prefer existing tools
and direct commands; create a helper only for a concrete need they cannot meet.

Keep committed paths portable: repository-relative paths, environment variables or
CLI inputs. Never hardcode private host paths or disclose them unnecessarily.
Already authorized local inputs need no second approval; missing configuration is
an input gap, not permission to invent it.

### Specification authority

Approved Design specifications govern behavior, timing, memory semantics and
architecture. When a requirement, expected result or reference model conflicts
with a specification, stop the conflicting change and present the requirement,
contrary evidence and concrete options. Obtain a recorded user decision before
changing or waiving an approved requirement or adding behavior outside scope.
A review finding or code agreement does not grant that authority.

### Language

Answer in the user's language. Write repository artifacts in English, including
saved plans and agent result files, regardless of the conversation language.
Translate user concepts into idiomatic English when writing repository artifacts.
State outcomes and actual evidence plainly, distinguishing assumptions,
hypotheses and unrun work.

### Explain the system to the engineer

Answer the question directly in the user's language. Assume a technically fluent
colleague who commissioned the system and may picture its internals differently.
Give enough project context in the first answer to follow your reasoning. For a
diagnosis, review finding or architecture question, follow one ordinary user
action through the relevant components. Explain each component's job, who calls
whom, when, and where state is read or written when it matters. If expectations
differ, identify the precise step, the expected and actual behavior, and what
the user would observe. Say what the evidence establishes, what you infer, and
which conditions a probe introduced. Identify proposed behavior as future work.
Put identifiers and links after the explanation they support. Keep simple
answers short. If the user asks again, walk through a concrete sequence that
resolves the missing connection.

Before starting or resuming mouse/keyboard control, announce it. Stop input when
the user takes over. On finishing, including recoverable errors or interruption,
perform the tool's documented completion/release step before unrelated work. Then
explicitly tell the user in their language that control ended and they can work
normally. Do not claim release unless confirmed.

### Choose the task checkout

Before new implementation, identify the current checkout and branch with Git.
Resume a saved plan in its recorded checkout and branch unless the user selects
another location. For other work, reuse an explicit location or establish whether
to use the current checkout or a new one. Continue with read-only discovery while
that choice is pending. Report missing local assets without inventing inputs.

## User-Selected Operations

The user selects the outcome and scope through ordinary language or a named skill.
The main session matches that request and the active task context to the relevant
repository skill, reads it and coordinates its authorized steps. For example,
implementing an existing task selects `TA-WF-Main-implement-task`; correcting
identified errors in an active task selects the repair mode of
`TA-WF-Step-implement-change`; creating or amending a plan selects
`TA-WF-Main-plan-change`. A request to explain or assess something remains an
inspection request. Use skill descriptions for other operations; these examples
are not an exact-phrase command parser. Resolve missing scope before dependent work.

Selecting a skill does not extend authorization. Recommendations and findings do
not start another operation, and completing a workflow does not start the next
stage. Commits, publication and cleanup still need their corresponding scope.

When the user steers active work beyond its approved scope, behavior or acceptance
criteria, identify the changed requirement and keep any active plan consistent
with the user's decision.

Repository roles live in `.codex/agents/`; runtime roles and installed plugin skills
are separate. Use fresh contexts for independent reviews. Do not claim filesystem,
context or budget enforcement beyond the runtime controls actually observed.

### Subagent assignment and model selection

The main session coordinates `TA-WF-Main-` workflows on its current model. Loading
a skill does not switch that session's model or reasoning effort. Delegate planning
and test design to `TA-planner`, source/test implementation and repairs to
`TA-implementer`, independent assessments to `TA-code-reviewer`, hardware source
research to `TA-hardware-researcher` and selected command/coverage execution to
`TA-test-runner`. Other skills keep their stated executor. A compound operation
may include several assignments; resolve settings for each assignment separately.

Choose model and reasoning effort independently, in this order: the user's explicit
setting for that assignment, the contextual rule below, a default stated by the
selected skill, the role defaults in this table, then the project `[agents]`
defaults in `.codex/config.toml`. An explicit user setting can change either field
without discarding the rule for the other field.

| Role | Default model | Default reasoning effort |
| --- | --- | --- |
| `TA-planner` | `gpt-6.2-astra` | `xhigh` |
| `TA-implementer` | `gpt-6.2-sol` | `high` |
| `TA-code-reviewer` | `gpt-6.2-sol` | `high` |
| `TA-hardware-researcher` | `gpt-6.2-sol` | `high` |
| `TA-test-runner` | `gpt-6.2-sol` | `medium` |

For implementation, repair or optimization that changes Amiga chipset or peripheral
hardware behavior, use `gpt-6.2-astra` with `xhigh`. Infer this from the requested
behavior, plan, affected interactions and authorized writes: examples include
custom-chip timing, DMA, bus arbitration, register semantics and interrupt/device
behavior. Paths and hardware names are evidence, not sufficient triggers. Merely
reading chipset code or changing its prose, logs or build tooling does not trigger
this rule. Reviewers retain their own defaults even when reviewing chipset work.

Before every launch, including corrections and resumed work, announce the role,
model, effort and a short reason. Supply the selected skill, checkout, requirements,
current plan/findings, write scope and result boundary in the assignment. Use fresh
context (`fork_turns = "none"` where supported) and pass `model` and
`reasoning_effort` explicitly to the spawn tool. A planner applies the included
test-design step in its own assignment; it does not launch another planner.

When selecting requirements for an assignment, look for applicable entries in the
[review decisions register](.agents/references/review-decisions.md). Supply only
relevant entries and their source pointers to the assignee.

Re-evaluate settings when the assigned scope changes. If a new assignment needs
different settings, launch a fresh agent with the necessary current context;
a follow-up message does not itself change an existing agent's model or effort.
Keep `model` and `model_reasoning_effort` absent from role TOML files so their
native precedence cannot defeat assignment-specific selection.

If the role or selected settings are unavailable, report the actual launch gap
instead of silently substituting another model or doing delegated work in the
main session. Distinguish requested settings from effective settings observed in
runtime records; an agent's claim about its own model is not confirmation. Changed
role/skill configuration may require a new session before it is discoverable.

## Task-Specific Rules and Navigation

Load only the relevant technical requirements. For a requested review, load its
assigned criteria; authors do not read all reviewer checklists.
Read-only questions need only the material needed to answer them.

| Work | Read at its trigger |
| --- | --- |
| Crates, exports or CPU module layout | [Workspace layout](Obsidian/Amiga/Design/General%20Architecture.md#workspace-layout) |
| Chip, DMA, bus, interrupts or peripherals | [Hardware topology](Obsidian/Amiga/Design/General%20Architecture.md#11-physical-bus-topology--inter-chip-isolation-invariant) |
| Reference conversion | [Nested bootstrap instructions](tools/bootstrap/AGENTS.md), then the applicable program README |

When using a graph for navigation, identify the project or reference corpus and
verify relevant source definitions. Use `rg` when it provides the needed context
or the graph is missing, stale or insufficient. Distinguish hardware manuals,
measured observations and reference-model behavior. Manuals live under
`Obsidian/Amiga/Reference/`; imported models and vectors under `ref_src/`.

For planning, test design, implementation and review, consult the shared
[knowledge tools guide](.agents/references/knowledge-tools.md) when graph
navigation, RAG retrieval, hardware research or a GUI screenshot helps the
assigned work. It distinguishes read-only discovery from indexing and refresh.

## Verification and Completion

Report actual checks, their inputs and outcomes. Distinguish executed checks,
retained evidence and assessments. Failed, missing or invalidated required evidence
cannot establish readiness; an assessment does not replace an executed check.

Preparation, reviews, commits, publication and cleanup require the user's selected
scope; routing does not grant it. Before reporting completion,
review the diff against the requested scope and state checks and remaining limits.

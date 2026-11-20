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
write authority. Handle specification conflicts immediately under
[reference integrity and requirements](.agents/rules/spec-compliance.md).

Ordinary changes stay unstaged; batch preparation authorizes only selected staging, and commits,
publication, cleanup and push require their corresponding authorization.
When a direct commit request follows a decision to decline required repairs,
lead the first user-visible message with the working-commit limit and the
named blockers from the known checkpoint, including when that message is
commentary before inspection. Use the
[checkpoint warning](.agents/references/workflow-guidance.md#working-commit-after-declining-repairs)
before any tool call for the commit.
Architectural checkers/policy data, weakened gates and protected references retain
their [explicit approval boundaries](.agents/rules/unit-testing-policy.md#protected-verification).

Keep active, paused and blocked plans in `.agent/tasks/` and use `.agent/tmp/`
for disposable diagnostics. Preserve stable task IDs across copies and
continuations. On resumption, verify approved input versions and record
the continuation boundary.
Before planning a new task or starting execution of an existing plan, satisfy
the [telemetry startup prerequisite](.agents/references/workflow-contract.md#telemetry-startup-prerequisite).
Do not trust old artifacts without checking their inputs. Run commands explicitly;
do not install, configure or use lifecycle, code or Git hooks. Prefer existing tools
and direct commands; create a helper only for a concrete need they cannot meet.

Keep committed paths portable: repository-relative paths, environment variables or
CLI inputs. Never hardcode private host paths or disclose them unnecessarily.
Already authorized local inputs need no second approval; missing configuration is
an input gap, not permission to invent it.

Answer in the user's language. Write repository artifacts in English, including
all saved plans and agent result files. The `.agent/tasks/` exclusion applies
only to automated language and vocabulary checks. Follow the shared
[language policy](.agents/rules/language-policy.md) for writing, findings and
audits. State outcomes and actual evidence plainly, distinguishing assumptions,
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
Starting a saved plan selects its own worktree and branch under
[TA-start-plan](.agents/skills/TA-start-plan/SKILL.md). Resume the recorded working
plan in that checkout. For other work, reuse an explicit location or establish
whether to use the current checkout or a new one. Continue with planning and
read-only discovery while that choice is pending. Use
[TA-repair-worktree](.agents/skills/TA-repair-worktree/SKILL.md) for selected
missing local assets. The [local integration procedure](.agents/rules/git-merge-commits.md#choose-the-task-checkout)
describes placement and the bounded detached diagnostic exception.

## One Selected Operation

Select the role and atomic `TA-` skill from the actual request. Before dispatch,
apply the [runtime settings](.agents/references/workflow-runtime.md#model-and-reasoning-effort)
and announce the assignment under the [invocation and dual-result contract](.agents/references/workflow-contract.md).
Each invocation returns matching text and JSON with exact input identity,
subject to the contract's ordinary whole-file preservation exception. Use
one to four declared rounds (three by default). A missing prerequisite, finding or
limit returns control; it does not start another skill, repair or lifecycle phase.
The receiving main session automatically explains the actual result and applies
[next-step guidance](.agents/references/workflow-guidance.md). A request to save
agreed behavior selects [planning and applicable test design](.agents/references/workflow-guidance.md#save-a-formal-plan).
An expressly scoped [end-to-end delivery](.agents/references/workflow-guidance.md#start-or-deliver-a-saved-plan)
selects the operations within its recorded delegation. Otherwise wait for the
user's choice, recheck applicability and run only that operation. The skill
matrix governs official suggestions, and the user may directly select another
skill. Each executable choice names its role, skill, scope and copyable invocation.

When the user steers active work beyond its approved scope, behavior or acceptance
criteria, [record the proposed amendment immediately](.agents/references/workflow-guidance.md#amend-an-active-plan).
Track the proposal through restart, approval or decline even if the user gives no
separate plan-edit instruction.

Repository roles live in `.codex/agents/`; runtime roles and installed plugin skills
are separate. Use fresh contexts for independent reviews. Do not claim filesystem,
context or budget enforcement beyond the runtime controls actually observed.

For a new repository skill, use [TA-create-skill](.agents/skills/TA-create-skill/SKILL.md).
For a new repository role, use [TA-create-agent](.agents/skills/TA-create-agent/SKILL.md).
Select each operation separately when the request needs both definitions.

## Task-Specific Rules and Navigation

Load only the relevant advance requirements and selected operation. Reviewers also
load their assigned lane criteria; authors do not read all reviewer checklists.
Read-only questions need only the material needed to answer them.

| Work | Read at its trigger |
| --- | --- |
| Rust implementation | [Rust constraints](.agents/rules/rust-best-practices.md); affected specifications through [General Architecture](Obsidian/Amiga/Design/General%20Architecture.md) |
| Execution-path refactoring or performance work | [Execution semantics](.agents/rules/performance-and-readability.md) |
| Crates, exports or CPU module layout | [Workspace layout](Obsidian/Amiga/Design/General%20Architecture.md#workspace-layout) |
| Internal APIs, configuration, generated formats or snapshots | [Internal migrations](.agents/rules/internal-migrations.md) |
| Chip, DMA, bus, interrupts or peripherals | [Hardware topology](Obsidian/Amiga/Design/General%20Architecture.md#11-physical-bus-topology--inter-chip-isolation-invariant) |
| Test design, execution or assessment | [Testing policy](.agents/rules/unit-testing-policy.md) and the selected skill's criteria |
| Defect investigation or repair | [Original reproduction and causal repair](.agents/references/defect-resolution.md) before mutation |
| GUI | [GUI advance requirements](.agents/rules/egui-best-practices.md) |
| Documentation or specifications | [Documentation value](.agents/references/documentation-criteria.md#documentation-value) |
| Rules, skills, roles or instruction ownership | [Information hierarchy](.agents/rules/information-hierarchy.md) |
| Preparation, commits or closure | [Batch readiness](.agents/rules/git-commits.md) and [roadmap or milestone completion](Obsidian/Amiga/Design/Verification/Testing%20Strategy%20and%20Quality%20Assurance.md#roadmap-and-milestone-completion) at their triggers |
| Synchronization, publication or cleanup | The relevant part of [integration policy](.agents/rules/git-merge-commits.md) and the separately selected skill |
| PowerShell changes | [TA-write-powershell](.agents/skills/TA-write-powershell/SKILL.md) |
| Reference conversion | [Nested bootstrap instructions](tools/bootstrap/AGENTS.md), then the applicable program README |

For cross-file code navigation use the selected Graphify read operation:
[query](.agents/skills/TA-query-graph/SKILL.md),
[path](.agents/skills/TA-find-graph-path/SKILL.md) or
[symbol explanation](.agents/skills/TA-explain-graph-symbol/SKILL.md).
Choose project or reference data explicitly, read source definitions, and use `rg`
when the graph is missing, stale or insufficient. Hardware/manual retrieval follows
[domain provenance](.agents/skills/TA-research-hardware-manuals/references/provenance.md). Manuals live under
`Obsidian/Amiga/Reference/`; imported models and vectors under `ref_src/`.

## Verification and Completion

Run relevant focused checks during development. Canonical batch verification is
`python tools/verification/verify.py commit`; milestone verification is
`python tools/verification/verify.py milestone` at an actual closure trigger.
The [verifier](tools/verification/README.md) owns selection, required gates and
evidence reuse. A failed or missing required gate blocks readiness. Assessment
results do not replace executed verification.

Required batch review uses four fresh independent `TA-code-reviewer` instances,
one for each fixed lane. Schedule within available slots; an author or earlier
audit cannot replace a lane. Preparation, checks, reviews, commits, publication and
cleanup remain separately selected operations; broad existing authorization may
cover their scope without repeated permission requests, while guidance still waits
for selection. Before reporting completion,
review the diff against the requested scope and state checks and remaining limits.
